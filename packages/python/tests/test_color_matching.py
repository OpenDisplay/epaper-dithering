"""Tests for LAB color matching and perceptual color science."""

import numpy as np
import pytest
from epaper_dithering import ColorScheme, DitherMode, dither_image
from epaper_dithering.color_space import srgb_to_linear
from epaper_dithering.color_space_lab import rgb_to_lab
from epaper_dithering.tone_map import auto_compress_dynamic_range, compress_dynamic_range
from PIL import Image


class TestLABConversion:
    """Test RGB to LAB conversion accuracy."""

    def test_white_converts_to_l100(self):
        """Pure white in linear RGB should produce L*=100, a=0, b=0."""
        white = np.array([1.0, 1.0, 1.0])
        lab = rgb_to_lab(white)
        assert lab[0] == pytest.approx(100.0, abs=0.1)
        assert lab[1] == pytest.approx(0.0, abs=0.5)
        assert lab[2] == pytest.approx(0.0, abs=0.5)

    def test_black_converts_to_l0(self):
        """Pure black should produce L*=0, a=0, b=0."""
        black = np.array([0.0, 0.0, 0.0])
        lab = rgb_to_lab(black)
        assert lab[0] == pytest.approx(0.0, abs=0.1)
        assert lab[1] == pytest.approx(0.0, abs=0.5)
        assert lab[2] == pytest.approx(0.0, abs=0.5)

    def test_midgray_lightness(self):
        """50% linear gray should produce L* around 76 (perceptual midpoint)."""
        gray = np.array([0.5, 0.5, 0.5])
        lab = rgb_to_lab(gray)
        assert 70 < lab[0] < 80, f"50% linear gray L* should be ~76, got {lab[0]:.1f}"

    def test_red_has_positive_a(self):
        """Pure red should have positive a* (red-green axis)."""
        red = np.array([1.0, 0.0, 0.0])
        lab = rgb_to_lab(red)
        assert lab[1] > 50, f"Red should have high positive a*, got {lab[1]:.1f}"

    def test_batch_matches_single(self):
        """Batch conversion should match individual conversions."""
        colors = np.array([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
        batch = rgb_to_lab(colors)
        for i in range(3):
            single = rgb_to_lab(colors[i])
            np.testing.assert_allclose(batch[i], single, atol=1e-10)


class TestColorMatchingAccuracy:
    """Test LCH-weighted color matching on measured palettes."""

    def test_bright_green_matches_green_not_yellow(self):
        """Bright green should match palette green, not yellow.

        With the SPECTRA measured palette, green is very dark (L~31) while
        yellow is bright (L~75). The LCH-weighted distance fixes this by
        de-emphasizing lightness and emphasizing hue.
        """
        from epaper_dithering import SPECTRA_7_3_6COLOR

        green_img = Image.new("RGB", (10, 10), (100, 255, 40))
        result = dither_image(green_img, SPECTRA_7_3_6COLOR, DitherMode.NONE)

        pixels = list(result.get_flattened_data())
        green_idx = 5  # SPECTRA order: black=0, white=1, yellow=2, red=3, blue=4, green=5
        assert all(p == green_idx for p in pixels), (
            f"Bright green should map to palette green (idx 5), got indices: {set(pixels)}"
        )

    def test_pure_blue_matches_blue_not_black(self):
        """Blue should match palette blue, not black.

        The SPECTRA measured black has a slight blue tint (26,13,35).
        """
        from epaper_dithering import SPECTRA_7_3_6COLOR

        blue_img = Image.new("RGB", (10, 10), (0, 0, 255))
        result = dither_image(blue_img, SPECTRA_7_3_6COLOR, DitherMode.NONE)

        pixels = list(result.get_flattened_data())
        blue_idx = 4
        assert all(p == blue_idx for p in pixels), (
            f"Pure blue should map to palette blue (idx 4), got indices: {set(pixels)}"
        )

    def test_measured_vs_pure_produces_different_output(self):
        """Measured colors should produce different dithering than pure RGB."""
        from epaper_dithering import ColorPalette

        gradient = Image.new("RGB", (50, 50), (128, 128, 128))

        pure = ColorScheme.BWR
        measured = ColorPalette(
            colors={"black": (5, 5, 5), "white": (180, 180, 170), "red": (115, 12, 2)}, accent="red"
        )

        result_pure = dither_image(gradient, pure, DitherMode.FLOYD_STEINBERG)
        result_measured = dither_image(gradient, measured, DitherMode.FLOYD_STEINBERG)

        assert not np.array_equal(np.array(result_pure), np.array(result_measured))


class TestCompressDynamicRange:
    """Unit tests for dynamic range compression (tone mapping)."""

    def _make_palette_linear(self, black_srgb, white_srgb):
        """Helper: convert black/white sRGB tuples to linear palette array."""
        palette_srgb = np.array([black_srgb, white_srgb], dtype=np.float32)
        return srgb_to_linear(palette_srgb)

    def test_output_luminance_within_display_range(self):
        """Compressed pixel luminance should fall within [black_Y, white_Y]."""
        # Simulate a display with black=(30,30,30) and white=(200,200,200)
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        black_Y = float(
            0.2126729 * palette_linear[0, 0] + 0.7151522 * palette_linear[0, 1] + 0.0721750 * palette_linear[0, 2]
        )
        white_Y = float(
            0.2126729 * palette_linear[1, 0] + 0.7151522 * palette_linear[1, 1] + 0.0721750 * palette_linear[1, 2]
        )

        # Create a gradient from black to white in linear space
        pixels = np.linspace(0.0, 1.0, 100).reshape(10, 10, 1).repeat(3, axis=2).astype(np.float32)
        result = compress_dynamic_range(pixels, palette_linear, strength=1.0)

        # Compute luminance of result
        result_Y = 0.2126729 * result[:, :, 0] + 0.7151522 * result[:, :, 1] + 0.0721750 * result[:, :, 2]

        assert result_Y.min() >= black_Y - 1e-5, (
            f"Min luminance {result_Y.min():.4f} should be >= black_Y {black_Y:.4f}"
        )
        assert result_Y.max() <= white_Y + 1e-5, (
            f"Max luminance {result_Y.max():.4f} should be <= white_Y {white_Y:.4f}"
        )

    def test_strength_zero_is_identity(self):
        """strength=0.0 should return pixels unchanged."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        pixels = np.random.default_rng(42).random((5, 5, 3)).astype(np.float32)

        result = compress_dynamic_range(pixels, palette_linear, strength=0.0)
        np.testing.assert_array_equal(result, pixels)

    def test_strength_half_is_intermediate(self):
        """strength=0.5 should produce values between original and fully compressed."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        pixels = np.full((5, 5, 3), 0.8, dtype=np.float32)

        full = compress_dynamic_range(pixels.copy(), palette_linear, strength=1.0)
        half = compress_dynamic_range(pixels.copy(), palette_linear, strength=0.5)

        # Half-strength should be between original and full
        assert np.all(half <= pixels + 1e-6)
        assert np.all(half >= full - 1e-6)

    def test_pure_black_white_palette_is_near_identity(self):
        """With black=(0,0,0) and white=(255,255,255), compression is near-identity."""
        palette_linear = self._make_palette_linear([0, 0, 0], [255, 255, 255])
        pixels = np.random.default_rng(42).random((5, 5, 3)).astype(np.float32)

        result = compress_dynamic_range(pixels.copy(), palette_linear, strength=1.0)
        # black_Y ≈ 0.0, white_Y ≈ 1.0, so compressed ≈ 0 + Y * 1.0 = Y
        np.testing.assert_allclose(result, pixels, atol=1e-5)

    def test_near_black_pixels_get_display_black(self):
        """Pixels with near-zero luminance should be set to display black level."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        pixels = np.zeros((3, 3, 3), dtype=np.float32)  # All black

        result = compress_dynamic_range(pixels, palette_linear, strength=1.0)

        black_Y = float(
            0.2126729 * palette_linear[0, 0] + 0.7151522 * palette_linear[0, 1] + 0.0721750 * palette_linear[0, 2]
        )
        # Near-black pixels should be set to approximately display black luminance
        assert result.mean() == pytest.approx(black_Y, abs=0.01)


class TestAutoCompressDynamicRange:
    """Unit tests for auto (percentile-based) dynamic range compression."""

    def _make_palette_linear(self, black_srgb, white_srgb):
        """Helper: convert black/white sRGB tuples to linear palette array."""
        palette_srgb = np.array([black_srgb, white_srgb], dtype=np.float32)
        return srgb_to_linear(palette_srgb)

    def _luminance(self, pixels):
        """Compute per-pixel luminance."""
        return 0.2126729 * pixels[:, :, 0] + 0.7151522 * pixels[:, :, 1] + 0.0721750 * pixels[:, :, 2]

    def test_full_range_gradient_matches_linear(self):
        """Full-range gradient should produce similar result to linear compression."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        pixels = np.linspace(0.0, 1.0, 100).reshape(10, 10, 1).repeat(3, axis=2).astype(np.float32)

        auto_result = auto_compress_dynamic_range(pixels.copy(), palette_linear)
        linear_result = compress_dynamic_range(pixels.copy(), palette_linear, 1.0)

        # For a full-range gradient, auto should be close to linear 1.0
        np.testing.assert_allclose(auto_result, linear_result, atol=0.05)

    def test_narrow_range_has_more_contrast(self):
        """Narrow-range image should have more contrast with auto than fixed 1.0."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        # Image using only 30-70% of luminance range
        pixels = np.linspace(0.3, 0.7, 100).reshape(10, 10, 1).repeat(3, axis=2).astype(np.float32)

        auto_result = auto_compress_dynamic_range(pixels.copy(), palette_linear)
        linear_result = compress_dynamic_range(pixels.copy(), palette_linear, 1.0)

        # Auto should stretch to use more of the display range
        auto_Y = self._luminance(auto_result)
        linear_Y = self._luminance(linear_result)
        auto_range = float(auto_Y.max() - auto_Y.min())
        linear_range = float(linear_Y.max() - linear_Y.min())

        assert auto_range > linear_range, f"Auto range {auto_range:.4f} should exceed linear range {linear_range:.4f}"

    def test_uniform_image_falls_back(self):
        """Uniform image should fall back to linear compression."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        pixels = np.full((5, 5, 3), 0.5, dtype=np.float32)

        result = auto_compress_dynamic_range(pixels, palette_linear)
        expected = compress_dynamic_range(pixels.copy(), palette_linear, 1.0)
        np.testing.assert_allclose(result, expected, atol=1e-5)

    def test_output_luminance_within_display_range(self):
        """Auto-compressed output luminance should stay within display range (with tolerance)."""
        palette_linear = self._make_palette_linear([30, 30, 30], [200, 200, 200])
        black_Y = float(
            0.2126729 * palette_linear[0, 0] + 0.7151522 * palette_linear[0, 1] + 0.0721750 * palette_linear[0, 2]
        )
        white_Y = float(
            0.2126729 * palette_linear[1, 0] + 0.7151522 * palette_linear[1, 1] + 0.0721750 * palette_linear[1, 2]
        )

        pixels = np.linspace(0.0, 1.0, 100).reshape(10, 10, 1).repeat(3, axis=2).astype(np.float32)
        result = auto_compress_dynamic_range(pixels, palette_linear)
        result_Y = self._luminance(result)

        # p2/p98 percentile-based: the 2% outliers at each end may exceed the range slightly
        p2 = float(np.percentile(result_Y, 2))
        p98 = float(np.percentile(result_Y, 98))
        assert p2 >= black_Y - 0.01, f"p2 luminance {p2:.4f} should be near black_Y {black_Y:.4f}"
        assert p98 <= white_Y + 0.01, f"p98 luminance {p98:.4f} should be near white_Y {white_Y:.4f}"

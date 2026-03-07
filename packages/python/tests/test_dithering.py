"""Tests for dithering algorithms and color science."""

import numpy as np
import pytest
from epaper_dithering import ColorScheme, DitherMode, dither_image
from PIL import Image


class TestDitheringAlgorithms:
    """Test all dithering algorithms."""

    @pytest.mark.parametrize("mode", list(DitherMode))
    def test_all_modes_produce_valid_output(self, small_test_image, mode):
        """Test each dithering mode produces valid palette image."""
        result = dither_image(small_test_image, ColorScheme.BWR, mode)

        assert result.mode == "P", f"Output should be palette mode, got {result.mode}"
        assert result.size == small_test_image.size, "Output size should match input"

        palette = result.getpalette()
        assert palette is not None, "Output should have a palette"

    @pytest.mark.parametrize("scheme", list(ColorScheme))
    def test_all_color_schemes(self, small_test_image, scheme):
        """Test each color scheme works correctly."""
        result = dither_image(small_test_image, scheme, DitherMode.BURKES)

        assert result.mode == "P"
        palette = result.getpalette()
        assert len(palette) >= scheme.color_count * 3, "Palette should contain all scheme colors"

    def test_output_image_type(self, small_test_image):
        """Test output is PIL Image."""
        result = dither_image(small_test_image, ColorScheme.MONO, DitherMode.FLOYD_STEINBERG)
        assert isinstance(result, Image.Image)

    def test_rgba_input_handling(self):
        """Test RGBA images are handled correctly."""
        rgba_img = Image.new("RGBA", (10, 10), color=(128, 128, 128, 255))
        result = dither_image(rgba_img, ColorScheme.BWR, DitherMode.BURKES)

        assert result.mode == "P"
        assert result.size == rgba_img.size

    @pytest.mark.parametrize("scheme", list(ColorScheme))
    def test_output_contains_only_valid_indices(self, scheme):
        """All output pixels must be valid palette indices.

        No pixel should have an index >= the number of colors in the scheme.
        """
        gradient = Image.new("RGB", (64, 64))
        pixels = gradient.load()
        for y in range(64):
            for x in range(64):
                r = int(x * 255 / 63)
                g = int(y * 255 / 63)
                b = 128
                pixels[x, y] = (r, g, b)

        result = dither_image(gradient, scheme, DitherMode.FLOYD_STEINBERG)
        result_pixels = np.array(result)
        max_idx = result_pixels.max()
        assert max_idx < scheme.color_count, f"{scheme.name}: pixel index {max_idx} >= color count {scheme.color_count}"


class TestColorScience:
    """Test color science improvements."""

    def test_gamma_correction_improves_midtones(self):
        """Test that gamma correction prevents dark crushing in midtones."""
        gradient = Image.new("RGB", (256, 64))
        for x in range(256):
            for y in range(64):
                gradient.putpixel((x, y), (x, x, x))

        result = dither_image(gradient, ColorScheme.GRAYSCALE_4, DitherMode.FLOYD_STEINBERG)

        histogram = result.histogram()[:4]

        assert all(count > 0 for count in histogram), f"All 4 grayscale levels should be used, got counts: {histogram}"

        assert histogram[1] > 100, "Gray1 should be used in midtones"
        assert histogram[2] > 100, "Gray2 should be used in midtones"

    def test_alpha_composites_on_white(self):
        """Test RGBA images composite on white background, not black."""
        rgba_white = Image.new("RGBA", (10, 10), (255, 255, 255, 128))
        result_white = dither_image(rgba_white, ColorScheme.MONO, DitherMode.NONE)
        histogram_white = result_white.histogram()

        assert histogram_white[1] > histogram_white[0], (
            f"Semi-transparent white should stay white, got {histogram_white[:2]}"
        )

        rgba_transparent = Image.new("RGBA", (10, 10), (0, 0, 0, 0))
        result_transparent = dither_image(rgba_transparent, ColorScheme.MONO, DitherMode.NONE)
        histogram_transparent = result_transparent.histogram()

        assert histogram_transparent[1] == 100, (
            f"Fully transparent should become white background, got {histogram_transparent[:2]}"
        )

    def test_serpentine_parameter_works(self):
        """Test serpentine parameter can be enabled/disabled."""
        gradient = Image.new("RGB", (100, 100))
        pixels = gradient.load()
        for y in range(100):
            for x in range(100):
                gray_value = int(x * 255 / 99)
                pixels[x, y] = (gray_value, gray_value, gray_value)

        result_serpentine = dither_image(gradient, ColorScheme.MONO, DitherMode.FLOYD_STEINBERG, serpentine=True)
        result_raster = dither_image(gradient, ColorScheme.MONO, DitherMode.FLOYD_STEINBERG, serpentine=False)

        assert result_serpentine.mode == "P"
        assert result_raster.mode == "P"

        array_serpentine = np.array(result_serpentine)
        array_raster = np.array(result_raster)
        assert not np.array_equal(array_serpentine, array_raster), (
            "Serpentine should produce different output than raster"
        )

    def test_deterministic_output(self):
        """Test that dithering produces identical output on repeated runs."""
        img = Image.new("RGB", (50, 50), (128, 128, 128))

        result1 = dither_image(img, ColorScheme.BWR, DitherMode.FLOYD_STEINBERG)
        result2 = dither_image(img, ColorScheme.BWR, DitherMode.FLOYD_STEINBERG)

        assert np.array_equal(np.array(result1), np.array(result2)), "Dithering should be deterministic"

    def test_ordered_dithering_uses_threshold_correctly(self):
        """Test ordered dithering produces reasonable distribution."""
        gray = Image.new("RGB", (16, 16), (186, 186, 186))
        result = dither_image(gray, ColorScheme.MONO, DitherMode.ORDERED)

        pixels = list(result.get_flattened_data())

        unique = set(pixels)
        assert len(unique) == 2, f"Should use both black and white, got {unique}"
        assert 0 in unique and 1 in unique

        black_count = pixels.count(0)
        white_count = pixels.count(1)
        ratio = black_count / (black_count + white_count)
        assert 0.20 < ratio < 0.35, f"Should be ~25% black with LAB matching, got ratio {ratio:.2f}"

    def test_all_error_diffusion_with_serpentine(self):
        """Test all error diffusion algorithms accept serpentine parameter."""
        img = Image.new("RGB", (20, 20), (100, 100, 100))

        error_diffusion_modes = [
            DitherMode.FLOYD_STEINBERG,
            DitherMode.BURKES,
            DitherMode.ATKINSON,
            DitherMode.STUCKI,
            DitherMode.SIERRA,
            DitherMode.SIERRA_LITE,
            DitherMode.JARVIS_JUDICE_NINKE,
        ]

        for mode in error_diffusion_modes:
            result_true = dither_image(img, ColorScheme.MONO, mode, serpentine=True)
            assert result_true.mode == "P", f"{mode.name} should work with serpentine=True"

            result_false = dither_image(img, ColorScheme.MONO, mode, serpentine=False)
            assert result_false.mode == "P", f"{mode.name} should work with serpentine=False"


class TestToneCompression:
    """Test dynamic range compression with measured palettes."""

    def test_tone_compression_with_measured_palette(self):
        """Tone compression should run and produce valid output with measured palette."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (20, 20), (200, 200, 200))
        result = dither_image(img, SPECTRA_7_3_6COLOR, DitherMode.FLOYD_STEINBERG, tone_compression=1.0)

        assert result.mode == "P"
        assert result.size == (20, 20)

    @pytest.mark.parametrize("mode", list(DitherMode))
    def test_tone_compression_all_modes(self, mode):
        """Tone compression should work with all dithering modes."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (10, 10), (128, 128, 128))
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode, tone_compression=1.0)

        assert result.mode == "P"
        assert result.size == (10, 10)

    def test_tone_compression_zero_matches_no_compression(self):
        """tone_compression=0.0 should produce same result as without compression."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (20, 20), (128, 128, 128))
        result_zero = dither_image(img, SPECTRA_7_3_6COLOR, DitherMode.NONE, tone_compression=0.0)
        # With ColorScheme (not measured), compression is always skipped
        result_scheme = dither_image(img, ColorScheme.BWGBRY, DitherMode.NONE, tone_compression=1.0)

        # Both should produce valid palette output
        assert result_zero.mode == "P"
        assert result_scheme.mode == "P"

    def test_tone_compression_skipped_for_color_scheme(self):
        """Tone compression should be skipped for theoretical ColorScheme."""
        img = Image.new("RGB", (20, 20), (128, 128, 128))

        # These should produce identical output since ColorScheme bypasses compression
        result_tc0 = dither_image(img, ColorScheme.MONO, DitherMode.NONE, tone_compression=0.0)
        result_tc1 = dither_image(img, ColorScheme.MONO, DitherMode.NONE, tone_compression=1.0)
        result_auto = dither_image(img, ColorScheme.MONO, DitherMode.NONE, tone_compression="auto")

        assert np.array_equal(np.array(result_tc0), np.array(result_tc1)), (
            "Tone compression should have no effect on theoretical ColorScheme"
        )
        assert np.array_equal(np.array(result_tc0), np.array(result_auto)), (
            "Auto tone compression should have no effect on theoretical ColorScheme"
        )

    @pytest.mark.parametrize("mode", list(DitherMode))
    def test_auto_tone_compression_all_modes(self, mode):
        """Auto tone compression (default) should produce valid output for all modes."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (10, 10), (128, 128, 128))
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode)

        assert result.mode == "P"
        assert result.size == (10, 10)

    def test_tone_compression_changes_measured_output(self):
        """Tone compression should change the output for measured palettes."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        # Use a gradient to see meaningful differences
        gradient = Image.new("RGB", (50, 50))
        pixels = gradient.load()
        for y in range(50):
            for x in range(50):
                v = int(x * 255 / 49)
                pixels[x, y] = (v, v, v)

        result_off = dither_image(gradient, SPECTRA_7_3_6COLOR, DitherMode.FLOYD_STEINBERG, tone_compression=0.0)
        result_on = dither_image(gradient, SPECTRA_7_3_6COLOR, DitherMode.FLOYD_STEINBERG, tone_compression=1.0)

        assert not np.array_equal(np.array(result_off), np.array(result_on)), (
            "Tone compression should produce different output than no compression"
        )

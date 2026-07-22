"""Tests for dithering algorithms and color science."""

import threading
import time

import epaper_dithering._rs as _rs
import numpy as np
import pytest
from epaper_dithering import ColorScheme, DitherMode, dither_image
from epaper_dithering.core import _to_rgb_bytes
from PIL import Image


class TestDitheringAlgorithms:
    """Test all dithering algorithms."""

    @pytest.mark.parametrize("mode", list(DitherMode))
    def test_all_modes_produce_valid_output(self, small_test_image, mode):
        """Test each dithering mode produces valid palette image."""
        result = dither_image(small_test_image, ColorScheme.BWR, mode=mode)

        assert result.mode == "P", f"Output should be palette mode, got {result.mode}"
        assert result.size == small_test_image.size, "Output size should match input"

        palette = result.getpalette()
        assert palette is not None, "Output should have a palette"

    @pytest.mark.parametrize("scheme", list(ColorScheme))
    def test_all_color_schemes(self, small_test_image, scheme):
        """Test each color scheme works correctly."""
        result = dither_image(small_test_image, scheme, mode=DitherMode.BURKES)

        assert result.mode == "P"
        palette = result.getpalette()
        assert len(palette) >= scheme.color_count * 3, "Palette should contain all scheme colors"

    def test_output_image_type(self, small_test_image):
        """Test output is PIL Image."""
        result = dither_image(small_test_image, ColorScheme.MONO, mode=DitherMode.FLOYD_STEINBERG)
        assert isinstance(result, Image.Image)

    def test_rgba_input_handling(self):
        """Test RGBA images are handled correctly."""
        rgba_img = Image.new("RGBA", (10, 10), color=(128, 128, 128, 255))
        result = dither_image(rgba_img, ColorScheme.BWR, mode=DitherMode.BURKES)

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

        result = dither_image(gradient, scheme, mode=DitherMode.FLOYD_STEINBERG)
        result_pixels = np.array(result)
        max_idx = result_pixels.max()
        assert max_idx < scheme.color_count, f"{scheme.name}: pixel index {max_idx} >= color count {scheme.color_count}"


class TestBufferValidation:
    """Test the low-level `_rs.dither_image` validates buffer dimensions."""

    def test_mismatched_dimensions_raise(self):
        """A pixel buffer whose length != width * height * 3 must raise, not truncate."""
        import epaper_dithering._rs as _rs

        # 10x10 RGB image = 300 bytes; claim a height that does not match.
        pixels = bytes(10 * 10 * 3)
        with pytest.raises(ValueError):
            _rs.dither_image(pixels, 10, 11, scheme_id=ColorScheme.MONO.value)

    def test_matching_dimensions_succeed(self):
        """The correct width/height is accepted."""
        import epaper_dithering._rs as _rs

        pixels = bytes(10 * 10 * 3)
        indices = _rs.dither_image(pixels, 10, 10, scheme_id=ColorScheme.MONO.value)
        assert len(indices) == 10 * 10

    def test_bytearray_pixels_still_extract(self):
        """`pixels` (and `palette_bytes`) must still accept `bytearray`, not just `bytes`.

        `dither_image` takes `PyBackedBytes` now (needed to release the GIL during the
        dither), and `PyBackedBytes::extract` supports both `PyBytes` and `PyByteArray`.
        """
        import epaper_dithering._rs as _rs

        pixels = bytearray(10 * 10 * 3)
        indices = _rs.dither_image(pixels, 10, 10, scheme_id=ColorScheme.MONO.value)
        assert len(indices) == 10 * 10

        palette_bytes = bytearray(b"\x00\x00\x00\xff\xff\xff")
        indices = _rs.dither_image(pixels, 10, 10, palette_bytes=palette_bytes, accent_idx=0)
        assert len(indices) == 10 * 10


class TestColorScience:
    """Test color science improvements."""

    def test_gamma_correction_improves_midtones(self):
        """Test that gamma correction prevents dark crushing in midtones."""
        gradient = Image.new("RGB", (256, 64))
        for x in range(256):
            for y in range(64):
                gradient.putpixel((x, y), (x, x, x))

        result = dither_image(gradient, ColorScheme.GRAYSCALE_4, mode=DitherMode.FLOYD_STEINBERG)

        histogram = result.histogram()[:4]

        assert all(count > 0 for count in histogram), f"All 4 grayscale levels should be used, got counts: {histogram}"

        assert histogram[1] > 100, "Gray1 should be used in midtones"
        assert histogram[2] > 100, "Gray2 should be used in midtones"

    def test_alpha_composites_on_white(self):
        """Test RGBA images composite on white background, not black."""
        rgba_white = Image.new("RGBA", (10, 10), (255, 255, 255, 128))
        result_white = dither_image(rgba_white, ColorScheme.MONO, mode=DitherMode.NONE)
        histogram_white = result_white.histogram()

        assert histogram_white[1] > histogram_white[0], (
            f"Semi-transparent white should stay white, got {histogram_white[:2]}"
        )

        rgba_transparent = Image.new("RGBA", (10, 10), (0, 0, 0, 0))
        result_transparent = dither_image(rgba_transparent, ColorScheme.MONO, mode=DitherMode.NONE)
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

        result_serpentine = dither_image(gradient, ColorScheme.MONO, mode=DitherMode.FLOYD_STEINBERG, serpentine=True)
        result_raster = dither_image(gradient, ColorScheme.MONO, mode=DitherMode.FLOYD_STEINBERG, serpentine=False)

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

        result1 = dither_image(img, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG)
        result2 = dither_image(img, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG)

        assert np.array_equal(np.array(result1), np.array(result2)), "Dithering should be deterministic"

    def test_ordered_dithering_uses_threshold_correctly(self):
        """Test ordered dithering produces reasonable distribution."""
        gray = Image.new("RGB", (16, 16), (186, 186, 186))
        result = dither_image(gray, ColorScheme.MONO, mode=DitherMode.ORDERED)

        pixels = list(result.get_flattened_data())

        unique = set(pixels)
        assert len(unique) == 2, f"Should use both black and white, got {unique}"
        assert 0 in unique and 1 in unique

        black_count = pixels.count(0)
        white_count = pixels.count(1)
        ratio = black_count / (black_count + white_count)
        assert 0.10 < ratio < 0.45, f"Should use mostly white with some black, got ratio {ratio:.2f}"

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
            result_true = dither_image(img, ColorScheme.MONO, mode=mode, serpentine=True)
            assert result_true.mode == "P", f"{mode.name} should work with serpentine=True"

            result_false = dither_image(img, ColorScheme.MONO, mode=mode, serpentine=False)
            assert result_false.mode == "P", f"{mode.name} should work with serpentine=False"


class TestToneCompression:
    """Test dynamic range compression with measured palettes."""

    def test_tone_compression_with_measured_palette(self):
        """Tone compression should run and produce valid output with measured palette."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (20, 20), (200, 200, 200))
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG, tone=1.0)

        assert result.mode == "P"
        assert result.size == (20, 20)

    @pytest.mark.parametrize("mode", list(DitherMode))
    def test_tone_compression_all_modes(self, mode):
        """Tone compression should work with all dithering modes."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (10, 10), (128, 128, 128))
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode=mode, tone=1.0)

        assert result.mode == "P"
        assert result.size == (10, 10)

    def test_tone_compression_zero_matches_no_compression(self):
        """tone=0.0 should produce same result as without compression."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (20, 20), (128, 128, 128))
        result_zero = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.NONE, tone=0.0)
        # With ColorScheme (not measured), compression is always skipped
        result_scheme = dither_image(img, ColorScheme.BWGBRY, mode=DitherMode.NONE, tone=1.0)

        # Both should produce valid palette output
        assert result_zero.mode == "P"
        assert result_scheme.mode == "P"

    def test_tone_compression_skipped_for_color_scheme(self):
        """Tone compression should be skipped for theoretical ColorScheme."""
        img = Image.new("RGB", (20, 20), (128, 128, 128))

        # These should produce identical output since ColorScheme bypasses compression
        result_tc0 = dither_image(img, ColorScheme.MONO, mode=DitherMode.NONE, tone=0.0)
        result_tc1 = dither_image(img, ColorScheme.MONO, mode=DitherMode.NONE, tone=1.0)
        result_auto = dither_image(img, ColorScheme.MONO, mode=DitherMode.NONE, tone="auto")

        assert np.array_equal(np.array(result_tc0), np.array(result_tc1)), (
            "Tone compression should have no effect on theoretical ColorScheme"
        )
        assert np.array_equal(np.array(result_tc0), np.array(result_auto)), (
            "Auto tone compression should have no effect on theoretical ColorScheme"
        )

    @pytest.mark.parametrize("mode", list(DitherMode))
    def test_auto_tone_compression_all_modes(self, mode):
        """Auto tone compression should produce valid output for all modes."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (10, 10), (128, 128, 128))
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode=mode, tone="auto", gamut="auto")

        assert result.mode == "P"
        assert result.size == (10, 10)

    def test_default_tone_gamut_match_off_aliases(self):
        """tone/gamut default to off, and 'off' is the string alias for 0.0."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (16, 16), (128, 128, 128))
        result_default = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES)
        result_zero = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES, tone=0.0, gamut=0.0)
        result_off = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES, tone="off", gamut="off")

        assert np.array_equal(np.array(result_default), np.array(result_zero))
        assert np.array_equal(np.array(result_default), np.array(result_off))

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

        result_off = dither_image(gradient, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG, tone=0.0)
        result_on = dither_image(gradient, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG, tone=1.0)

        assert not np.array_equal(np.array(result_off), np.array(result_on)), (
            "Tone compression should produce different output than no compression"
        )


class TestToneAutoRegression:
    """Regression: auto tone mapping must not NaN-collapse bilevel images.

    A crisp bilevel frame (pure-white background, a few percent pure-black
    pixels, no antialiasing — exactly what a UI renderer produces) drove the
    log-skew strength epsilon-negative; (-x).powf(1.4) is NaN, which poisoned
    every pixel and made the nearest-palette search return index 0 for the
    whole frame: an all-black display instead of white with text.
    """

    def test_bilevel_white_image_survives_auto_tone(self):
        from epaper_dithering import ColorPalette

        # Measured-style palette whose paper white is well below sRGB white, so
        # the auto tone compression actually engages for a pure-white image.
        palette = ColorPalette(
            colors={"black": (30, 30, 30), "white": (200, 200, 200)},
            accent="black",
            scheme=ColorScheme.MONO,
        )
        img = Image.new("RGB", (100, 100), (255, 255, 255))
        # 3% pure black — enough that the 2nd-percentile luminance is 0.0.
        for x in range(100):
            for y in range(3):
                img.putpixel((x, y), (0, 0, 0))

        result = dither_image(img, palette, mode=DitherMode.NONE, tone="auto")

        white_pixels = list(result.getdata()).count(1)
        assert white_pixels > 9000, (
            f"expected a mostly-white result, got {white_pixels} white pixels — auto tone collapsed the frame"
        )


class TestGilRelease:
    """`dither_image` must not hold the GIL for the whole rayon-parallel dither.

    Audit H2: `dither_image`/`tone_compress`/`gamut_compress` used to run their
    rayon-parallel work with the GIL held. Because the GIL is process-global,
    calling this from an executor thread (as Home Assistant's `drawcustom`
    pipeline does) stalled the whole event loop for the duration of the dither.

    This test calls `epaper_dithering._rs.dither_image` directly instead of the
    public `dither_image` wrapper. The wrapper does substantial PIL work after
    the Rust call returns (`Image.new`, `out.putdata()` over hundreds of
    thousands of pixels, `out.putpalette()`), and PIL releases the GIL for all
    of that. That GIL-releasing tail dominates wall time regardless of whether
    the Rust call itself held the GIL, which completely masks the regression
    this test exists to catch (measured: with the GIL fix reverted, the
    PIL-wrapped call still collected as many samples as the fixed build). Only
    the direct `_rs` call isolates the FFI boundary that was actually changed.
    Do not "simplify" this back to the public `dither_image` API — that would
    silently re-hollow the test.
    """

    def test_counter_thread_keeps_advancing_during_dither(self):
        """A plain Python counter thread must keep making progress *throughout* a
        dither running on another thread — proof the GIL is released for the
        compute-heavy part, not just released somewhere before/after the call.

        A naive "before vs. after" comparison is not sufficient: CPython can
        switch threads on the bytecode boundary right before/after the FFI
        call even when the call itself holds the GIL for its whole duration,
        which would make a before/after check pass despite a real stall. So
        this samples the counter at fixed intervals *while the dither runs on
        a separate thread*.

        The discriminating signal is the *sample count* collected by the
        sampling loop, not any per-interval "stall" heuristic: when the GIL is
        held for the whole FFI call, the sampling thread cannot run at all
        during that call, so the loop collects on the order of 1 sample no
        matter how long the call takes. When the GIL is released, the loop
        collects roughly one sample per sleep interval.

        This must be a *single* FFI call, not a loop of many small calls:
        looping introduces yield points between calls (CPython force-switches
        threads at the next bytecode boundary once the switch interval has
        elapsed, even if each call itself holds the GIL throughout), which
        lets the held-GIL case rack up one sample per iteration and defeats
        the discrimination. A single long call has no such in-between yield
        points.

        Measured on the reference machine, calling `_rs.dither_image` directly
        with a single call on a 3200x1920 image (tone="auto", gamut="auto"),
        3 trials each:
          - GIL held (fix reverted):    1-2 samples,  ~0.68s wall
          - GIL released (fix applied): 34-37 samples, ~0.70s wall
        We assert >= 15 samples, which sits with a wide margin above the held
        case and well below the released case, so it should not flake on a
        loaded CI runner.
        """
        from epaper_dithering import SPECTRA_7_3_6COLOR, _rs

        palette_colors = list(SPECTRA_7_3_6COLOR.colors.values())
        palette_bytes = bytes(c for rgb in palette_colors for c in rgb)
        accent_idx = list(SPECTRA_7_3_6COLOR.colors.keys()).index(SPECTRA_7_3_6COLOR.accent)
        scheme_id = SPECTRA_7_3_6COLOR.scheme.value if SPECTRA_7_3_6COLOR.scheme is not None else None

        # 3200x1920 (4x linear scale of the display's 800x480) makes a single
        # call run long enough (~0.7s locally) to collect a comfortable margin
        # of samples in the released-GIL case. See measured numbers above.
        width, height = 3200, 1920
        img = Image.new("RGB", (width, height), color=(128, 64, 200))
        pixels = img.tobytes()

        done = threading.Event()

        def run_dither():
            _rs.dither_image(
                pixels,
                width,
                height,
                scheme_id=scheme_id,
                palette_bytes=palette_bytes,
                accent_idx=accent_idx,
                mode_id=int(DitherMode.BURKES),
                serpentine=True,
                exposure=1.0,
                saturation=1.0,
                shadows=0.0,
                highlights=0.0,
                tone=None,  # "auto"
                gamut=None,  # "auto"
            )
            done.set()

        counter = {"value": 0}

        def spin():
            while not done.is_set():
                counter["value"] += 1

        worker = threading.Thread(target=run_dither)
        spinner = threading.Thread(target=spin)
        spinner.start()
        worker.start()

        samples = []
        while not done.is_set():
            samples.append(counter["value"])
            time.sleep(0.01)

        worker.join()
        spinner.join()

        # Threshold with wide margin: held-GIL case measured ~1-3 samples,
        # released-GIL case measured ~30-45 samples on the reference machine.
        assert len(samples) >= 15, (
            f"only collected {len(samples)} samples while dither_image ran on another "
            "thread — the GIL was likely held for the whole FFI call, starving the "
            "sampling thread (this is the regression this test exists to catch)"
        )


class TestCompositeRgba:
    """Cross-language contract for RGBA -> RGB compositing.

    The same fixed input and the same literal expected bytes are asserted in
    ``packages/javascript/tests/dithering.test.ts`` and in
    ``packages/rust/core/src/composite.rs::cross_language_reference_vector``.
    The three copies are written independently -- none is generated from
    another -- so a divergence in any binding fails here.
    """

    # 12 pixels: mid-gray at alpha 0/1/127/128/254/255, a saturated color at the
    # same rounding-sensitive alphas, plus two arbitrary mixed pixels.
    # fmt: off
    RGBA = bytes([
        128, 128, 128, 0,
        128, 128, 128, 1,
        128, 128, 128, 127,
        128, 128, 128, 128,
        128, 128, 128, 254,
        128, 128, 128, 255,
        0, 64, 200, 1,
        0, 64, 200, 127,
        0, 64, 200, 128,
        0, 64, 200, 254,
        17, 200, 3, 63,
        250, 5, 130, 191,
    ])

    EXPECTED_RGB = bytes([
        255, 255, 255,
        255, 255, 255,
        192, 192, 192,
        191, 191, 191,
        128, 128, 128,
        128, 128, 128,
        254, 254, 255,
        128, 160, 228,
        127, 159, 227,
        1, 65, 200,
        196, 241, 193,
        251, 68, 161,
    ])
    # fmt: on

    def test_produces_the_exact_expected_rgb_bytes(self):
        assert _rs.composite_rgba(self.RGBA) == self.EXPECTED_RGB

    def test_to_rgb_bytes_uses_the_shared_implementation(self):
        """The public RGBA path in core.py must yield the same bytes."""
        image = Image.frombytes("RGBA", (len(self.RGBA) // 4, 1), self.RGBA)
        pixels, width, height = _to_rgb_bytes(image)
        assert (width, height) == (len(self.RGBA) // 4, 1)
        assert pixels == self.EXPECTED_RGB

    def test_opaque_pixels_are_bit_identical(self):
        opaque = bytes([12, 34, 56, 255, 200, 199, 198, 255])
        assert _rs.composite_rgba(opaque) == bytes([12, 34, 56, 200, 199, 198])

    @pytest.mark.parametrize("length", [1, 2, 3, 5, 7])
    def test_length_not_multiple_of_four_raises(self, length):
        with pytest.raises(ValueError, match="multiple of 4"):
            _rs.composite_rgba(bytes(length))

    def test_empty_buffer_is_accepted(self):
        assert _rs.composite_rgba(b"") == b""

    def test_matches_legacy_pil_paste_over_the_whole_channel_alpha_space(self):
        """Opaque *and* semi-transparent output must match the pre-refactor PIL path.

        Exhaustive over all 256 channel values x 256 alpha values, which is the
        complete input space for a single channel (channels are independent).
        """
        px = [(c, c, c, a) for a in range(256) for c in range(256)]
        image = Image.new("RGBA", (len(px), 1))
        image.putdata(px)

        legacy_bg = Image.new("RGB", image.size, (255, 255, 255))
        legacy_bg.paste(image, mask=image.split()[3])

        assert _rs.composite_rgba(image.tobytes()) == legacy_bg.tobytes()


class TestDizzy:
    """Dizzy dithering (DitherMode 9) must match the Rust reference byte for byte."""

    def test_cross_language_reference_vector(self):
        img = Image.new("RGB", (4, 4))
        img.putdata([(x * 60 + y * 5,) * 3 for y in range(4) for x in range(4)])
        out = dither_image(img, ColorScheme.BWR, mode=DitherMode.DIZZY)
        # Frozen literals, identical to the Rust and JavaScript suites.
        assert list(out.getdata()) == [0, 0, 1, 0, 0, 2, 2, 1, 0, 1, 1, 1, 0, 0, 2, 1]

    def test_is_deterministic(self):
        img = Image.new("RGB", (16, 16), (128, 128, 128))
        a = dither_image(img, ColorScheme.BWR, mode=DitherMode.DIZZY)
        b = dither_image(img, ColorScheme.BWR, mode=DitherMode.DIZZY)
        assert list(a.getdata()) == list(b.getdata())

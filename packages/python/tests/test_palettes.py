"""Tests for palette definitions and firmware conventions."""

import pytest
from epaper_dithering import ColorScheme, DitherMode, dither_image
from PIL import Image


class TestColorSchemes:
    """Test color scheme definitions."""

    def test_scheme_has_correct_palette(self):
        """Test color schemes have expected color counts."""
        assert ColorScheme.MONO.color_count == 2
        assert ColorScheme.BWR.color_count == 3
        assert ColorScheme.BWY.color_count == 3
        assert ColorScheme.BWRY.color_count == 4
        assert ColorScheme.BWGBRY.color_count == 6
        assert ColorScheme.GRAYSCALE_4.color_count == 4
        assert ColorScheme.GRAYSCALE_16.color_count == 16
        assert ColorScheme.SEVEN_COLOR.color_count == 7
        assert ColorScheme.BWGBRY_SPLIT.color_count == 6

    def test_palette_colors_valid(self):
        """Test all palette colors are valid RGB tuples."""
        for scheme in ColorScheme:
            for color in scheme.palette.colors.values():
                assert len(color) == 3, "RGB tuple should have 3 values"
                assert all(0 <= c <= 255 for c in color), "RGB values should be 0-255"

    def test_accent_color_defined(self):
        """Test each scheme has an accent color."""
        for scheme in ColorScheme:
            assert scheme.accent_color in scheme.palette.colors, (
                f"Accent color '{scheme.accent_color}' should be in palette"
            )

    def test_palette_color_order_matches_firmware(self):
        """Test palette color ordering matches bb_epaper firmware conventions.

        The e-paper display controller (bb_epaper) defines:
            BBEP_BLACK=0, BBEP_WHITE=1, BBEP_YELLOW=2, BBEP_RED=3
        All schemes must match this hardware order. Getting this wrong
        causes colors to display swapped on real hardware.
        """
        bwry_keys = list(ColorScheme.BWRY.palette.colors.keys())
        assert bwry_keys == ["black", "white", "yellow", "red"], (
            f"BWRY order must be black,white,yellow,red (firmware convention), got {bwry_keys}"
        )

        bwgbry_keys = list(ColorScheme.BWGBRY.palette.colors.keys())
        assert bwgbry_keys == ["black", "white", "yellow", "red", "blue", "green"], (
            f"BWGBRY order must be black,white,yellow,red,blue,green, got {bwgbry_keys}"
        )

        seven_color_keys = list(ColorScheme.SEVEN_COLOR.palette.colors.keys())
        assert seven_color_keys == ["black", "white", "yellow", "red", "blue", "green", "orange"], (
            f"SEVEN_COLOR order must be black,white,yellow,red,blue,green,orange, got {seven_color_keys}"
        )

        bwgbry_split_keys = list(ColorScheme.BWGBRY_SPLIT.palette.colors.keys())
        assert bwgbry_split_keys == ["black", "white", "yellow", "red", "blue", "green"], (
            f"BWGBRY_SPLIT order must be black,white,yellow,red,blue,green, got {bwgbry_split_keys}"
        )

    def test_from_value_method(self):
        """Test ColorScheme.from_value() works correctly."""
        assert ColorScheme.from_value(0) == ColorScheme.MONO
        assert ColorScheme.from_value(1) == ColorScheme.BWR
        assert ColorScheme.from_value(7) == ColorScheme.SEVEN_COLOR
        assert ColorScheme.from_value(8) == ColorScheme.BWGBRY_SPLIT

        with pytest.raises(ValueError):
            ColorScheme.from_value(99)


class TestDitherMode:
    """Test DitherMode enum."""

    def test_all_modes_defined(self):
        """Test all expected dithering modes are defined."""
        expected_modes = [
            DitherMode.NONE,
            DitherMode.BURKES,
            DitherMode.ORDERED,
            DitherMode.FLOYD_STEINBERG,
            DitherMode.ATKINSON,
            DitherMode.STUCKI,
            DitherMode.SIERRA,
            DitherMode.SIERRA_LITE,
            DitherMode.JARVIS_JUDICE_NINKE,
        ]

        for mode in expected_modes:
            assert mode in DitherMode

    def test_mode_values(self):
        """Test DitherMode values match expected integers."""
        assert DitherMode.NONE == 0
        assert DitherMode.BURKES == 1
        assert DitherMode.FLOYD_STEINBERG == 3


class TestMeasuredPaletteIntegration:
    """Test measured palette integration with dithering."""

    def test_dithering_accepts_colorpalette(self, small_test_image):
        """Test ColorPalette accepted by dither_image."""
        from epaper_dithering import ColorPalette

        measured = ColorPalette(
            colors={"black": (2, 2, 2), "white": (179, 182, 171), "red": (117, 10, 0)}, accent="red"
        )
        result = dither_image(small_test_image, measured, mode=DitherMode.BURKES)

        assert result.mode == "P"
        assert result.size == small_test_image.size

    def test_backward_compatibility_colorscheme(self, small_test_image):
        """Test existing ColorScheme API still works unchanged."""
        result = dither_image(small_test_image, ColorScheme.BWR)
        assert result.mode == "P"

    def test_predefined_measured_palettes_work(self, small_test_image):
        """Test exported measured palette constants."""
        from epaper_dithering import HANSHOW_BWR, MONO_4_26, SPECTRA_7_3_6COLOR

        result = dither_image(small_test_image, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES)
        assert result.mode == "P"

        result = dither_image(small_test_image, MONO_4_26, mode=DitherMode.FLOYD_STEINBERG)
        assert result.mode == "P"

        result = dither_image(small_test_image, HANSHOW_BWR, mode=DitherMode.SIERRA)
        assert result.mode == "P"

    def test_predefined_measured_palettes_record_canonical_scheme(self):
        """Measured palettes identify the pure display palette they produce."""
        from epaper_dithering import BWRY_3_97, HANSHOW_BWR, SPECTRA_7_3_6COLOR

        assert SPECTRA_7_3_6COLOR.scheme == ColorScheme.BWGBRY
        assert BWRY_3_97.scheme == ColorScheme.BWRY
        assert HANSHOW_BWR.scheme == ColorScheme.BWR

    def test_none_uses_canonical_indices_for_measured_palette(self):
        """DitherMode.NONE maps pure display colors directly for measured palettes."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (4, 4), ColorScheme.BWGBRY.palette.colors["red"])
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.NONE)
        pixels = list(result.get_flattened_data())

        assert set(pixels) == {3}

    @pytest.mark.parametrize("mode", [DitherMode.ORDERED, DitherMode.BURKES, DitherMode.FLOYD_STEINBERG])
    def test_exact_display_colors_are_pinned_inside_mixed_measured_image(self, mode):
        """Pure display-color pixels stay exact even when neighboring pixels need dithering."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (8, 4), (128, 128, 128))
        for x in range(4):
            for y in range(2):
                img.putpixel((x, y), ColorScheme.BWGBRY.palette.colors["green"])

        result = dither_image(img, SPECTRA_7_3_6COLOR, mode=mode)
        pixels = list(result.get_flattened_data())
        green_pixels = [pixels[y * 8 + x] for y in range(2) for x in range(4)]

        assert set(green_pixels) == {5}

    def test_predefined_measured_palette_outputs_measured_preview_palette(self):
        """Indices stay canonical-compatible; the PIL preview palette stays measured."""
        from epaper_dithering import SPECTRA_7_3_6COLOR

        img = Image.new("RGB", (1, 1), ColorScheme.BWGBRY.palette.colors["red"])
        result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.NONE)
        palette = result.getpalette()

        assert palette is not None
        assert tuple(palette[3 * 3 : 3 * 3 + 3]) == SPECTRA_7_3_6COLOR.colors["red"]


class TestPureColorMapping:
    """Test that pure palette colors map to themselves."""

    @pytest.mark.parametrize("scheme", list(ColorScheme))
    def test_pure_colors_map_to_own_index(self, scheme):
        """Each palette color should map to its own index with DitherMode.NONE."""
        for idx, (name, rgb) in enumerate(scheme.palette.colors.items()):
            img = Image.new("RGB", (4, 4), rgb)
            result = dither_image(img, scheme, mode=DitherMode.NONE)
            pixels = list(result.get_flattened_data())
            assert all(p == idx for p in pixels), (
                f"{scheme.name}: {name} {rgb} should map to index {idx}, got {set(pixels)}"
            )


class TestSpectraNormalization:
    """Test that SPECTRA measured values match raw measurements + paper reference."""

    def test_spectra_values_match_normalization(self):
        """Verify SPECTRA_7_3_6COLOR matches raw values normalized by paper reference.

        Raw values from colors.txt, paper reference (215, 217, 218).
        Formula: normalized = round(raw * 255 / paper_channel)
        """
        from epaper_dithering import SPECTRA_7_3_6COLOR

        paper = (215, 217, 218)
        raw_colors = {
            "black": (22, 11, 30),
            "white": (156, 172, 175),
            "yellow": (170, 157, 0),
            "red": (102, 8, 0),
            "blue": (0, 59, 119),
            "green": (34, 70, 49),
        }

        for name, raw in raw_colors.items():
            expected = tuple(round(raw[c] * 255 / paper[c]) for c in range(3))
            actual = SPECTRA_7_3_6COLOR.colors[name]
            assert actual == expected, f"{name}: expected {expected} from normalization, got {actual}"

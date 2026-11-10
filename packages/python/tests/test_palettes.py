"""Runtime palettes: loading, validation, ink identity and how dither_image uses them."""

from pathlib import Path

import pytest
from epaper_dithering import (
    DitherMode,
    DitherOptions,
    Palette,
    PaletteEntry,
    PaletteError,
    dither_image,
)
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = ROOT / "examples/palettes"
SUCCESS = ROOT / "packages/rust/core/tests/fixtures"


def example(name: str) -> Palette:
    return Palette.load(EXAMPLES / f"{name}.json")


class TestLoading:
    @pytest.mark.parametrize("path", sorted(EXAMPLES.glob("*.json")), ids=lambda p: p.stem)
    def test_every_example_palette_loads(self, path):
        palette = Palette.load(path)
        assert len(palette) >= 2

    def test_from_json_round_trips(self):
        palette = example("spectra_7_3_6color")
        again = Palette.from_json(palette.to_json())
        assert again.entries == palette.entries
        assert again.measured is True
        assert again.source == palette.source

    def test_entries_carry_appearance_and_ink(self):
        palette = example("ideal_bwr")
        black = palette.entries[0]
        assert black.name == "black"
        assert black.nominal == (0, 0, 0)
        assert black.appearance[0] == pytest.approx(0.0, abs=1e-6)

    def test_index_of(self):
        palette = example("ideal_bwry")
        assert palette.index_of("red") == 3
        assert palette.index_of("blue") is None


class TestValidation:
    def test_rejects_unnormalised_white_and_names_the_fix(self):
        entries = [
            PaletteEntry((68.26, 8.33, 5.98), name="white", nominal="#ffffff"),
            PaletteEntry((30.47, 8.7, 4.58), name="black", nominal="#000000"),
        ]
        with pytest.raises(PaletteError, match="from_absolute_lab"):
            Palette(entries, measured=True)

    def test_from_absolute_lab_normalises_white(self):
        entries = [
            PaletteEntry((30.47, 8.7, 4.58)),
            PaletteEntry((68.26, 8.33, 5.98), name="white", nominal="#ffffff"),
        ]
        palette = Palette.from_absolute_lab(entries, white_index=1, measured=True)
        assert palette.entries[1].appearance == pytest.approx((100.0, 0.0, 0.0), abs=1e-9)

    def test_duplicate_name_raises_palette_error(self):
        entries = [
            PaletteEntry((0.0, 0.0, 0.0), name="black", nominal="#000000"),
            PaletteEntry((20.0, 0.0, 0.0), name="black", nominal="#111111"),
        ]
        with pytest.raises(PaletteError, match="black"):
            Palette(entries)

    def test_palette_error_is_a_value_error(self):
        assert issubclass(PaletteError, ValueError)

    def test_name_without_nominal_is_rejected(self):
        with pytest.raises(ValueError):
            PaletteEntry((50.0, 0.0, 0.0), name="grey")

    def test_bad_json_raises_palette_error(self):
        with pytest.raises(PaletteError):
            Palette.from_json("{")

    @pytest.mark.parametrize("nominal", ["#fff", "#gggggg", "ffffffff"])
    def test_bad_nominal_string_is_rejected(self, nominal):
        with pytest.raises(ValueError, match="#rrggbb"):
            PaletteEntry((50.0, 0.0, 0.0), name="grey", nominal=nominal)

    def test_from_srgb_rejects_name_without_nominal(self):
        with pytest.raises(ValueError, match="both a name and a nominal"):
            Palette.from_srgb([((0, 0, 0), "black", None), ((255, 255, 255), None, None)])

    def test_with_source_returns_a_copy_with_the_note(self):
        palette = example("ideal_mono")
        noted = palette.with_source("measured in the lab")
        assert noted.source == "measured in the lab"
        assert palette.source != "measured in the lab"

    def test_repr_names_the_inks(self):
        assert repr(example("ideal_bwr")) == "Palette(3 entries, inks=['black', 'white', 'red'], measured=False)"

    def test_from_srgb_accepts_tuples_and_hex(self):
        palette = Palette.from_srgb([((0, 0, 0), "black", "#000000"), ((255, 255, 255), "white", (255, 255, 255))])
        assert palette.preview_rgb() == [(0, 0, 0), (255, 255, 255)]
        assert palette.measured is False


class TestOptions:
    def test_defaults_turn_anchor_and_snap_on(self):
        options = DitherOptions()
        assert options.anchor is True
        assert options.snap is True
        assert options.mode == DitherMode.BURKES

    def test_mode_accepts_string_and_enum(self, small_test_image):
        palette = example("ideal_mono")
        by_name = dither_image(small_test_image, palette, DitherOptions(mode="floyd_steinberg"))
        by_enum = dither_image(small_test_image, palette, DitherOptions(mode=DitherMode.FLOYD_STEINBERG))
        assert list(by_name.getdata()) == list(by_enum.getdata())

    def test_non_numeric_gamut_raises_type_error(self, small_test_image):
        with pytest.raises(TypeError, match="gamut must be a number"):
            dither_image(small_test_image, example("ideal_mono"), DitherOptions(gamut="strong"))

    @pytest.mark.parametrize("value", ["auto", "off"])
    def test_auto_and_off_strings_are_gone(self, small_test_image, value):
        with pytest.raises(TypeError):
            dither_image(small_test_image, example("ideal_mono"), DitherOptions(tone=value))

    @pytest.mark.parametrize("field", ["tone", "gamut"])
    @pytest.mark.parametrize("value", [-0.1, 1.5, float("nan")])
    def test_strength_outside_zero_to_one_raises_value_error(self, small_test_image, field, value):
        with pytest.raises(ValueError, match=field):
            dither_image(small_test_image, example("ideal_mono"), DitherOptions(**{field: value}))

    def test_unknown_mode_raises_value_error(self, small_test_image):
        with pytest.raises(ValueError, match="bayer"):
            dither_image(small_test_image, example("ideal_mono"), DitherOptions(mode="bayer"))


class TestDitherImage:
    def test_output_is_p_image_with_preview_palette(self, small_test_image):
        palette = example("spectra_7_3_6color")
        out = dither_image(small_test_image, palette)
        assert out.mode == "P"
        flat = out.getpalette()[: 3 * len(palette)]
        assert [tuple(flat[i : i + 3]) for i in range(0, len(flat), 3)] == palette.preview_rgb()

    def test_pure_black_text_is_solid(self):
        palette = Palette.load(SUCCESS / "palettes/bwry16_measured.json")
        image = Image.open(SUCCESS / "images/success/dashboard_photo.png").convert("RGB")
        out = dither_image(image, palette)
        black = palette.index_of("black")
        for src, idx in zip(image.getdata(), out.getdata(), strict=True):
            if src == (0, 0, 0):
                assert idx == black

    def test_rgba_composited_on_white(self):
        image = Image.new("RGBA", (4, 4), (0, 0, 0, 0))
        out = dither_image(image, example("ideal_mono"), DitherOptions(mode="none"))
        assert set(out.getdata()) == {1}

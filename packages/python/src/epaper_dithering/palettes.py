"""Color palettes and schemes for e-paper displays."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


@dataclass(frozen=True)
class ColorPalette:
    """Color palette for an e-paper display."""

    colors: dict[str, tuple[int, int, int]]  # name -> RGB tuple
    accent: str  # Primary accent color name
    scheme: ColorScheme | None = None  # Canonical firmware color scheme, if known


class ColorScheme(Enum):
    """Common e-ink display color schemes.

    Each scheme stores a color palette optimized for e-paper displays.
    The integer values match OpenDisplay firmware conventions for compatibility.

    Usage:
        scheme = ColorScheme.BWR
        scheme.value           # 1 (firmware value)
        scheme.name            # "BWR"
        scheme.palette.colors  # {'black': (0,0,0), 'white': (255,255,255), 'red': (255,0,0)}
        scheme.accent_color    # "red"
    """

    MONO = (
        0,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
            },
            accent="black",
        ),
    )

    BWR = (
        1,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
                "red": (255, 0, 0),
            },
            accent="red",
        ),
    )

    BWY = (
        2,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
                "yellow": (255, 255, 0),
            },
            accent="yellow",
        ),
    )

    BWRY = (
        3,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
                "yellow": (255, 255, 0),
                "red": (255, 0, 0),
            },
            accent="red",
        ),
    )

    BWGBRY = (
        4,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
                "yellow": (255, 255, 0),
                "red": (255, 0, 0),
                "blue": (0, 0, 255),
                "green": (0, 255, 0),
            },
            accent="red",
        ),
    )

    GRAYSCALE_4 = (
        5,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "gray1": (85, 85, 85),
                "gray2": (170, 170, 170),
                "white": (255, 255, 255),
            },
            accent="black",
        ),
    )

    GRAYSCALE_16 = (
        6,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "gray1": (17, 17, 17),
                "gray2": (34, 34, 34),
                "gray3": (51, 51, 51),
                "gray4": (68, 68, 68),
                "gray5": (85, 85, 85),
                "gray6": (102, 102, 102),
                "gray7": (119, 119, 119),
                "gray8": (136, 136, 136),
                "gray9": (153, 153, 153),
                "gray10": (170, 170, 170),
                "gray11": (187, 187, 187),
                "gray12": (204, 204, 204),
                "gray13": (221, 221, 221),
                "gray14": (238, 238, 238),
                "white": (255, 255, 255),
            },
            accent="black",
        ),
    )

    # Value 7: SEVEN_COLOR (protocol v2). A former GRAYSCALE_8 = 7 was reassigned
    # to SEVEN_COLOR; GRAYSCALE_8 now lives at library-local value 9 (below).
    # Ink order is BWGBRY plus orange, matching the bb_epaper logical ink indices.
    SEVEN_COLOR = (
        7,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
                "yellow": (255, 255, 0),
                "red": (255, 0, 0),
                "blue": (0, 0, 255),
                "green": (0, 255, 0),
                "orange": (255, 128, 0),
            },
            accent="red",
        ),
    )

    # Same inks as BWGBRY; only the firmware-side plane packing differs
    # (left-half plane then right-half plane, for dual-CS panels).
    BWGBRY_SPLIT = (
        8,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "white": (255, 255, 255),
                "yellow": (255, 255, 0),
                "red": (255, 0, 0),
                "blue": (0, 0, 255),
                "green": (0, 255, 0),
            },
            accent="red",
        ),
    )

    # Value 9 is LIBRARY-LOCAL, NOT a firmware wire value. The canonical
    # `enum ColorScheme` in opendisplay_structs.h defines values 0-8 and
    # 100-102 only -- it has no value 9 and none is expected to be assigned
    # here. This exists purely to serve 8-level-gray panels outside the
    # OpenDisplay ecosystem (e.g. the Inkplate 10, issue #19). NEVER send this
    # value to a device as a `color_scheme` field.
    GRAYSCALE_8 = (
        9,
        ColorPalette(
            colors={
                "black": (0, 0, 0),
                "gray1": (36, 36, 36),
                "gray2": (73, 73, 73),
                "gray3": (109, 109, 109),
                "gray4": (146, 146, 146),
                "gray5": (182, 182, 182),
                "gray6": (219, 219, 219),
                "white": (255, 255, 255),
            },
            accent="black",
        ),
    )

    def __init__(self, value: int, palette: ColorPalette):
        self._value_ = value  # type: ignore[assignment]
        self.palette = palette

    @property
    def accent_color(self) -> str:
        """Get accent color name for this scheme."""
        return self.palette.accent

    @property
    def color_count(self) -> int:
        """Get number of colors in palette."""
        return len(self.palette.colors)

    @classmethod
    def from_value(cls, value: int) -> ColorScheme:
        """Get ColorScheme from firmware int value.

        Args:
            value: Firmware color scheme value (0-8), or the library-local
                GRAYSCALE_8 value (9, not a firmware wire value)

        Returns:
            Matching ColorScheme

        Raises:
            ValueError: If value is invalid
        """
        for scheme in cls:
            if scheme.value == value:  # type: ignore[comparison-overlap]
                return scheme
        raise ValueError(f"Invalid color scheme value: {value}")


# ============================================================================
# Measured Palettes for Specific E-Paper Displays
# ============================================================================
#
# These constants are derived from the Rust core at import time — RGB values
# are defined once in packages/rust/core/src/measured_palettes.rs.
#
# To add a new display palette:
#   1. Add a new `pub static` palette + CATALOG entry in measured_palettes.rs
#   2. Add the constant name here and export it in __init__.py
#   No RGB values needed in Python.
#
# ============================================================================


def _load_measured_palettes() -> dict[str, "ColorPalette"]:
    from . import _rs  # noqa: PLC0415  (local import avoids circular dependency at module level)

    result: dict[str, ColorPalette] = {}
    for name, rgb_bytes, color_names, accent_idx, scheme_id in _rs.measured_palettes():
        colors = {
            color_names[i]: (rgb_bytes[i * 3], rgb_bytes[i * 3 + 1], rgb_bytes[i * 3 + 2])
            for i in range(len(color_names))
        }
        result[name] = ColorPalette(
            colors=colors,
            accent=color_names[accent_idx],
            scheme=ColorScheme.from_value(scheme_id),
        )
    return result


_MEASURED = _load_measured_palettes()

SPECTRA_7_3_6COLOR: ColorPalette = _MEASURED["SPECTRA_7_3_6COLOR"]
SPECTRA_7_3_6COLOR_V2: ColorPalette = _MEASURED["SPECTRA_7_3_6COLOR_V2"]
MONO_4_26: ColorPalette = _MEASURED["MONO_4_26"]
BWRY_4_2: ColorPalette = _MEASURED["BWRY_4_2"]
BWRY_3_97: ColorPalette = _MEASURED["BWRY_3_97"]
SOLUM_BWR: ColorPalette = _MEASURED["SOLUM_BWR"]
HANSHOW_BWR: ColorPalette = _MEASURED["HANSHOW_BWR"]
HANSHOW_BWY: ColorPalette = _MEASURED["HANSHOW_BWY"]

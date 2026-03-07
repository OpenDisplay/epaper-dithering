"""Color palettes and schemes for e-paper displays."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


@dataclass(frozen=True)
class ColorPalette:
    """Color palette for an e-paper display."""

    colors: dict[str, tuple[int, int, int]]  # name -> RGB tuple
    accent: str  # Primary accent color name


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

    # NOTE: Values 6 and 7 are placeholders pending firmware assignment.
    GRAYSCALE_8 = (
        6,
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

    GRAYSCALE_16 = (
        7,
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
            value: Firmware color scheme value (0-7)

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
# These constants provide measured RGB values from real e-paper displays
# for more accurate dithering. Pure RGB colors (e.g., White=255,255,255,
# Red=255,0,0) are much brighter than real displays, which are typically
# 30-87% darker due to reflective screen technology.
#
# USAGE:
#     from epaper_dithering import dither_image, SPECTRA_7_3_6COLOR
#     result = dither_image(img, SPECTRA_7_3_6COLOR)
#
# TO ADD YOUR DISPLAY:
#     1. Measure colors following docs/CALIBRATION.md
#     2. Create ColorPalette with measured values
#     3. Add constant here
#     4. Export in __init__.py
#
# IMPORTANT: Color names and order MUST match the corresponding ColorScheme!
#            Reordering colors will break palette encoding compatibility.
#
# STATUS: All values below are THEORETICAL/PLACEHOLDER until measured.
#         See docs/CALIBRATION.md for measurement procedures.
# ============================================================================

# 7.3" Spectra™ 6-color (BWGBRY scheme)
# Measured: 2026-02-03
# Equipment: iPhone 15 Pro Max RAW + Hue Play bars @ 6500K (154 mireds)
# Method: Photographed calibration patches with white paper reference
# Raw values in colors.txt, paper reference RGB(215,217,218)
# Normalization: per-channel scaling value × (255 / paper_channel)
SPECTRA_7_3_6COLOR = ColorPalette(
    colors={
        "black": (26, 13, 35),
        "white": (185, 202, 205),
        "yellow": (202, 184, 0),
        "red": (121, 9, 0),
        "blue": (0, 69, 139),
        "green": (40, 82, 57),
    },
    accent="red",
)

# 4.26" Monochrome (MONO scheme)
# TODO: Measure actual display
MONO_4_26 = ColorPalette(
    colors={
        "black": (5, 5, 5),  # Measure: likely darker than pure black
        "white": (220, 220, 220),  # Measure: real displays ~200-230, not 255
    },
    accent="black",
)

# 4.2" BWRY (BWRY scheme)
# TODO: Measure actual display
BWRY_4_2 = ColorPalette(
    colors={
        "black": (5, 5, 5),  # Measure
        "white": (200, 200, 200),  # Measure
        "yellow": (200, 180, 0),  # Measure
        "red": (120, 15, 5),  # Measure
    },
    accent="red",
)

# Solum BWR (harvested display, BWR scheme)
# TODO: Measure actual display
SOLUM_BWR = ColorPalette(
    colors={
        "black": (5, 5, 5),  # Measure
        "white": (200, 200, 200),  # Measure
        "red": (120, 15, 5),  # Measure
    },
    accent="red",
)

# Hanshow BWR (harvested display, BWR scheme)
# TODO: Measure actual display
HANSHOW_BWR = ColorPalette(
    colors={
        "black": (5, 5, 5),  # Measure
        "white": (200, 200, 200),  # Measure
        "red": (120, 15, 5),  # Measure
    },
    accent="red",
)

# Hanshow BWY (harvested display, BWY scheme)
# TODO: Measure actual display
HANSHOW_BWY = ColorPalette(
    colors={
        "black": (5, 5, 5),  # Measure
        "white": (200, 200, 200),  # Measure
        "yellow": (200, 180, 0),  # Measure
    },
    accent="yellow",
)

# 3.97" BWRY — EP397YR_800x480 (panel_ic_type=0x37 / 55), BWRY scheme
# Measured: 2026-03-06
# Equipment: iPhone RAW
# Method: Photographed calibration patches with white paper reference
# Paper reference RGB(205,205,205); normalization: value × (255/205) per channel
# Yellow blue channel clipped to 0 (expected for yellow; kept as-is)
BWRY_3_97 = ColorPalette(
    colors={
        "black": (10, 7, 14),
        "white": (173, 178, 174),
        "yellow": (172, 128, 0),
        "red": (85, 24, 14),
    },
    accent="red",
)

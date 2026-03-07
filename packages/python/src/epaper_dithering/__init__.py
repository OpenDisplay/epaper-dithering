"""E-ink display dithering algorithms.

A standalone library providing multiple dithering algorithms optimized
for limited-color e-paper/e-ink displays.
"""

from .core import dither_image
from .enums import DitherMode
from .palettes import (
    BWRY_3_97,
    BWRY_4_2,
    HANSHOW_BWR,
    HANSHOW_BWY,
    MONO_4_26,
    SOLUM_BWR,
    SPECTRA_7_3_6COLOR,
    ColorPalette,
    ColorScheme,
)

__version__ = "0.6.4"

__all__ = [
    "dither_image",
    "DitherMode",
    "ColorPalette",
    "ColorScheme",
    # Measured palettes for specific displays (v0.4.0)
    "SPECTRA_7_3_6COLOR",
    "MONO_4_26",
    "BWRY_4_2",
    "BWRY_3_97",
    "SOLUM_BWR",
    "HANSHOW_BWR",
    "HANSHOW_BWY",
]

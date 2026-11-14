"""Dithering for e-paper displays: an image and a palette in, palette indices out.

Load or build a `Palette` for your panel, then call `dither_image`. The library ships no
built-in palettes; see `examples/palettes/` in the repository for starting points.
"""

from .core import DbsParams, DitherOptions, dither_image
from .enums import DitherMode
from .palette import Palette, PaletteEntry, PaletteError

__version__ = "7.0.1"

__all__ = [
    "dither_image",
    "DitherOptions",
    "DbsParams",
    "DitherMode",
    "Palette",
    "PaletteEntry",
    "PaletteError",
]

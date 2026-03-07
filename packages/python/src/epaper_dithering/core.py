"""Main dithering interface."""

from __future__ import annotations

import logging

from PIL import Image

from . import algorithms
from .enums import DitherMode
from .palettes import ColorPalette, ColorScheme

_LOGGER = logging.getLogger(__name__)


def dither_image(
    image: Image.Image,
    color_scheme: ColorScheme | ColorPalette,
    mode: DitherMode = DitherMode.BURKES,
    serpentine: bool = True,
    tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply dithering to image for e-paper display.

    Args:
        image: Input image (RGB or RGBA)
        color_scheme: Target display color scheme OR measured ColorPalette
        mode: Dithering algorithm (default: BURKES)
        serpentine: Use serpentine scanning for error diffusion (default: True).
            Alternates scan direction each row to reduce directional artifacts.
            Only applies to error diffusion algorithms, ignored for NONE and ORDERED.
        tone_compression: Dynamic range compression (default: "auto").
            "auto" = analyze image histogram and fit to display range.
            0.0 = disabled, 0.0-1.0 = fixed linear compression strength.
            Only applies to measured ColorPalette.

    Returns:
        Dithered palette image matching color scheme
    """
    # Log color scheme name if available
    scheme_name = color_scheme.name if isinstance(color_scheme, ColorScheme) else "custom"
    _LOGGER.debug("Applying %s dithering for %s palette", mode.name, scheme_name)

    tc = tone_compression
    match mode:
        case DitherMode.NONE:
            return algorithms.direct_palette_map(image, color_scheme, tc)
        case DitherMode.ORDERED:
            return algorithms.ordered_dither(image, color_scheme, tc)
        case DitherMode.FLOYD_STEINBERG:
            return algorithms.floyd_steinberg_dither(image, color_scheme, serpentine, tc)
        case DitherMode.ATKINSON:
            return algorithms.atkinson_dither(image, color_scheme, serpentine, tc)
        case DitherMode.STUCKI:
            return algorithms.stucki_dither(image, color_scheme, serpentine, tc)
        case DitherMode.SIERRA:
            return algorithms.sierra_dither(image, color_scheme, serpentine, tc)
        case DitherMode.SIERRA_LITE:
            return algorithms.sierra_lite_dither(image, color_scheme, serpentine, tc)
        case DitherMode.JARVIS_JUDICE_NINKE:
            return algorithms.jarvis_judice_ninke_dither(image, color_scheme, serpentine, tc)
        case _:  # BURKES or fallback
            return algorithms.burkes_dither(image, color_scheme, serpentine, tc)

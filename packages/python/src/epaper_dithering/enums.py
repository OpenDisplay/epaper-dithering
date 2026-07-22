"""Enumeration types for dithering algorithms."""

from __future__ import annotations

from enum import IntEnum


class DitherMode(IntEnum):
    """Image dithering algorithms for e-paper displays.

    Values are compatible with OpenDisplay firmware conventions.
    """

    #: Direct nearest-color mapping with no error diffusion. Intended for
    #: already-quantized graphics only. On limited palettes (especially BWR),
    #: continuous-tone photos or large flat mid-tone areas can map to an
    #: unexpected ink (e.g. a mid-gray region rendered as solid red); use an
    #: error-diffusion mode (e.g. FLOYD_STEINBERG, BURKES) for photographic input.
    NONE = 0
    BURKES = 1
    ORDERED = 2
    FLOYD_STEINBERG = 3
    ATKINSON = 4
    STUCKI = 5
    SIERRA = 6
    SIERRA_LITE = 7
    JARVIS_JUDICE_NINKE = 8
    #: Error diffusion with pseudo-random traversal instead of a raster scan,
    #: diffusing error only to not-yet-quantized neighbours. Produces no
    #: directional structure. ``serpentine`` is ignored: there is no scan direction.
    DIZZY = 9

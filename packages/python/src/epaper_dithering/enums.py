"""Enumeration types for dithering algorithms."""

from __future__ import annotations

from enum import IntEnum


class DitherMode(IntEnum):
    """Image dithering algorithms for e-paper displays.

    Values are compatible with OpenDisplay firmware conventions.
    """

    NONE = 0
    BURKES = 1
    ORDERED = 2
    FLOYD_STEINBERG = 3
    ATKINSON = 4
    STUCKI = 5
    SIERRA = 6
    SIERRA_LITE = 7
    JARVIS_JUDICE_NINKE = 8

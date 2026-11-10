"""Enumeration types for dithering algorithms."""

from __future__ import annotations

from enum import StrEnum


class DitherMode(StrEnum):
    """Image dithering algorithms. Values are the names the core accepts."""

    NONE = "none"
    BURKES = "burkes"
    ORDERED = "ordered"
    FLOYD_STEINBERG = "floyd_steinberg"
    ATKINSON = "atkinson"
    STUCKI = "stucki"
    SIERRA = "sierra"
    SIERRA_LITE = "sierra_lite"
    JARVIS_JUDICE_NINKE = "jarvis_judice_ninke"
    DIZZY = "dizzy"

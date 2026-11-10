"""Load the repository's example palette files (`examples/palettes/`) for the dev tools."""

from __future__ import annotations

from pathlib import Path

from epaper_dithering import Palette

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples/palettes"


def load(name: str) -> Palette:
    """Load `examples/palettes/<name>.json`."""
    return Palette.load(EXAMPLES / f"{name}.json")


def ideal() -> list[tuple[str, Palette]]:
    """The idealised palettes, labelled like the old schemes (e.g. "BWR")."""
    return [(p.stem.removeprefix("ideal_").upper(), Palette.load(p)) for p in sorted(EXAMPLES.glob("ideal_*.json"))]


def measured() -> list[tuple[str, Palette]]:
    """The measured palettes, labelled by file name (e.g. "SPECTRA_7_3_6COLOR")."""
    return [
        (p.stem.upper(), Palette.load(p))
        for p in sorted(EXAMPLES.glob("*.json"))
        if not p.stem.startswith("ideal_")
    ]

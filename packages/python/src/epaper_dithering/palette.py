"""Runtime palettes.

A palette is data supplied by whoever owns the hardware: one entry per colour the panel can
show, in output-index order. Each entry has its measured appearance (CIELAB D65, normalised so
the panel's white is L* 100, a* = b* = 0) and, optionally, the identity of a pure ink: a name
plus the "pure" sRGB colour people ask for when they mean that ink.

All validation happens in the Rust core; this module only adapts Python values to it.
"""

from __future__ import annotations

import os
from collections.abc import Sequence
from dataclasses import dataclass

import epaper_dithering._rs as _rs

PaletteError = _rs.PaletteError

RGB = tuple[int, int, int]
Nominal = str | RGB


def _parse_nominal(value: Nominal) -> RGB:
    if isinstance(value, str):
        text = value.removeprefix("#")
        if len(text) != 6:
            raise ValueError(f"nominal colour {value!r} is not #rrggbb")
        try:
            return (int(text[0:2], 16), int(text[2:4], 16), int(text[4:6], 16))
        except ValueError as exc:
            raise ValueError(f"nominal colour {value!r} is not #rrggbb") from exc
    r, g, b = value
    return (int(r), int(g), int(b))


@dataclass(frozen=True)
class PaletteEntry:
    """One palette colour.

    Attributes:
        appearance: How the colour looks on the panel, as (L*, a*, b*).
        name: Ink name, e.g. "black". Only for pure inks; mixes leave it unset.
        nominal: The colour that requests this ink, as "#rrggbb" or an (r, g, b) tuple.
            Required exactly when `name` is set.
    """

    appearance: tuple[float, float, float]
    name: str | None = None
    nominal: RGB | None = None

    def __init__(
        self,
        appearance: tuple[float, float, float],
        name: str | None = None,
        nominal: Nominal | None = None,
    ) -> None:
        if (name is None) != (nominal is None):
            raise ValueError("an ink needs both a name and a nominal colour")
        l_star, a_star, b_star = appearance
        object.__setattr__(self, "appearance", (float(l_star), float(a_star), float(b_star)))
        object.__setattr__(self, "name", name)
        object.__setattr__(self, "nominal", None if nominal is None else _parse_nominal(nominal))

    def as_tuple(self) -> tuple[tuple[float, float, float], str | None, RGB | None]:
        """The entry as `(appearance, name, nominal)`."""
        return (self.appearance, self.name, self.nominal)


class Palette:
    """A validated palette. Raises `PaletteError` (a `ValueError`) when invalid."""

    __slots__ = ("_inner",)

    def __init__(self, entries: Sequence[PaletteEntry], measured: bool = False) -> None:
        self._inner = _rs.Palette([e.as_tuple() for e in entries], measured)

    @classmethod
    def _wrap(cls, inner: _rs.Palette) -> Palette:
        palette = cls.__new__(cls)
        palette._inner = inner
        return palette

    @classmethod
    def from_json(cls, text: str) -> Palette:
        """Load an `epaper-palette` JSON document."""
        return cls._wrap(_rs.Palette.from_json(text))

    @classmethod
    def load(cls, path: str | os.PathLike[str]) -> Palette:
        """Load an `epaper-palette` JSON file."""
        with open(path, encoding="utf-8") as fh:
            return cls.from_json(fh.read())

    @classmethod
    def from_absolute_lab(cls, entries: Sequence[PaletteEntry], white_index: int, measured: bool = False) -> Palette:
        """Build from absolute measurements, normalising so `entries[white_index]` is L* 100."""
        return cls._wrap(_rs.Palette.from_absolute_lab([e.as_tuple() for e in entries], white_index, measured))

    @classmethod
    def from_srgb(
        cls,
        colors: Sequence[tuple[RGB, str | None, Nominal | None]],
        measured: bool = False,
    ) -> Palette:
        """Build from sRGB colours, each `(rgb, ink_name, ink_nominal)`.

        If an ink named "white" is present, every colour is normalised to it.
        """
        converted = []
        for rgb, name, nominal in colors:
            if (name is None) != (nominal is None):
                raise ValueError("an ink needs both a name and a nominal colour")
            converted.append((_parse_nominal(rgb), name, None if nominal is None else _parse_nominal(nominal)))
        return cls._wrap(_rs.Palette.from_srgb(converted, measured))

    def with_source(self, source: str) -> Palette:
        """A copy with a free-text note of where the palette came from."""
        return Palette._wrap(self._inner.with_source(source))

    def to_json(self) -> str:
        """Serialise as an `epaper-palette` JSON document."""
        return self._inner.to_json()

    @property
    def entries(self) -> list[PaletteEntry]:
        """Every entry, in output-index order."""
        return [PaletteEntry(appearance, name, nominal) for appearance, name, nominal in self._inner.entries()]

    @property
    def measured(self) -> bool:
        """Whether the appearances were measured on this unit (metadata only)."""
        return self._inner.measured

    @property
    def source(self) -> str | None:
        """Free-text note of where the palette came from."""
        return self._inner.source

    def index_of(self, name: str) -> int | None:
        """Output index of the ink with this name."""
        return self._inner.index_of(name)

    def preview_rgb(self) -> list[RGB]:
        """sRGB of each entry's appearance: what the panel shows, for previews."""
        return self._inner.preview_rgb()

    def __len__(self) -> int:
        return len(self._inner)

    def __repr__(self) -> str:
        inks = [e.name for e in self.entries if e.name]
        return f"Palette({len(self)} entries, inks={inks}, measured={self.measured})"

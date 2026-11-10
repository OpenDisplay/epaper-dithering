"""Main dithering interface."""

from __future__ import annotations

import logging
import math
from dataclasses import dataclass

from PIL import Image

import epaper_dithering._rs as _rs

from .enums import DitherMode
from .palette import Palette

_LOGGER = logging.getLogger(__name__)


@dataclass(frozen=True)
class DbsParams:
    """Direct Binary Search refinement settings.

    DBS improves a dithered image by repeatedly changing pixels to other inks or swapping
    them with neighbours whenever that lowers the error as perceived through a model of the
    eye at the given viewing geometry. It is slow (seconds on a full panel), so it is off
    unless requested.

    Attributes:
        viewing_distance_cm: Distance between viewer and panel. Larger values blur more.
        ppi: Panel pixel density. A 7.3" 800x480 panel is about 127 ppi.
        max_passes: Upper bound on full-image passes; stops early once nothing improves.
    """

    viewing_distance_cm: float = 40.0
    ppi: float = 127.0
    max_passes: int = 10

    def __post_init__(self) -> None:
        for name in ("viewing_distance_cm", "ppi"):
            value = getattr(self, name)
            if not (math.isfinite(value) and value > 0):
                raise ValueError(f"{name} must be finite and greater than 0, got {value}")
        if self.max_passes < 0:
            raise ValueError(f"max_passes must be >= 0, got {self.max_passes}")


def _dbs_kwargs(dbs: DbsParams | bool | None) -> dict[str, object]:
    """Map the `dbs` argument onto the binding's flat `dbs_*` keyword arguments."""
    if dbs is None or dbs is False:
        return {}
    if dbs is True:
        dbs = DbsParams()
    if not isinstance(dbs, DbsParams):
        raise TypeError(f"dbs must be DbsParams, bool, or None, got {type(dbs).__name__}")
    return {
        "dbs_passes": dbs.max_passes,
        "dbs_viewing_distance_cm": dbs.viewing_distance_cm,
        "dbs_ppi": dbs.ppi,
    }


def _to_rgb_bytes(image: Image.Image) -> tuple[bytes, int, int]:
    """Convert PIL image to flat RGB bytes. Composites RGBA on white."""
    if image.mode == "RGBA":
        # Composite in Rust rather than with PIL's paste, so Python and JavaScript
        # share one implementation and cannot drift on semi-transparent pixels.
        width, height = image.size
        return _rs.composite_rgba(image.tobytes()), width, height
    img_rgb = image.convert("RGB")
    width, height = img_rgb.size
    return img_rgb.tobytes(), width, height


def _compression(v: float | str) -> float | None:
    """Map compression params: 'auto' -> Rust None, 'off' -> 0.0, float -> Some."""
    if v == "auto":
        return None
    if v == "off":
        return 0.0
    if not isinstance(v, (int, float)):
        raise TypeError(f"expected a float, 'auto' or 'off', got {v!r}")
    return float(v)


@dataclass(frozen=True)
class DitherOptions:  # pylint: disable=too-many-instance-attributes
    """Options for `dither_image`. Every field has a sensible default.

    Pipeline, each step a no-op at its identity value:
    exposure → saturation → shadows/highlights → anchor → gamut → match + diffuse → dbs.
    Ink snapping is decided on the original image, before any of these steps.

    Attributes:
        mode: Dithering algorithm, as a `DitherMode` or its name (default "burkes").
        serpentine: Alternate row scan direction for error diffusion.
        exposure: Linear-RGB exposure multiplier. 1.0 = no change, 2.0 = +1 stop.
        saturation: OKLab saturation multiplier. 1.0 = no change, 0.0 = grayscale.
        shadows: Shadow lift strength. 0.0 = off, 1.0 = strong.
        highlights: Highlight compression strength. 0.0 = off, 1.0 = strong.
        gamut: Gamut compression: 0.0 / "off", "auto", or a 0.0–1.0 strength.
        dbs: Direct Binary Search refinement: None/False = off, True = defaults, or `DbsParams`.
        anchor: Map pure black and white onto the panel's own black and white (black-point
            compensation) and each named ink's nominal colour onto its appearance first.
        snap: A pixel exactly equal to a named ink's nominal takes that ink, undithered.
    """

    mode: DitherMode | str = DitherMode.BURKES
    serpentine: bool = True
    exposure: float = 1.0
    saturation: float = 1.0
    shadows: float = 0.0
    highlights: float = 0.0
    gamut: float | str = 0.0
    dbs: DbsParams | bool | None = None
    anchor: bool = True
    snap: bool = True


def dither_image(image: Image.Image, palette: Palette, options: DitherOptions | None = None) -> Image.Image:
    """Dither an image to a palette.

    Args:
        image: Input image (any mode; RGBA is composited on white).
        palette: The panel's palette.
        options: Pipeline options; defaults to `DitherOptions()`.

    Returns:
        A palette-mode ("P") image whose indices are palette entries and whose PIL palette
        holds each entry's preview colour.

    Raises:
        ValueError: An unknown mode name or invalid DBS parameters.
    """
    o = options or DitherOptions()
    _LOGGER.debug("Applying %s dithering to a %d-colour palette", o.mode, len(palette))
    pixels, width, height = _to_rgb_bytes(image)
    indices = _rs.dither_image(
        pixels,
        width,
        height,
        palette._inner,  # pylint: disable=protected-access
        mode=str(o.mode),
        serpentine=o.serpentine,
        exposure=o.exposure,
        saturation=o.saturation,
        shadows=o.shadows,
        highlights=o.highlights,
        gamut=_compression(o.gamut),
        anchor=o.anchor,
        snap=o.snap,
        **_dbs_kwargs(o.dbs),  # type: ignore[arg-type]
    )
    out = Image.new("P", (width, height))
    out.putdata(indices)
    out.putpalette([c for rgb in palette.preview_rgb() for c in rgb])
    return out

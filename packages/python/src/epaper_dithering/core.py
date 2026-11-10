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


def _strength(name: str, value: object) -> float:
    """Validate a 0.0–1.0 strength option."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{name} must be a number from 0.0 to 1.0, got {value!r}")
    if not 0.0 <= value <= 1.0:  # also rejects NaN
        raise ValueError(f"{name} must be from 0.0 to 1.0, got {value!r}")
    return float(value)


@dataclass(frozen=True)
class DitherOptions:  # pylint: disable=too-many-instance-attributes
    """Options for `dither_image`. Every field has a sensible default.

    Pipeline, each step a no-op at its identity value:
    exposure → saturation → shadows/highlights → tone → anchor → gamut → match + diffuse → dbs.
    Ink snapping is decided on the original image, before any of these steps.

    Attributes:
        mode: Dithering algorithm, as a `DitherMode` or its name (default "burkes").
        serpentine: Alternate row scan direction for error diffusion.
        exposure: Linear-RGB exposure multiplier. 1.0 = no change, 2.0 = +1 stop.
        saturation: OKLab saturation multiplier. 1.0 = no change, 0.0 = grayscale.
        shadows: Shadow lift strength. 0.0 = off, 1.0 = strong.
        highlights: Highlight compression strength. 0.0 = off, 1.0 = strong.
        tone: Black-point lift, 0.0 (off) to 1.0: remaps lightness onto the panel's darkest and
            lightest colours, keeping hue. Brings out shadow detail at the cost of deep blacks.
        gamut: Gamut compression, 0.0 (off) to 1.0: pulls out-of-gamut colours toward the
            palette's gamut before matching.
        dbs: Direct Binary Search refinement: None/False = off, True = defaults, or `DbsParams`.
        anchor: Map each named ink's nominal colour onto its measured appearance first; colours
            near an ink follow, colours far from every ink are left alone.
        snap: A pixel exactly equal to a named ink's nominal takes that ink, undithered.
    """

    mode: DitherMode | str = DitherMode.BURKES
    serpentine: bool = True
    exposure: float = 1.0
    saturation: float = 1.0
    shadows: float = 0.0
    highlights: float = 0.0
    tone: float = 0.0
    gamut: float = 0.0
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
        tone=_strength("tone", o.tone),
        gamut=_strength("gamut", o.gamut),
        anchor=o.anchor,
        snap=o.snap,
        **_dbs_kwargs(o.dbs),  # type: ignore[arg-type]
    )
    out = Image.new("P", (width, height))
    out.putdata(indices)
    out.putpalette([c for rgb in palette.preview_rgb() for c in rgb])
    return out

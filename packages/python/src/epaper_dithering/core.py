"""Main dithering interface."""

from __future__ import annotations

import logging
import math
from dataclasses import dataclass
from typing import cast

from PIL import Image

import epaper_dithering._rs as _rs

from .enums import DitherMode
from .palettes import ColorPalette, ColorScheme

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
    return float(v)


def dither_image(  # pylint: disable=too-many-arguments
    image: Image.Image,
    color_scheme: ColorScheme | ColorPalette,
    *,
    mode: DitherMode = DitherMode.BURKES,
    serpentine: bool = True,
    exposure: float = 1.0,
    saturation: float = 1.0,
    shadows: float = 0.0,
    highlights: float = 0.0,
    tone: float | str = 0.0,
    gamut: float | str = 0.0,
    dbs: DbsParams | bool | None = None,
) -> Image.Image:
    """Apply dithering to an image for e-paper display.

    Args:
        image: Input image (RGB or RGBA). RGBA is composited on white.
        color_scheme: Target display palette — `ColorScheme` enum (idealized) or
            measured `ColorPalette` instance.

    Keyword Args:
        mode: Dithering algorithm (default: BURKES).
        serpentine: Alternate row scan direction for error diffusion (default: True).
            Ignored for NONE and ORDERED modes.
        exposure: Linear-RGB exposure multiplier. 1.0 = no change, 2.0 = +1 stop.
        saturation: OKLab saturation multiplier. 1.0 = no change, 0.0 = grayscale.
            Hue-preserving.
        shadows: Shadow lift strength (S-curve lower half). 0.0 = off, 1.0 = strong.
        highlights: Highlight compression strength (S-curve upper half). 0.0 = off, 1.0 = strong.
        tone: Dynamic-range compression. 0.0 = off, "auto" = histogram-based fit
            to display range, 0.0–1.0 = fixed strength. Only meaningful for measured palettes.
        gamut: Gamut compression for out-of-gamut pixels. 0.0 = off, "auto" = full
            strength on out-of-gamut pixels (smoothstep), 0.0–1.0 = fixed strength.
        dbs: Direct Binary Search refinement after dithering. `None`/`False` = off,
            `True` = default `DbsParams()`, or a `DbsParams` for custom viewing geometry.
            Slow: seconds on a full panel.

    Returns:
        Dithered palette-mode (`"P"`) PIL Image matching the color scheme.
    """
    if not isinstance(tone, (float, int, str)):
        raise TypeError(f"tone must be float, 'auto', or 'off', got {type(tone).__name__}")
    if not isinstance(gamut, (float, int, str)):
        raise TypeError(f"gamut must be float, 'auto', or 'off', got {type(gamut).__name__}")

    scheme_name = color_scheme.name if isinstance(color_scheme, ColorScheme) else "custom"
    _LOGGER.debug("Applying %s dithering for %s palette", mode.name, scheme_name)

    pixels, width, height = _to_rgb_bytes(image)

    common_kwargs: dict[str, object] = {
        "mode_id": int(mode),
        "serpentine": serpentine,
        "exposure": exposure,
        "saturation": saturation,
        "shadows": shadows,
        "highlights": highlights,
        "tone": _compression(tone),
        "gamut": _compression(gamut),
        **_dbs_kwargs(dbs),
    }

    if isinstance(color_scheme, ColorScheme):
        # Idealized scheme: tone/gamut auto don't apply; force them off.
        common_kwargs["tone"] = 0.0
        common_kwargs["gamut"] = 0.0
        indices = _rs.dither_image(
            pixels,
            width,
            height,
            scheme_id=color_scheme.value,  # type: ignore[arg-type]  # non-standard enum: _value_ is int
            **common_kwargs,  # type: ignore[arg-type]
        )
        palette_colors = list(color_scheme.palette.colors.values())
    else:
        palette_colors = list(color_scheme.colors.values())
        palette_bytes = bytes(c for rgb in palette_colors for c in rgb)
        accent_idx = list(color_scheme.colors.keys()).index(color_scheme.accent)
        scheme_id = cast(int, color_scheme.scheme.value) if color_scheme.scheme is not None else None
        indices = _rs.dither_image(
            pixels,
            width,
            height,
            scheme_id=scheme_id,
            palette_bytes=palette_bytes,
            accent_idx=accent_idx,
            **common_kwargs,  # type: ignore[arg-type]
        )

    out = Image.new("P", (width, height))
    out.putdata(indices)
    out.putpalette([c for rgb in palette_colors for c in rgb])
    return out

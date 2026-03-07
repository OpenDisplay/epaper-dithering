"""LAB color space conversions and LCH-weighted color matching for dithering.

CIELAB (L*a*b*) is a perceptually uniform color space where equal distances
represent equal perceived color differences. This module uses LAB for color
matching with a dithering-optimized LCH (Lightness-Chroma-Hue) weighting.

Why LCH Weighting for Dithering:
---------------------------------
Standard perceptual distance (Delta E) weights lightness, chroma, and hue
equally. But for error-diffusion dithering, hue preservation matters MORE
than lightness accuracy because:
- Error diffusion compensates for lightness by mixing dark+light pixels spatially
- Error diffusion CANNOT compensate for hue errors (no way to mix green from
  non-green palette colors)

The LCH decomposition uses the CIE identity: da^2 + db^2 = dC^2 + dH^2,
allowing us to weight the three perceptual dimensions independently:
- Lightness (WL=0.5): de-emphasized, error diffusion handles this
- Chroma (WC=1.0): standard weight
- Hue (WH=2.0): emphasized, prevents cross-hue errors like green->yellow

References:
----------
- CIE 1976 L*a*b* color space
- http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html
"""

from __future__ import annotations

import math

import numpy as np

# =============================================================================
# Constants
# =============================================================================

# sRGB to XYZ matrix (D65 illuminant, sRGB primaries)
# From http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html
_M_RGB_TO_XYZ = np.array([
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
], dtype=np.float64)

# D65 reference white point
_XN = 0.95047
_YN = 1.00000
_ZN = 1.08883
_D65_WHITE = np.array([_XN, _YN, _ZN], dtype=np.float64)

# CIE LAB constants
_EPSILON = 216.0 / 24389.0  # 0.008856...
_KAPPA = 24389.0 / 27.0     # 903.296...

# LCH distance weights for dithering
_WL = 0.5   # lightness: de-emphasized (error diffusion compensates)
_WC = 1.0   # chroma: standard weight
_WH = 2.0   # hue: emphasized (error diffusion cannot compensate)


# =============================================================================
# Vectorized Functions (for batch operations: direct mapping, ordered dither)
# =============================================================================

def rgb_to_lab(rgb: np.ndarray) -> np.ndarray:
    """Convert linear RGB to CIELAB color space.

    Args:
        rgb: Linear RGB values in [0, 1] range. Shape: (..., 3)

    Returns:
        LAB values. L in [0, 100], a and b in [-128, 127]. Shape: (..., 3)
    """
    xyz = rgb @ _M_RGB_TO_XYZ.T
    xyz_norm = xyz / _D65_WHITE

    def f(t: np.ndarray) -> np.ndarray:
        mask = t > _EPSILON
        result = np.empty_like(t)
        result[mask] = np.cbrt(t[mask])
        result[~mask] = (_KAPPA * t[~mask] + 16.0) / 116.0
        return result

    fx = f(xyz_norm[..., 0])
    fy = f(xyz_norm[..., 1])
    fz = f(xyz_norm[..., 2])

    L = 116.0 * fy - 16.0
    a = 500.0 * (fx - fy)
    b = 200.0 * (fy - fz)

    return np.stack([L, a, b], axis=-1)


def find_closest_palette_color_lab(
    rgb_linear: np.ndarray,
    palette_linear: np.ndarray,
) -> np.ndarray:
    """Find closest palette color using LCH-weighted LAB distance.

    Optimized for batch operations (entire image at once). Uses numpy
    broadcasting to compute distances for all pixels simultaneously.

    Args:
        rgb_linear: Linear RGB values. Shape:
            - (3,) for single pixel
            - (height, width, 3) for entire image
        palette_linear: Palette colors in linear space. Shape: (num_colors, 3)

    Returns:
        Palette indices. Shape matches input without last dimension.
    """
    lab_pixels = rgb_to_lab(rgb_linear)
    lab_palette = rgb_to_lab(palette_linear)

    # Chroma of each palette color
    C_palette = np.sqrt(lab_palette[:, 1] ** 2 + lab_palette[:, 2] ** 2)

    # Chroma of each pixel
    C_pixels = np.sqrt(lab_pixels[..., 1] ** 2 + lab_pixels[..., 2] ** 2)

    # Broadcast differences: (..., 1, 3) - (num_colors, 3) -> (..., num_colors, 3)
    diff = lab_pixels[..., np.newaxis, :] - lab_palette[np.newaxis, :, :]
    dL = diff[..., 0]
    da = diff[..., 1]
    db = diff[..., 2]

    # LCH decomposition: da^2 + db^2 = dC^2 + dH^2
    dC = C_pixels[..., np.newaxis] - C_palette[np.newaxis, :]
    dH_sq = np.maximum(0.0, da ** 2 + db ** 2 - dC ** 2)

    # Weighted distance
    distances = (_WL * dL) ** 2 + (_WC * dC) ** 2 + _WH ** 2 * dH_sq

    return np.argmin(distances, axis=-1)  # type: ignore[no-any-return]


# =============================================================================
# Scalar Functions (for per-pixel error diffusion — no numpy overhead)
# =============================================================================

def _lab_f(t: float) -> float:
    """CIE LAB nonlinear function (scalar version)."""
    if t > _EPSILON:
        return float(t ** (1.0 / 3.0))
    return (_KAPPA * t + 16.0) / 116.0


def _rgb_to_lab_scalar(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert a single linear RGB pixel to LAB (scalar, no numpy)."""
    # RGB -> XYZ (inline matrix multiply)
    x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b
    y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b
    z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b

    # Normalize by D65 white point
    fx = _lab_f(x / _XN)
    fy = _lab_f(y / _YN)
    fz = _lab_f(z / _ZN)

    L = 116.0 * fy - 16.0
    a = 500.0 * (fx - fy)
    b_val = 200.0 * (fy - fz)
    return L, a, b_val


def _match_pixel_lch(
    r: float, g: float, b: float,
    palette_L: tuple[float, ...],
    palette_a: tuple[float, ...],
    palette_b: tuple[float, ...],
    palette_C: tuple[float, ...],
) -> int:
    """Find closest palette color for a single pixel using LCH distance.

    Pure Python (no numpy) for minimal per-call overhead in error diffusion.

    Args:
        r, g, b: Pixel in linear RGB [0, 1]
        palette_L, palette_a, palette_b: Pre-computed LAB components of palette
        palette_C: Pre-computed chroma of palette colors

    Returns:
        Index of closest palette color
    """
    pL, pa, pb = _rgb_to_lab_scalar(r, g, b)
    pC = math.sqrt(pa * pa + pb * pb)

    best_idx = 0
    best_dist = float("inf")

    for i in range(len(palette_L)):
        dL = pL - palette_L[i]
        da = pa - palette_a[i]
        db = pb - palette_b[i]
        dC = pC - palette_C[i]
        dH_sq = da * da + db * db - dC * dC
        if dH_sq < 0.0:
            dH_sq = 0.0

        dist = (_WL * dL) ** 2 + (_WC * dC) ** 2 + _WH * _WH * dH_sq
        if dist < best_dist:
            best_dist = dist
            best_idx = i

    return best_idx


def precompute_palette_lab(palette_linear: np.ndarray) -> tuple[
    tuple[float, ...], tuple[float, ...], tuple[float, ...], tuple[float, ...]
]:
    """Pre-compute palette LAB components for scalar matching.

    Call once before the error diffusion loop, then pass results to
    _match_pixel_lch() for each pixel.

    Args:
        palette_linear: Palette in linear RGB. Shape: (num_colors, 3)

    Returns:
        (palette_L, palette_a, palette_b, palette_C) as tuples of floats
    """
    lab = rgb_to_lab(palette_linear)
    L = tuple(float(x) for x in lab[:, 0])
    a = tuple(float(x) for x in lab[:, 1])
    b = tuple(float(x) for x in lab[:, 2])
    C = tuple(math.sqrt(float(ai) ** 2 + float(bi) ** 2) for ai, bi in zip(lab[:, 1], lab[:, 2]))
    return L, a, b, C

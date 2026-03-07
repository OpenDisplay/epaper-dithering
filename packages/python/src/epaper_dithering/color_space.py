"""Color space conversion utilities for e-paper dithering.

This module provides functions for converting between sRGB (gamma-encoded)
and linear RGB color spaces. Proper gamma correction is essential for
perceptually correct error diffusion dithering.

The functions implement the IEC 61966-2-1 sRGB standard with piecewise
transfer functions to avoid numerical issues with very dark values.
"""

from __future__ import annotations

import numpy as np


def srgb_to_linear(srgb: np.ndarray) -> np.ndarray:
    """Convert sRGB [0-255] to linear RGB [0.0-1.0].

    Implements the IEC 61966-2-1 sRGB standard inverse transfer function.
    This is essential for correct error diffusion dithering, as error
    calculations only make physical sense in linear light space.

    The function uses a piecewise approach:
    - For dark values (≤ 0.04045): linear section (value / 12.92)
    - For other values: power function ((value + 0.055) / 1.055) ^ 2.4

    Args:
        srgb: Array of sRGB values in range [0, 255]. Can be any shape.

    Returns:
        Array of linear RGB values in range [0.0, 1.0], same shape as input.

    Examples:
        >>> srgb = np.array([0, 128, 255], dtype=np.float32)
        >>> linear = srgb_to_linear(srgb)
        >>> linear[0]  # Black stays black
        0.0
        >>> linear[2]  # White stays white
        1.0
        >>> linear[1]  # Middle gray is much darker in sRGB
        0.2158...

    References:
        - IEC 61966-2-1:1999 standard
        - https://en.wikipedia.org/wiki/SRGB#Specification_of_the_transformation
    """
    # Normalize to [0, 1]
    normalized = srgb / 255.0

    # Apply inverse sRGB gamma (piecewise)
    # The threshold 0.04045 corresponds to linear value 0.0031308
    linear = np.where(
        normalized <= 0.04045,
        normalized / 12.92,  # Linear section for dark values
        np.power((normalized + 0.055) / 1.055, 2.4),  # Gamma section
    )

    return linear


def linear_to_srgb(linear: np.ndarray) -> np.ndarray:
    """Convert linear RGB [0.0-1.0] to sRGB [0-255].

    Implements the IEC 61966-2-1 sRGB standard transfer function.
    This converts from linear light values back to gamma-encoded sRGB
    for display or storage.

    The function uses a piecewise approach:
    - For dark values (≤ 0.0031308): linear section (value * 12.92)
    - For other values: power function (1.055 * value ^ (1/2.4) - 0.055)

    Args:
        linear: Array of linear RGB values in range [0.0, 1.0]. Can be any shape.

    Returns:
        Array of sRGB values in range [0, 255], same shape as input.
        Values are rounded (not truncated) to nearest integer.

    Examples:
        >>> linear = np.array([0.0, 0.5, 1.0], dtype=np.float32)
        >>> srgb = linear_to_srgb(linear)
        >>> srgb[0]  # Black stays black
        0
        >>> srgb[2]  # White stays white
        255
        >>> srgb[1]  # 50% linear light is bright in sRGB
        188

    Notes:
        - Uses np.round() instead of int() to avoid truncation bias
        - Output is uint8 for compatibility with PIL Image

    References:
        - IEC 61966-2-1:1999 standard
        - https://en.wikipedia.org/wiki/SRGB#Specification_of_the_transformation
    """
    # Apply sRGB gamma (piecewise)
    # The threshold 0.0031308 corresponds to sRGB value 0.04045
    normalized = np.where(
        linear <= 0.0031308,
        linear * 12.92,  # Linear section
        1.055 * np.power(linear, 1.0 / 2.4) - 0.055,  # Gamma section
    )

    # Convert to [0, 255] and round (not truncate!)
    # Rounding prevents systematic darkening bias from truncation
    return np.round(normalized * 255.0).astype(np.uint8)

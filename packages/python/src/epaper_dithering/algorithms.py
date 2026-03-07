"""Dithering algorithm implementations for e-paper displays."""

from __future__ import annotations

from dataclasses import dataclass
from typing import cast

import numpy as np
from PIL import Image

from .color_space import srgb_to_linear
from .color_space_lab import (
    _match_pixel_lch,
    find_closest_palette_color_lab,
    precompute_palette_lab,
)
from .palettes import ColorPalette, ColorScheme
from .tone_map import auto_compress_dynamic_range, compress_dynamic_range


@dataclass(frozen=True)
class ErrorDiffusionKernel:
    """Error diffusion kernel specification.

    Defines how quantization error is distributed to neighboring pixels.
    Each kernel is characterized by a name, divisor, and list of offset weights.

    Attributes:
        name: Human-readable kernel name
        divisor: Normalization divisor for weights
        offsets: List of (dx, dy, weight) tuples where:
            - dx: horizontal offset (positive = right)
            - dy: vertical offset (positive = down)
            - weight: error weight (will be divided by divisor)
            Note: (0, 0) is current pixel (already processed)
    """

    name: str
    divisor: float
    offsets: list[tuple[int, int, float]]


# Error diffusion kernel definitions
# Each kernel represents a different error distribution strategy

FLOYD_STEINBERG = ErrorDiffusionKernel(
    name="Floyd-Steinberg",
    divisor=16,
    offsets=[
        (1, 0, 7),   # Right: 7/16
        (-1, 1, 3),  # Down-left: 3/16
        (0, 1, 5),   # Down: 5/16
        (1, 1, 1),   # Down-right: 1/16
    ],
)

BURKES = ErrorDiffusionKernel(
    name="Burkes",
    divisor=32,
    offsets=[
        (1, 0, 8), (2, 0, 4),  # Current row
        (-2, 1, 2), (-1, 1, 4), (0, 1, 8), (1, 1, 4), (2, 1, 2),  # Next row
    ],
)

SIERRA = ErrorDiffusionKernel(
    name="Sierra",
    divisor=32,
    offsets=[
        (1, 0, 5), (2, 0, 3),  # Current row
        (-2, 1, 2), (-1, 1, 4), (0, 1, 5), (1, 1, 4), (2, 1, 2),  # Row +1
        (-1, 2, 2), (0, 2, 3), (1, 2, 2),  # Row +2
    ],
)

SIERRA_LITE = ErrorDiffusionKernel(
    name="Sierra Lite",
    divisor=4,
    offsets=[
        (1, 0, 2),   # Right: 2/4
        (-1, 1, 1),  # Down-left: 1/4
        (0, 1, 1),   # Down: 1/4
    ],
)

ATKINSON = ErrorDiffusionKernel(
    name="Atkinson",
    divisor=8,
    offsets=[
        (1, 0, 1), (2, 0, 1),  # Current row
        (-1, 1, 1), (0, 1, 1), (1, 1, 1),  # Row +1
        (0, 2, 1),  # Row +2
    ],
)

STUCKI = ErrorDiffusionKernel(
    name="Stucki",
    divisor=42,
    offsets=[
        (1, 0, 8), (2, 0, 4),  # Current row
        (-2, 1, 2), (-1, 1, 4), (0, 1, 8), (1, 1, 4), (2, 1, 2),  # Row +1
        (-2, 2, 1), (-1, 2, 2), (0, 2, 4), (1, 2, 2), (2, 2, 1),  # Row +2
    ],
)

JARVIS_JUDICE_NINKE = ErrorDiffusionKernel(
    name="Jarvis-Judice-Ninke",
    divisor=48,
    offsets=[
        (1, 0, 7), (2, 0, 5),  # Current row
        (-2, 1, 3), (-1, 1, 5), (0, 1, 7), (1, 1, 5), (2, 1, 3),  # Row +1
        (-2, 2, 1), (-1, 2, 3), (0, 2, 5), (1, 2, 3), (2, 2, 1),  # Row +2
    ],
)

# Bayer 4x4 matrix normalized to [-0.5, 0.5] (centered around 0)
_BAYER_4X4 = (
    np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
             dtype=np.float32)
    / 16.0
    - 0.5
)


def get_palette_colors(color_scheme: ColorScheme | ColorPalette) -> list[tuple[int, int, int]]:
    """Get RGB palette for color scheme or custom palette.

    Args:
        color_scheme: Display color scheme enum OR custom ColorPalette

    Returns:
        List of RGB tuples for palette (order matters for encoding)
    """
    if isinstance(color_scheme, ColorScheme):
        return list(color_scheme.palette.colors.values())
    return list(color_scheme.colors.values())


def error_diffusion_dither(
    image: Image.Image,
    color_scheme: ColorScheme | ColorPalette,
    kernel: ErrorDiffusionKernel,
    serpentine: bool = True,
    tone_compression: float | str = "auto",
) -> Image.Image:
    """Generic error diffusion dithering with any kernel.

    This function handles all aspects of error diffusion dithering:
    - Image preprocessing (RGBA → RGB on white, gamma correction)
    - Palette conversion to linear space
    - Serpentine or raster scanning
    - Error diffusion using provided kernel
    - Output assembly

    Working in linear RGB space ensures that error distribution is
    perceptually correct. Errors are calculated and propagated in
    linear light values, not gamma-encoded sRGB.

    Args:
        image: Input image (any PIL mode)
        color_scheme: Target color scheme
        kernel: Error diffusion kernel specification
        serpentine: Use serpentine scanning to reduce directional artifacts

    Returns:
        Dithered palette image in sRGB

    Notes:
        - RGBA images are composited on WHITE background (e-paper assumption)
        - Error buffer is unbounded during processing (can go negative or >1.0)
        - Clamping only occurs when reading pixels for quantization
        - Serpentine scanning alternates row direction to eliminate worm artifacts
    """
    # ===== Image Preprocessing =====
    # Convert to RGB, handling alpha channel properly
    if image.mode == "RGBA":
        # Composite on WHITE background (e-paper displays have white base)
        background = Image.new("RGB", image.size, (255, 255, 255))
        background.paste(image, mask=image.split()[3])  # Use alpha as mask
        image = background
    elif image.mode != "RGB":
        image = image.convert("RGB")

    # ===== Color Space Conversion =====
    # Convert from sRGB [0-255] to linear RGB [0.0-1.0]
    pixels_srgb = np.array(image, dtype=np.uint8)
    pixels_linear = srgb_to_linear(pixels_srgb.astype(np.float32))
    height, width = pixels_linear.shape[:2]

    # Convert palette to linear space
    palette_srgb = get_palette_colors(color_scheme)
    palette_linear = srgb_to_linear(np.array(palette_srgb, dtype=np.float32))

    # Compress dynamic range for measured palettes
    if isinstance(color_scheme, ColorPalette) and tone_compression != 0:
        if tone_compression == "auto":
            pixels_linear = auto_compress_dynamic_range(pixels_linear, palette_linear)
        else:
            pixels_linear = compress_dynamic_range(pixels_linear, palette_linear, cast(float, tone_compression))

    # Pre-compute palette LAB components for scalar per-pixel matching
    palette_L, palette_a, palette_b, palette_C = precompute_palette_lab(palette_linear)

    # Pre-extract palette linear RGB as Python floats (avoids numpy indexing in loop)
    palette_rgb = [
        (float(palette_linear[i, 0]), float(palette_linear[i, 1]), float(palette_linear[i, 2]))
        for i in range(len(palette_srgb))
    ]

    # Pre-normalize kernel weights (eliminates division per pixel)
    normalized_offsets = [(dx, dy, weight / kernel.divisor) for dx, dy, weight in kernel.offsets]

    # ===== Output Preparation =====
    output = Image.new("P", (width, height))
    output_pixels = np.zeros((height, width), dtype=np.uint8)

    # ===== Error Diffusion Loop =====
    for y in range(height):
        # Serpentine scanning: alternate direction each row
        if serpentine and y % 2 == 1:
            x_range = range(width - 1, -1, -1)  # Right to left
        else:
            x_range = range(width)  # Left to right

        for x in x_range:
            # Read current pixel as scalars (clamped to valid range)
            # Note: pixels_linear buffer can be outside [0, 1] due to error accumulation
            r = max(0.0, min(1.0, float(pixels_linear[y, x, 0])))
            g = max(0.0, min(1.0, float(pixels_linear[y, x, 1])))
            b = max(0.0, min(1.0, float(pixels_linear[y, x, 2])))

            # Find closest palette color using LCH-weighted LAB distance
            new_idx = _match_pixel_lch(r, g, b, palette_L, palette_a, palette_b, palette_C)

            # Store palette index
            output_pixels[y, x] = new_idx

            # Calculate quantization error per channel (in linear space)
            pr, pg, pb = palette_rgb[new_idx]
            err_r = r - pr
            err_g = g - pg
            err_b = b - pb

            # Distribute error using pre-normalized kernel weights
            for dx, dy, nw in normalized_offsets:
                # Flip horizontal offset if serpentine on odd row
                if serpentine and y % 2 == 1:
                    dx = -dx

                nx, ny = x + dx, y + dy

                # Check bounds and distribute error
                if 0 <= nx < width and 0 <= ny < height:
                    pixels_linear[ny, nx, 0] += err_r * nw
                    pixels_linear[ny, nx, 1] += err_g * nw
                    pixels_linear[ny, nx, 2] += err_b * nw

    # ===== Output Assembly =====
    output.putdata(output_pixels.flatten())

    # Set palette (in sRGB)
    flat_palette = [c for rgb in palette_srgb for c in rgb]
    output.putpalette(flat_palette)

    return output


# =============================================================================
# Individual Error Diffusion Algorithms (Thin Wrappers)
# =============================================================================
# Each function below is a thin wrapper around error_diffusion_dither()
# with a specific kernel. This eliminates code duplication while maintaining
# the original API.


def floyd_steinberg_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Floyd-Steinberg error diffusion dithering.

    Floyd-Steinberg kernel (divisor 16):
           X   7
       3   5   1

    Most popular error diffusion algorithm, good balance of quality and speed.

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, FLOYD_STEINBERG, serpentine, tone_compression)


def burkes_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Burkes error diffusion dithering.

    Burkes kernel (divisor 32):
             X   8   4
     2   4   8   4   2

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, BURKES, serpentine, tone_compression)


def sierra_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Sierra error diffusion dithering.

    Sierra kernel (divisor 32):
               X   5   3
       2   4   5   4   2
           2   3   2

    Sierra-2-4A variant, balanced quality and performance.

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, SIERRA, serpentine, tone_compression)


def sierra_lite_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Sierra Lite error diffusion dithering.

    Sierra Lite kernel (divisor 4):
         X   2
       1   1

    Fast, simple 3-neighbor algorithm.

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, SIERRA_LITE, serpentine, tone_compression)


def atkinson_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Atkinson error diffusion dithering.

    Atkinson kernel (divisor 8):
           X   1   1
       1   1   1
           1

    Designed for early Macintosh computers, produces distinct artistic look.

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, ATKINSON, serpentine, tone_compression)


def stucki_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Stucki error diffusion dithering.

    Stucki kernel (divisor 42):
               X   8   4
       2   4   8   4   2
       1   2   4   2   1

    High quality algorithm with wide error distribution.

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, STUCKI, serpentine, tone_compression)


def jarvis_judice_ninke_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette,
    serpentine: bool = True, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply Jarvis-Judice-Ninke error diffusion dithering.

    Jarvis-Judice-Ninke kernel (divisor 48):
               X   7   5
       3   5   7   5   3
       1   3   5   3   1

    Highest quality algorithm with symmetrical error distribution.

    Args:
        image: Input image
        color_scheme: Target color scheme
        serpentine: Use serpentine scanning (reduces artifacts)
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Dithered image
    """
    return error_diffusion_dither(image, color_scheme, JARVIS_JUDICE_NINKE, serpentine, tone_compression)


# =============================================================================
# Non-Error-Diffusion Algorithms
# =============================================================================


def direct_palette_map(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette, tone_compression: float | str = "auto",
) -> Image.Image:
    """Map image colors directly to palette without dithering.

    Args:
        image: Input image
        color_scheme: Target color scheme OR custom ColorPalette
        tone_compression: Dynamic range compression strength (0.0-1.0)

    Returns:
        Image with palette colors
    """
    # Handle alpha channel properly (composite on white)
    if image.mode == "RGBA":
        background = Image.new("RGB", image.size, (255, 255, 255))
        background.paste(image, mask=image.split()[3])
        image = background
    elif image.mode != "RGB":
        image = image.convert("RGB")

    palette_srgb = get_palette_colors(color_scheme)
    pixels_srgb = np.array(image, dtype=np.uint8)
    height, width = pixels_srgb.shape[:2]

    # ===== VECTORIZED PALETTE MAPPING =====

    # Convert to linear space for perceptual accuracy
    pixels_linear = srgb_to_linear(pixels_srgb.astype(np.float32))

    # Convert palette to linear space
    palette_linear = srgb_to_linear(np.array(palette_srgb, dtype=np.float32))

    # Compress dynamic range for measured palettes
    if isinstance(color_scheme, ColorPalette) and tone_compression != 0:
        if tone_compression == "auto":
            pixels_linear = auto_compress_dynamic_range(pixels_linear, palette_linear)
        else:
            pixels_linear = compress_dynamic_range(pixels_linear, palette_linear, cast(float, tone_compression))

    # Find closest palette color for ALL pixels at once using LAB
    output_pixels = find_closest_palette_color_lab(pixels_linear, palette_linear)

    # ===== Output Assembly =====
    output = Image.new("P", (width, height))
    output.putdata(output_pixels.flatten())
    flat_palette = [c for rgb in palette_srgb for c in rgb]
    output.putpalette(flat_palette)

    return output


def ordered_dither(
    image: Image.Image, color_scheme: ColorScheme | ColorPalette, tone_compression: float | str = "auto",
) -> Image.Image:
    """Apply ordered (Bayer) dithering with full vectorization.

    Uses a normalized 4x4 Bayer matrix to add spatially-distributed noise
    before quantization. Unlike error diffusion, ordered dithering does not
    propagate errors between pixels, making it ideal for vectorization.

    This implementation works in linear RGB space with proper gamma correction
    and uses small centered threshold offsets (not the broken 0-240 bias from
    the previous version).

    Args:
        image: Input image (any PIL mode)
        color_scheme: Target color scheme

    Returns:
        Dithered palette image

    Notes:
        - Bayer matrix normalized to [-0.5, 0.5] centered around 0
        - RGBA images composited on white background
        - Uses perceptual color distance (ITU-R BT.601 luma weights)
        - Works in linear RGB space for correct quantization
    """
    # Bayer 4x4 matrix normalized to [-0.5, 0.5] (centered around 0)
    bayer_matrix = (
        np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
                 dtype=np.float32)
        / 16.0
        - 0.5
    )

    # ===== Image Preprocessing =====
    if image.mode == "RGBA":
        background = Image.new("RGB", image.size, (255, 255, 255))
        background.paste(image, mask=image.split()[3])
        image = background
    elif image.mode != "RGB":
        image = image.convert("RGB")

    # ===== Color Space Conversion =====
    pixels_srgb = np.array(image, dtype=np.uint8)
    pixels_linear = srgb_to_linear(pixels_srgb.astype(np.float32))
    height, width = pixels_linear.shape[:2]

    # Convert palette to linear space
    palette_srgb = get_palette_colors(color_scheme)
    palette_linear = srgb_to_linear(np.array(palette_srgb, dtype=np.float32))

    # Compress dynamic range for measured palettes
    if isinstance(color_scheme, ColorPalette) and tone_compression != 0:
        if tone_compression == "auto":
            pixels_linear = auto_compress_dynamic_range(pixels_linear, palette_linear)
        else:
            pixels_linear = compress_dynamic_range(pixels_linear, palette_linear, cast(float, tone_compression))

    # ===== VECTORIZED ORDERED DITHERING =====

    # Create threshold matrix for entire image using broadcasting
    y_indices = np.arange(height)[:, np.newaxis] % 4  # Shape: (height, 1)
    x_indices = np.arange(width)[np.newaxis, :] % 4   # Shape: (1, width)
    threshold_matrix = bayer_matrix[y_indices, x_indices]  # Shape: (height, width)

    # Add threshold to all pixels at once
    dithered_pixels = pixels_linear + threshold_matrix[:, :, np.newaxis]
    dithered_pixels = np.clip(dithered_pixels, 0.0, 1.0)

    # Find closest palette color for ALL pixels at once using LAB
    output_pixels = find_closest_palette_color_lab(dithered_pixels, palette_linear)

    # ===== Output Assembly =====
    output = Image.new("P", (width, height))
    output.putdata(output_pixels.flatten())
    flat_palette = [c for rgb in palette_srgb for c in rgb]
    output.putpalette(flat_palette)

    return output

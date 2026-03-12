import { SRGB_TO_LINEAR_LUT } from './color_space';
import { precomputePaletteLab, matchPixelLch } from './color_space_lab';
import { autoCompressDynamicRange } from './tone_map';
import type { RGB, ImageBuffer, PaletteImageBuffer, ColorPalette } from './types';
import { ColorScheme, getPalette } from './palettes';

// Bayer 4×4 matrix normalized to [-0.5, 0.5] — matches Python's _BAYER_4X4
const BAYER_4X4: Float32Array = new Float32Array([
   0,  8,  2, 10,
  12,  4, 14,  6,
   3, 11,  1,  9,
  15,  7, 13,  5,
].map(v => v / 16.0 - 0.5));

interface ErrorKernel {
  dx: number;
  dy: number;
  weight: number; // pre-normalized (already divided by divisor)
}

// =============================================================================
// Internal helpers
// =============================================================================

function resolvePalette(scheme: ColorScheme | ColorPalette): ColorPalette {
  return typeof scheme === 'number' ? getPalette(scheme) : scheme;
}

/** Convert ColorPalette sRGB colors to linear [0,1] tuples. */
function paletteToLinear(palette: ColorPalette): {
  paletteRgb: RGB[];
  paletteLinear: Array<[number, number, number]>;
} {
  const paletteRgb = Object.values(palette.colors);
  const paletteLinear = paletteRgb.map(c => [
    SRGB_TO_LINEAR_LUT[c.r],
    SRGB_TO_LINEAR_LUT[c.g],
    SRGB_TO_LINEAR_LUT[c.b],
  ] as [number, number, number]);
  return { paletteRgb, paletteLinear };
}

/**
 * Build a Float32Array linear pixel buffer from RGBA input, compositing on white.
 * Alpha compositing in linear space: channel = LUT[srgb] * alpha + (1 - alpha)
 * (white = 1.0 in linear; compositing is done in one pass, no separate alpha step)
 */
function buildLinearBuffer(image: ImageBuffer): Float32Array {
  const n = image.width * image.height;
  const pixels = new Float32Array(n * 3);
  const { data } = image;

  for (let i = 0; i < n; i++) {
    const base4 = i * 4;
    const base3 = i * 3;
    const alpha = data[base4 + 3] / 255.0;
    const inv   = 1.0 - alpha;
    pixels[base3]     = SRGB_TO_LINEAR_LUT[data[base4]]     * alpha + inv;
    pixels[base3 + 1] = SRGB_TO_LINEAR_LUT[data[base4 + 1]] * alpha + inv;
    pixels[base3 + 2] = SRGB_TO_LINEAR_LUT[data[base4 + 2]] * alpha + inv;
  }

  return pixels;
}

// =============================================================================
// Error diffusion
// =============================================================================

function errorDiffusionDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  kernel: ErrorKernel[],
  serpentine: boolean,
): PaletteImageBuffer {
  const { width, height } = image;
  const palette = resolvePalette(scheme);
  const { paletteRgb, paletteLinear } = paletteToLinear(palette);
  const numColors = paletteRgb.length;

  // Build linear pixel buffer with RGBA compositing
  const pixels = buildLinearBuffer(image);

  // Tone compression for measured palettes only
  if (typeof scheme !== 'number') {
    autoCompressDynamicRange(pixels, width, height, paletteLinear);
  }

  // Pre-compute palette LAB for the hot loop
  const { L: palL, a: palA, b: palB, C: palC } = precomputePaletteLab(paletteLinear);

  // Palette linear RGB as flat typed arrays for error computation
  const palR = new Float64Array(numColors);
  const palG = new Float64Array(numColors);
  const palBl = new Float64Array(numColors);
  for (let i = 0; i < numColors; i++) {
    palR[i]  = paletteLinear[i][0];
    palG[i]  = paletteLinear[i][1];
    palBl[i] = paletteLinear[i][2];
  }

  const indices = new Uint8Array(width * height);

  for (let y = 0; y < height; y++) {
    // Serpentine: alternate row direction to reduce directional artifacts
    const leftToRight = !serpentine || (y % 2 === 0);
    const xStart = leftToRight ? 0 : width - 1;
    const xEnd   = leftToRight ? width : -1;
    const xStep  = leftToRight ? 1 : -1;

    for (let x = xStart; x !== xEnd; x += xStep) {
      const pixIdx = (y * width + x) * 3;

      // Clamp accumulated error to valid range before matching
      const r = Math.max(0.0, Math.min(1.0, pixels[pixIdx]));
      const g = Math.max(0.0, Math.min(1.0, pixels[pixIdx + 1]));
      const b = Math.max(0.0, Math.min(1.0, pixels[pixIdx + 2]));

      // LCH-weighted LAB color matching
      const newIdx = matchPixelLch(r, g, b, palL, palA, palB, palC);
      indices[y * width + x] = newIdx;

      // Quantization error in linear space
      const errR = r - palR[newIdx];
      const errG = g - palG[newIdx];
      const errB = b - palBl[newIdx];

      // Distribute error to neighbors
      for (let k = 0; k < kernel.length; k++) {
        const kEntry = kernel[k];
        // Flip horizontal offset on right-to-left rows (serpentine)
        const nx = x + (leftToRight ? kEntry.dx : -kEntry.dx);
        const ny = y + kEntry.dy;

        if (nx >= 0 && nx < width && ny >= 0 && ny < height) {
          const nIdx = (ny * width + nx) * 3;
          pixels[nIdx]     += errR * kEntry.weight;
          pixels[nIdx + 1] += errG * kEntry.weight;
          pixels[nIdx + 2] += errB * kEntry.weight;
        }
      }
    }
  }

  return { width, height, indices, palette: paletteRgb };
}

// =============================================================================
// Non-error-diffusion algorithms
// =============================================================================

export function directPaletteMap(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
): PaletteImageBuffer {
  const { width, height } = image;
  const palette = resolvePalette(scheme);
  const { paletteRgb, paletteLinear } = paletteToLinear(palette);

  const pixels = buildLinearBuffer(image);

  if (typeof scheme !== 'number') {
    autoCompressDynamicRange(pixels, width, height, paletteLinear);
  }

  const { L: palL, a: palA, b: palB, C: palC } = precomputePaletteLab(paletteLinear);
  const indices = new Uint8Array(width * height);
  const n = width * height;

  for (let i = 0; i < n; i++) {
    const base = i * 3;
    const r = Math.max(0.0, Math.min(1.0, pixels[base]));
    const g = Math.max(0.0, Math.min(1.0, pixels[base + 1]));
    const b = Math.max(0.0, Math.min(1.0, pixels[base + 2]));
    indices[i] = matchPixelLch(r, g, b, palL, palA, palB, palC);
  }

  return { width, height, indices, palette: paletteRgb };
}

export function orderedDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
): PaletteImageBuffer {
  const { width, height } = image;
  const palette = resolvePalette(scheme);
  const { paletteRgb, paletteLinear } = paletteToLinear(palette);

  const pixels = buildLinearBuffer(image);

  if (typeof scheme !== 'number') {
    autoCompressDynamicRange(pixels, width, height, paletteLinear);
  }

  const { L: palL, a: palA, b: palB, C: palC } = precomputePaletteLab(paletteLinear);
  const indices = new Uint8Array(width * height);

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const base = (y * width + x) * 3;
      const threshold = BAYER_4X4[(y % 4) * 4 + (x % 4)];

      const r = Math.max(0.0, Math.min(1.0, pixels[base]     + threshold));
      const g = Math.max(0.0, Math.min(1.0, pixels[base + 1] + threshold));
      const b = Math.max(0.0, Math.min(1.0, pixels[base + 2] + threshold));

      indices[y * width + x] = matchPixelLch(r, g, b, palL, palA, palB, palC);
    }
  }

  return { width, height, indices, palette: paletteRgb };
}

// =============================================================================
// Error diffusion algorithm wrappers
// Kernel weights are pre-normalized (divided by divisor) to eliminate per-pixel division.
// =============================================================================

export function floydSteinbergDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight: 7 / 16 },
    { dx: -1, dy: 1, weight: 3 / 16 },
    { dx:  0, dy: 1, weight: 5 / 16 },
    { dx:  1, dy: 1, weight: 1 / 16 },
  ], serpentine);
}

export function burkesDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  // Correct Burkes kernel: divisor 32 (not 200)
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight: 8 / 32 },
    { dx:  2, dy: 0, weight: 4 / 32 },
    { dx: -2, dy: 1, weight: 2 / 32 },
    { dx: -1, dy: 1, weight: 4 / 32 },
    { dx:  0, dy: 1, weight: 8 / 32 },
    { dx:  1, dy: 1, weight: 4 / 32 },
    { dx:  2, dy: 1, weight: 2 / 32 },
  ], serpentine);
}

export function sierraDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight: 5 / 32 },
    { dx:  2, dy: 0, weight: 3 / 32 },
    { dx: -2, dy: 1, weight: 2 / 32 },
    { dx: -1, dy: 1, weight: 4 / 32 },
    { dx:  0, dy: 1, weight: 5 / 32 },
    { dx:  1, dy: 1, weight: 4 / 32 },
    { dx:  2, dy: 1, weight: 2 / 32 },
    { dx: -1, dy: 2, weight: 2 / 32 },
    { dx:  0, dy: 2, weight: 3 / 32 },
    { dx:  1, dy: 2, weight: 2 / 32 },
  ], serpentine);
}

export function sierraLiteDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight: 2 / 4 },
    { dx: -1, dy: 1, weight: 1 / 4 },
    { dx:  0, dy: 1, weight: 1 / 4 },
  ], serpentine);
}

export function atkinsonDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight: 1 / 8 },
    { dx:  2, dy: 0, weight: 1 / 8 },
    { dx: -1, dy: 1, weight: 1 / 8 },
    { dx:  0, dy: 1, weight: 1 / 8 },
    { dx:  1, dy: 1, weight: 1 / 8 },
    { dx:  0, dy: 2, weight: 1 / 8 },
  ], serpentine);
}

export function stuckiDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight:  8 / 42 },
    { dx:  2, dy: 0, weight:  4 / 42 },
    { dx: -2, dy: 1, weight:  2 / 42 },
    { dx: -1, dy: 1, weight:  4 / 42 },
    { dx:  0, dy: 1, weight:  8 / 42 },
    { dx:  1, dy: 1, weight:  4 / 42 },
    { dx:  2, dy: 1, weight:  2 / 42 },
    { dx: -2, dy: 2, weight:  1 / 42 },
    { dx: -1, dy: 2, weight:  2 / 42 },
    { dx:  0, dy: 2, weight:  4 / 42 },
    { dx:  1, dy: 2, weight:  2 / 42 },
    { dx:  2, dy: 2, weight:  1 / 42 },
  ], serpentine);
}

export function jarvisJudiceNinkeDither(
  image: ImageBuffer,
  scheme: ColorScheme | ColorPalette,
  serpentine: boolean = true,
): PaletteImageBuffer {
  return errorDiffusionDither(image, scheme, [
    { dx:  1, dy: 0, weight:  7 / 48 },
    { dx:  2, dy: 0, weight:  5 / 48 },
    { dx: -2, dy: 1, weight:  3 / 48 },
    { dx: -1, dy: 1, weight:  5 / 48 },
    { dx:  0, dy: 1, weight:  7 / 48 },
    { dx:  1, dy: 1, weight:  5 / 48 },
    { dx:  2, dy: 1, weight:  3 / 48 },
    { dx: -2, dy: 2, weight:  1 / 48 },
    { dx: -1, dy: 2, weight:  3 / 48 },
    { dx:  0, dy: 2, weight:  5 / 48 },
    { dx:  1, dy: 2, weight:  3 / 48 },
    { dx:  2, dy: 2, weight:  1 / 48 },
  ], serpentine);
}

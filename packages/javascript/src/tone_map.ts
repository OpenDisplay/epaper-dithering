/**
 * Dynamic range compression for measured e-paper palettes.
 *
 * Real e-paper displays reflect much less light than pure sRGB colors suggest.
 * Measured white is ~185,202,205 vs pure 255,255,255 — a significant dynamic
 * range reduction. Compressing the image into the display's actual range before
 * dithering reduces large quantization errors at highlights and shadows.
 *
 * Only applied when using measured ColorPalette instances, never for ColorScheme enums.
 */

// ITU-R BT.709 / sRGB luminance coefficients
const LUM_R = 0.2126729;
const LUM_G = 0.7151522;
const LUM_B = 0.0721750;

/**
 * Compress image dynamic range to match display capabilities.
 *
 * Remaps pixel luminance from [0, 1] to the display's actual [black_Y, white_Y],
 * scaling RGB channels proportionally to preserve hue.
 *
 * @param pixels - Linear RGB flat buffer [r,g,b, r,g,b, ...], modified in-place
 * @param width - Image width
 * @param height - Image height
 * @param paletteLinear - Palette in linear RGB: index 0 = black, index 1 = white
 * @param strength - Blend factor: 0.0 = no compression, 1.0 = full compression
 */
export function compressDynamicRange(
  pixels: Float32Array,
  width: number,
  height: number,
  paletteLinear: Array<[number, number, number]>,
  strength: number = 1.0,
): void {
  if (strength <= 0.0) return;

  const [br, bg, bb] = paletteLinear[0];
  const [wr, wg, wb] = paletteLinear[1];
  const blackY = LUM_R * br + LUM_G * bg + LUM_B * bb;
  const whiteY = LUM_R * wr + LUM_G * wg + LUM_B * wb;
  const displayRange = whiteY - blackY;
  if (displayRange <= 0) return;

  const n = width * height;
  const blackLevel = blackY * strength;

  for (let i = 0; i < n; i++) {
    const base = i * 3;
    const r = pixels[base];
    const g = pixels[base + 1];
    const b = pixels[base + 2];
    const Y = LUM_R * r + LUM_G * g + LUM_B * b;

    if (Y > 1e-6) {
      const compressedY = blackY + Y * displayRange;
      const targetY = strength < 1.0 ? Y + strength * (compressedY - Y) : compressedY;
      const scale = targetY / Y;
      pixels[base]     = Math.max(0, Math.min(1, r * scale));
      pixels[base + 1] = Math.max(0, Math.min(1, g * scale));
      pixels[base + 2] = Math.max(0, Math.min(1, b * scale));
    } else {
      pixels[base]     = blackLevel;
      pixels[base + 1] = blackLevel;
      pixels[base + 2] = blackLevel;
    }
  }
}

/**
 * Auto-levels dynamic range compression fitted to the image's actual content.
 *
 * Uses 2nd/98th percentile luminance to ignore outliers, maximizing contrast
 * within the display's capabilities. Falls back to full compressDynamicRange
 * for uniform images.
 *
 * @param pixels - Linear RGB flat buffer, modified in-place
 * @param width - Image width
 * @param height - Image height
 * @param paletteLinear - Palette in linear RGB: index 0 = black, index 1 = white
 */
export function autoCompressDynamicRange(
  pixels: Float32Array,
  width: number,
  height: number,
  paletteLinear: Array<[number, number, number]>,
): void {
  const [br, bg, bb] = paletteLinear[0];
  const [wr, wg, wb] = paletteLinear[1];
  const blackY = LUM_R * br + LUM_G * bg + LUM_B * bb;
  const whiteY = LUM_R * wr + LUM_G * wg + LUM_B * wb;
  const displayRange = whiteY - blackY;
  if (displayRange <= 0) return;

  const n = width * height;

  // First pass: compute per-pixel luminance
  const luminances = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const base = i * 3;
    luminances[i] = LUM_R * pixels[base] + LUM_G * pixels[base + 1] + LUM_B * pixels[base + 2];
  }

  // Sort to find percentiles
  luminances.sort();
  const pLow  = luminances[Math.floor(n * 0.02)];
  const pHigh = luminances[Math.floor(n * 0.98)];
  const imageRange = pHigh - pLow;

  if (imageRange < 1e-6) {
    // Uniform image: fall back to standard linear compression
    compressDynamicRange(pixels, width, height, paletteLinear, 1.0);
    return;
  }

  // Second pass: remap [pLow, pHigh] → [blackY, whiteY]
  for (let i = 0; i < n; i++) {
    const base = i * 3;
    const r = pixels[base];
    const g = pixels[base + 1];
    const b = pixels[base + 2];
    const Y = LUM_R * r + LUM_G * g + LUM_B * b;

    if (Y > 1e-6) {
      const normalizedY = (Y - pLow) / imageRange;
      const targetY = blackY + normalizedY * displayRange;
      const scale = targetY / Y;
      pixels[base]     = Math.max(0, Math.min(1, r * scale));
      pixels[base + 1] = Math.max(0, Math.min(1, g * scale));
      pixels[base + 2] = Math.max(0, Math.min(1, b * scale));
    } else {
      pixels[base]     = blackY;
      pixels[base + 1] = blackY;
      pixels[base + 2] = blackY;
    }
  }
}

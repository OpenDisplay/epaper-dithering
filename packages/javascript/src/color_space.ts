/**
 * sRGB ↔ linear RGB conversion (IEC 61966-2-1).
 *
 * Key optimization: SRGB_TO_LINEAR_LUT is a 256-entry lookup table built once
 * at module load. Since input pixels are always integers 0–255 (Uint8ClampedArray),
 * conversion is a single array index — no Math.pow per pixel.
 */

// 256-entry LUT: index = sRGB byte value, value = linear [0, 1]
export const SRGB_TO_LINEAR_LUT: Float64Array = (() => {
  const lut = new Float64Array(256);
  for (let i = 0; i < 256; i++) {
    const s = i / 255.0;
    lut[i] = s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  }
  return lut;
})();

/**
 * Convert a single linear [0, 1] value to a rounded sRGB byte [0, 255].
 * Only needed for palette color encoding — not called per pixel.
 */
export function linearToSrgbByte(v: number): number {
  const s = v <= 0.0031308 ? v * 12.92 : 1.055 * Math.pow(v, 1.0 / 2.4) - 0.055;
  return Math.max(0, Math.min(255, Math.round(s * 255)));
}

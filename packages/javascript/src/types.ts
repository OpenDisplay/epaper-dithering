/**
 * RGB color representation
 */
export interface RGB {
  r: number; // 0-255
  g: number; // 0-255
  b: number; // 0-255
}

/**
 * Image buffer in RGBA format
 * Compatible with Canvas ImageData and Node.js image libraries
 */
export interface ImageBuffer {
  width: number;
  height: number;
  data: Uint8ClampedArray; // RGBA: [r, g, b, a, r, g, b, a, ...]
}

/**
 * Palette-indexed image output
 */
export interface PaletteImageBuffer {
  width: number;
  height: number;
  indices: Uint8Array; // Palette index per pixel
  palette: RGB[]; // Available colors
}

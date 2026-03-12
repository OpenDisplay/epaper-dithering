import type { ImageBuffer, RGB } from '../../src';

/** Create solid color test image with full opacity */
export function createTestImage(
  width: number,
  height: number,
  color: RGB,
): ImageBuffer {
  const data = new Uint8ClampedArray(width * height * 4);

  for (let i = 0; i < width * height; i++) {
    data[i * 4]     = color.r;
    data[i * 4 + 1] = color.g;
    data[i * 4 + 2] = color.b;
    data[i * 4 + 3] = 255;
  }

  return { width, height, data };
}

/** Create horizontal gradient test image (grayscale, left=black, right=white) */
export function createGradient(width: number, height: number): ImageBuffer {
  const data = new Uint8ClampedArray(width * height * 4);

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const value = Math.floor((x / width) * 255);
      const idx = (y * width + x) * 4;
      data[idx]     = value;
      data[idx + 1] = value;
      data[idx + 2] = value;
      data[idx + 3] = 255;
    }
  }

  return { width, height, data };
}

/** Create test image with specified alpha (for compositing tests) */
export function createTransparentTestImage(
  width: number,
  height: number,
  color: RGB,
  alpha: number,
): ImageBuffer {
  const data = new Uint8ClampedArray(width * height * 4);

  for (let i = 0; i < width * height; i++) {
    data[i * 4]     = color.r;
    data[i * 4 + 1] = color.g;
    data[i * 4 + 2] = color.b;
    data[i * 4 + 3] = alpha;
  }

  return { width, height, data };
}

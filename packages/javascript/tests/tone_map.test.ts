import { describe, it, expect } from 'vitest';
import { compressDynamicRange, autoCompressDynamicRange } from '../src/tone_map';

const LUM_R = 0.2126729;
const LUM_G = 0.7151522;
const LUM_B = 0.0721750;

function luminance(r: number, g: number, b: number): number {
  return LUM_R * r + LUM_G * g + LUM_B * b;
}

// Measured SPECTRA palette in linear (approximate)
const SPECTRA_LINEAR: Array<[number, number, number]> = [
  [0.008, 0.002, 0.017], // black
  [0.498, 0.593, 0.617], // white
];

describe('compressDynamicRange', () => {
  it('strength=0 leaves buffer unchanged', () => {
    const pixels = new Float32Array([0.5, 0.5, 0.5, 0.2, 0.8, 0.1]);
    const original = new Float32Array(pixels);
    compressDynamicRange(pixels, 2, 1, SPECTRA_LINEAR, 0.0);
    for (let i = 0; i < pixels.length; i++) {
      expect(pixels[i]).toBeCloseTo(original[i], 10);
    }
  });

  it('output luminance stays within [black_Y, white_Y]', () => {
    const blackY = luminance(...SPECTRA_LINEAR[0]);
    const whiteY = luminance(...SPECTRA_LINEAR[1]);

    const pixels = new Float32Array(3 * 9);
    // Cover full range: dark, mid, bright
    for (let i = 0; i < 3; i++) {
      const v = i * 0.4;
      pixels[i * 3]     = v;
      pixels[i * 3 + 1] = v;
      pixels[i * 3 + 2] = v;
    }

    compressDynamicRange(pixels, 3, 1, SPECTRA_LINEAR, 1.0);

    for (let i = 0; i < 3; i++) {
      const base = i * 3;
      const Y = luminance(pixels[base], pixels[base + 1], pixels[base + 2]);
      expect(Y).toBeGreaterThanOrEqual(blackY - 1e-5);
      expect(Y).toBeLessThanOrEqual(whiteY + 1e-5);
    }
  });

  it('pure black/white palette leaves full-range images essentially unchanged', () => {
    const pureLinear: Array<[number, number, number]> = [[0, 0, 0], [1, 1, 1]];
    const pixels = new Float32Array([0.3, 0.3, 0.3]);
    const before = pixels[0];
    compressDynamicRange(pixels, 1, 1, pureLinear, 1.0);
    expect(pixels[0]).toBeCloseTo(before, 5);
  });
});

describe('autoCompressDynamicRange', () => {
  it('does not throw on uniform image', () => {
    const pixels = new Float32Array(3 * 4).fill(0.5);
    expect(() => autoCompressDynamicRange(pixels, 2, 2, SPECTRA_LINEAR)).not.toThrow();
  });

  it('output values are in [0, 1]', () => {
    const pixels = new Float32Array(3 * 100);
    for (let i = 0; i < 100; i++) {
      const v = i / 99;
      pixels[i * 3]     = v;
      pixels[i * 3 + 1] = v;
      pixels[i * 3 + 2] = v;
    }
    autoCompressDynamicRange(pixels, 100, 1, SPECTRA_LINEAR);
    for (let i = 0; i < pixels.length; i++) {
      expect(pixels[i]).toBeGreaterThanOrEqual(-1e-5);
      expect(pixels[i]).toBeLessThanOrEqual(1 + 1e-5);
    }
  });

  it('narrow-range image produces wider luminance spread than input', () => {
    // Image only uses midtones [0.4, 0.6]
    const n = 50;
    const pixels = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) {
      const v = 0.4 + (i / (n - 1)) * 0.2;
      pixels[i * 3]     = v;
      pixels[i * 3 + 1] = v;
      pixels[i * 3 + 2] = v;
    }
    const blackY = luminance(...SPECTRA_LINEAR[0]);
    const whiteY = luminance(...SPECTRA_LINEAR[1]);

    autoCompressDynamicRange(pixels, n, 1, SPECTRA_LINEAR);

    const outMin = Math.min(...Array.from({ length: n }, (_, i) =>
      luminance(pixels[i * 3], pixels[i * 3 + 1], pixels[i * 3 + 2])
    ));
    const outMax = Math.max(...Array.from({ length: n }, (_, i) =>
      luminance(pixels[i * 3], pixels[i * 3 + 1], pixels[i * 3 + 2])
    ));

    // Auto mode should stretch midtones to fill [blackY, whiteY]
    expect(outMin).toBeCloseTo(blackY, 2);
    expect(outMax).toBeCloseTo(whiteY, 2);
  });
});

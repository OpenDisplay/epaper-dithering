import { describe, it, expect } from 'vitest';
import {
  ditherImage,
  DitherMode,
  ColorScheme,
  getPalette,
  getColorCount,
  fromValue,
  SPECTRA_7_3_6COLOR,
} from '../src';
import { createTestImage, createGradient, createTransparentTestImage } from './fixtures';

describe('Dithering Algorithms', () => {
  it.each(Object.values(DitherMode).filter((v) => typeof v === 'number'))(
    'produces valid output for mode %s',
    (mode) => {
      const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });
      const result = ditherImage(image, ColorScheme.BWR, mode as DitherMode);

      expect(result.width).toBe(10);
      expect(result.height).toBe(10);
      expect(result.indices.length).toBe(100);
      expect(result.palette.length).toBe(3);
    }
  );

  it.each(Object.values(ColorScheme).filter((v) => typeof v === 'number'))(
    'works with color scheme %s',
    (scheme) => {
      const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });
      const result = ditherImage(image, scheme as ColorScheme, DitherMode.BURKES);

      expect(result.palette.length).toBeGreaterThan(0);
    }
  );

  it('handles RGBA input correctly', () => {
    const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });
    const result = ditherImage(image, ColorScheme.BWR, DitherMode.BURKES);

    expect(result).toBeDefined();
    expect(result.width).toBe(10);
    expect(result.height).toBe(10);
  });

  it('produces different output for different algorithms', () => {
    const image = createGradient(100, 100);

    const burkes = ditherImage(image, ColorScheme.MONO, DitherMode.BURKES);
    const floydSteinberg = ditherImage(image, ColorScheme.MONO, DitherMode.FLOYD_STEINBERG);

    let differences = 0;
    for (let i = 0; i < burkes.indices.length; i++) {
      if (burkes.indices[i] !== floydSteinberg.indices[i]) differences++;
    }

    expect(differences).toBeGreaterThan(0);
  });

  it('default mode is BURKES', () => {
    const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });

    const withDefault = ditherImage(image, ColorScheme.BWR);
    const withBurkes = ditherImage(image, ColorScheme.BWR, DitherMode.BURKES);

    expect(withDefault.indices).toEqual(withBurkes.indices);
  });

  it('serpentine=true and serpentine=false produce different results on gradient', () => {
    const image = createGradient(50, 50);

    const withSerpentine    = ditherImage(image, ColorScheme.MONO, DitherMode.FLOYD_STEINBERG, true);
    const withoutSerpentine = ditherImage(image, ColorScheme.MONO, DitherMode.FLOYD_STEINBERG, false);

    let differences = 0;
    for (let i = 0; i < withSerpentine.indices.length; i++) {
      if (withSerpentine.indices[i] !== withoutSerpentine.indices[i]) differences++;
    }
    expect(differences).toBeGreaterThan(0);
  });

  it('alpha compositing: fully transparent red is treated as white', () => {
    // Fully transparent red (alpha=0) should composite to white
    const image = createTransparentTestImage(4, 4, { r: 255, g: 0, b: 0 }, 0);
    const result = ditherImage(image, ColorScheme.MONO, DitherMode.NONE);

    // All pixels should be white (index 1 in MONO: black=0, white=1)
    for (let i = 0; i < result.indices.length; i++) {
      expect(result.indices[i]).toBe(1);
    }
  });

  it('alpha compositing: alpha value affects the result', () => {
    // Opaque black → composites as black → maps to black (index 0)
    const opaque = createTransparentTestImage(4, 4, { r: 0, g: 0, b: 0 }, 255);
    // Very low alpha black → composites nearly to white → maps to white (index 1)
    const nearlyTransparent = createTransparentTestImage(4, 4, { r: 0, g: 0, b: 0 }, 10);

    const resultOpaque = ditherImage(opaque, ColorScheme.MONO, DitherMode.NONE);
    const resultNearly = ditherImage(nearlyTransparent, ColorScheme.MONO, DitherMode.NONE);

    expect(resultOpaque.indices[0]).toBe(0);    // black
    expect(resultNearly.indices[0]).toBe(1);    // white
  });

  it('accepts measured ColorPalette and returns correct palette length', () => {
    const image = createTestImage(10, 10, { r: 100, g: 150, b: 80 });
    const result = ditherImage(image, SPECTRA_7_3_6COLOR);

    expect(result.palette.length).toBe(6);
    expect(result.indices.length).toBe(100);
  });

  it('measured palette palette indices are within range', () => {
    const image = createGradient(20, 20);
    const result = ditherImage(image, SPECTRA_7_3_6COLOR, DitherMode.FLOYD_STEINBERG);

    for (let i = 0; i < result.indices.length; i++) {
      expect(result.indices[i]).toBeGreaterThanOrEqual(0);
      expect(result.indices[i]).toBeLessThan(6);
    }
  });
});

describe('ColorScheme', () => {
  it('has correct color counts', () => {
    expect(getColorCount(ColorScheme.MONO)).toBe(2);
    expect(getColorCount(ColorScheme.BWR)).toBe(3);
    expect(getColorCount(ColorScheme.BWY)).toBe(3);
    expect(getColorCount(ColorScheme.BWRY)).toBe(4);
    expect(getColorCount(ColorScheme.BWGBRY)).toBe(6);
    expect(getColorCount(ColorScheme.GRAYSCALE_4)).toBe(4);
    expect(getColorCount(ColorScheme.GRAYSCALE_8)).toBe(8);
    expect(getColorCount(ColorScheme.GRAYSCALE_16)).toBe(16);
  });

  it('fromValue works correctly for all schemes', () => {
    expect(fromValue(0)).toBe(ColorScheme.MONO);
    expect(fromValue(1)).toBe(ColorScheme.BWR);
    expect(fromValue(5)).toBe(ColorScheme.GRAYSCALE_4);
    expect(fromValue(6)).toBe(ColorScheme.GRAYSCALE_8);
    expect(fromValue(7)).toBe(ColorScheme.GRAYSCALE_16);
  });

  it('fromValue throws for out-of-range values', () => {
    expect(() => fromValue(8)).toThrow();
    expect(() => fromValue(99)).toThrow();
    expect(() => fromValue(-1)).toThrow();
  });

  it('palette colors are valid RGB', () => {
    for (const scheme of Object.values(ColorScheme).filter((v) => typeof v === 'number')) {
      const palette = getPalette(scheme as ColorScheme);
      for (const color of Object.values(palette.colors)) {
        expect(color.r).toBeGreaterThanOrEqual(0);
        expect(color.r).toBeLessThanOrEqual(255);
        expect(color.g).toBeGreaterThanOrEqual(0);
        expect(color.g).toBeLessThanOrEqual(255);
        expect(color.b).toBeGreaterThanOrEqual(0);
        expect(color.b).toBeLessThanOrEqual(255);
      }
    }
  });

  it('palettes have correct accent colors', () => {
    expect(getPalette(ColorScheme.MONO).accent).toBe('black');
    expect(getPalette(ColorScheme.BWR).accent).toBe('red');
    expect(getPalette(ColorScheme.BWY).accent).toBe('yellow');
    expect(getPalette(ColorScheme.BWRY).accent).toBe('red');
    expect(getPalette(ColorScheme.BWGBRY).accent).toBe('red');
    expect(getPalette(ColorScheme.GRAYSCALE_4).accent).toBe('black');
    expect(getPalette(ColorScheme.GRAYSCALE_8).accent).toBe('black');
    expect(getPalette(ColorScheme.GRAYSCALE_16).accent).toBe('black');
  });
});

describe('DitherMode', () => {
  it('has all expected modes', () => {
    expect(DitherMode.NONE).toBe(0);
    expect(DitherMode.BURKES).toBe(1);
    expect(DitherMode.ORDERED).toBe(2);
    expect(DitherMode.FLOYD_STEINBERG).toBe(3);
    expect(DitherMode.ATKINSON).toBe(4);
    expect(DitherMode.STUCKI).toBe(5);
    expect(DitherMode.SIERRA).toBe(6);
    expect(DitherMode.SIERRA_LITE).toBe(7);
    expect(DitherMode.JARVIS_JUDICE_NINKE).toBe(8);
  });
});

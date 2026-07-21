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
      const image = createGradient(10, 10);
      const result = ditherImage(image, ColorScheme.BWR, { mode: mode as DitherMode });

      expect(result.width).toBe(10);
      expect(result.height).toBe(10);
      expect(result.indices.length).toBe(100);
      expect(result.palette.length).toBe(3);

      // Every dithering mode (ordered or error-diffusion) must actually spread
      // error/thresholds differently than plain per-pixel thresholding (NONE) on
      // a gradient. This ties the assertion to the specific mode under test, so
      // a broken kernel for *that* mode fails *that* mode's case, not just some
      // other test in the suite.
      if (mode !== DitherMode.NONE) {
        const baseline = ditherImage(image, ColorScheme.BWR, { mode: DitherMode.NONE });
        const differsFromBaseline = result.indices.some((v, i) => v !== baseline.indices[i]);
        expect(differsFromBaseline).toBe(true);
      }
    }
  );

  it('per-mode outputs are not all identical (guards against the mode option being dropped)', () => {
    const image = createGradient(10, 10);
    const outputs = Object.values(DitherMode)
      .filter((v) => typeof v === 'number')
      .map((mode) => ditherImage(image, ColorScheme.BWR, { mode: mode as DitherMode }).indices);

    const distinctModePairs = outputs.some((indices, i) =>
      outputs.slice(i + 1).some((other) => !indices.every((v, idx) => v === other[idx]))
    );
    expect(distinctModePairs).toBe(true);
  });

  it.each(Object.values(ColorScheme).filter((v) => typeof v === 'number'))(
    'works with color scheme %s',
    (scheme) => {
      const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });
      const result = ditherImage(image, scheme as ColorScheme, { mode: DitherMode.BURKES });

      expect(result.palette.length).toBeGreaterThan(0);
    }
  );

  it('handles RGBA input correctly', () => {
    const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });
    const result = ditherImage(image, ColorScheme.BWR, { mode: DitherMode.BURKES });

    expect(result).toBeDefined();
    expect(result.width).toBe(10);
    expect(result.height).toBe(10);
  });

  it('produces different output for different algorithms', () => {
    const image = createGradient(100, 100);

    const burkes = ditherImage(image, ColorScheme.MONO, { mode: DitherMode.BURKES });
    const floydSteinberg = ditherImage(image, ColorScheme.MONO, { mode: DitherMode.FLOYD_STEINBERG });

    let differences = 0;
    for (let i = 0; i < burkes.indices.length; i++) {
      if (burkes.indices[i] !== floydSteinberg.indices[i]) differences++;
    }

    expect(differences).toBeGreaterThan(0);
  });

  it('default mode is BURKES', () => {
    const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });

    const withDefault = ditherImage(image, ColorScheme.BWR);
    const withBurkes = ditherImage(image, ColorScheme.BWR, { mode: DitherMode.BURKES });

    expect(withDefault.indices).toEqual(withBurkes.indices);
  });

  it('serpentine=true and serpentine=false produce different results on gradient', () => {
    const image = createGradient(50, 50);

    const withSerpentine    = ditherImage(image, ColorScheme.MONO, { mode: DitherMode.FLOYD_STEINBERG, serpentine: true });
    const withoutSerpentine = ditherImage(image, ColorScheme.MONO, { mode: DitherMode.FLOYD_STEINBERG, serpentine: false });

    let differences = 0;
    for (let i = 0; i < withSerpentine.indices.length; i++) {
      if (withSerpentine.indices[i] !== withoutSerpentine.indices[i]) differences++;
    }
    expect(differences).toBeGreaterThan(0);
  });

  it('alpha compositing: fully transparent red is treated as white', () => {
    // Fully transparent red (alpha=0) should composite to white
    const image = createTransparentTestImage(4, 4, { r: 255, g: 0, b: 0 }, 0);
    const result = ditherImage(image, ColorScheme.MONO, { mode: DitherMode.NONE });

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

    const resultOpaque = ditherImage(opaque, ColorScheme.MONO, { mode: DitherMode.NONE });
    const resultNearly = ditherImage(nearlyTransparent, ColorScheme.MONO, { mode: DitherMode.NONE });

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
    const result = ditherImage(image, SPECTRA_7_3_6COLOR, { mode: DitherMode.FLOYD_STEINBERG });

    for (let i = 0; i < result.indices.length; i++) {
      expect(result.indices[i]).toBeGreaterThanOrEqual(0);
      expect(result.indices[i]).toBeLessThan(6);
    }
  });

  it('defaults tone/gamut to off and accepts the off alias', () => {
    const image = createTestImage(16, 16, { r: 128, g: 128, b: 128 });
    const resultDefault = ditherImage(image, SPECTRA_7_3_6COLOR, { mode: DitherMode.BURKES });
    const resultZero = ditherImage(image, SPECTRA_7_3_6COLOR, {
      mode: DitherMode.BURKES,
      tone: 0.0,
      gamut: 0.0,
    });
    const resultOff = ditherImage(image, SPECTRA_7_3_6COLOR, {
      mode: DitherMode.BURKES,
      tone: 'off',
      gamut: 'off',
    });

    expect(resultDefault.indices).toEqual(resultZero.indices);
    expect(resultDefault.indices).toEqual(resultOff.indices);
  });

  it('measured palettes record their canonical firmware scheme', () => {
    expect(SPECTRA_7_3_6COLOR.scheme).toBe(ColorScheme.BWGBRY);
  });

  it('DitherMode.NONE maps pure display colors directly for measured palettes', () => {
    const red = getPalette(ColorScheme.BWGBRY).colors.red;
    const image = createTestImage(4, 4, red);
    const result = ditherImage(image, SPECTRA_7_3_6COLOR, { mode: DitherMode.NONE });

    expect(new Set(result.indices)).toEqual(new Set([3]));
  });

  it.each([DitherMode.ORDERED, DitherMode.BURKES, DitherMode.FLOYD_STEINBERG])(
    'pins exact display colors inside mixed measured images for mode %s',
    (mode) => {
      const green = getPalette(ColorScheme.BWGBRY).colors.green;
      const image = createTestImage(8, 4, { r: 128, g: 128, b: 128 });
      for (let y = 0; y < 2; y++) {
        for (let x = 0; x < 4; x++) {
          const idx = (y * image.width + x) * 4;
          image.data[idx] = green.r;
          image.data[idx + 1] = green.g;
          image.data[idx + 2] = green.b;
          image.data[idx + 3] = 255;
        }
      }

      const result = ditherImage(image, SPECTRA_7_3_6COLOR, { mode });
      const pinned = [];
      for (let y = 0; y < 2; y++) {
        for (let x = 0; x < 4; x++) {
          pinned.push(result.indices[y * image.width + x]);
        }
      }

      expect(new Set(pinned)).toEqual(new Set([5]));
    }
  );

  it('predefined measured palettes return measured preview colors', () => {
    const red = getPalette(ColorScheme.BWGBRY).colors.red;
    const image = createTestImage(1, 1, red);
    const result = ditherImage(image, SPECTRA_7_3_6COLOR, { mode: DitherMode.NONE });

    expect(result.palette[3]).toEqual(SPECTRA_7_3_6COLOR.colors.red);
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
    expect(getColorCount(ColorScheme.GRAYSCALE_16)).toBe(16);
    expect(getColorCount(ColorScheme.SEVEN_COLOR)).toBe(7);
    expect(getColorCount(ColorScheme.BWGBRY_SPLIT)).toBe(6);
  });

  it('fromValue works correctly for all schemes', () => {
    expect(fromValue(0)).toBe(ColorScheme.MONO);
    expect(fromValue(1)).toBe(ColorScheme.BWR);
    expect(fromValue(5)).toBe(ColorScheme.GRAYSCALE_4);
    expect(fromValue(6)).toBe(ColorScheme.GRAYSCALE_16);
    expect(fromValue(7)).toBe(ColorScheme.SEVEN_COLOR);
    expect(fromValue(8)).toBe(ColorScheme.BWGBRY_SPLIT);
  });

  it('fromValue throws for values that are not enum members', () => {
    expect(() => fromValue(9)).toThrow();
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

  it('has correct palette key order for the protocol v2 schemes', () => {
    // Palette color order is a wire contract: language bindings derive pixel
    // indices from object-literal key order. A silent reorder would pass every
    // count/accent assertion but ship wrong ink indices to hardware.
    expect(Object.keys(getPalette(ColorScheme.SEVEN_COLOR).colors)).toEqual([
      'black',
      'white',
      'yellow',
      'red',
      'blue',
      'green',
      'orange',
    ]);
    expect(Object.keys(getPalette(ColorScheme.BWGBRY_SPLIT).colors)).toEqual([
      'black',
      'white',
      'yellow',
      'red',
      'blue',
      'green',
    ]);
  });

  it('palettes have correct accent colors', () => {
    expect(getPalette(ColorScheme.MONO).accent).toBe('black');
    expect(getPalette(ColorScheme.BWR).accent).toBe('red');
    expect(getPalette(ColorScheme.BWY).accent).toBe('yellow');
    expect(getPalette(ColorScheme.BWRY).accent).toBe('red');
    expect(getPalette(ColorScheme.BWGBRY).accent).toBe('red');
    expect(getPalette(ColorScheme.GRAYSCALE_4).accent).toBe('black');
    expect(getPalette(ColorScheme.GRAYSCALE_16).accent).toBe('black');
    expect(getPalette(ColorScheme.SEVEN_COLOR).accent).toBe('red');
    expect(getPalette(ColorScheme.BWGBRY_SPLIT).accent).toBe('red');
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

describe('measured palette validation', () => {
  it('throws a clear error when the accent name is not a palette color', () => {
    const image = createTestImage(4, 4, { r: 128, g: 128, b: 128 });
    const badPalette = {
      colors: {
        black: { r: 0, g: 0, b: 0 },
        white: { r: 255, g: 255, b: 255 },
      },
      accent: 'crimson', // not present in colors
    };
    expect(() => ditherImage(image, badPalette, { mode: DitherMode.BURKES })).toThrow(
      /accent color 'crimson' not found/
    );
  });

  it('throws instead of silently falling through when a measured palette carries an invalid scheme id', () => {
    const image = createTestImage(4, 4, { r: 128, g: 128, b: 128 });
    const badSchemePalette = {
      colors: {
        black: { r: 0, g: 0, b: 0 },
        white: { r: 255, g: 255, b: 255 },
      },
      accent: 'black',
      scheme: 99, // not a valid ColorScheme value (0-8)
    };
    expect(() => ditherImage(image, badSchemePalette, { mode: DitherMode.BURKES })).toThrow();
  });
});

describe('image buffer validation', () => {
  it('throws when data.length does not match width * height * 4', () => {
    const shortImage = {
      width: 10,
      height: 10,
      // Deliberately too short: claims 10x10 but only carries 4x10 worth of RGBA data.
      data: new Uint8ClampedArray(4 * 10 * 4),
    };
    expect(() => ditherImage(shortImage, ColorScheme.BWR, { mode: DitherMode.BURKES })).toThrow(
      /image data length/
    );
  });
});

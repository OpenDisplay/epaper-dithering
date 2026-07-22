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
// Imported after '../src' so that core.ts has already run __wbg_set_wasm.
import { composite_rgba as compositeRgba } from '../src/wasm-core/epaper_dithering_wasm_bg.js';

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

  // Key order AND literal RGB values for every scheme, written out by hand rather than
  // derived from the code under test — so this cannot pass just because the source agrees
  // with itself. Object key order IS the palette index: the WASM core computes indices from
  // Rust's PALETTE_* statics while `PaletteImageBuffer.palette` returns these colors, so a
  // reorder or an RGB typo here ships the wrong ink at the right index. Every other palette
  // assertion in this file checks only counts, accents, or 0<=c<=255 ranges and would miss it.
  const EXPECTED_PALETTES: [ColorScheme, [string, number, number, number][]][] = [
    [ColorScheme.MONO, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
    ]],
    [ColorScheme.BWR, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
      ['red', 255, 0, 0],
    ]],
    [ColorScheme.BWY, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
      ['yellow', 255, 255, 0],
    ]],
    // black, white, YELLOW, red — matches firmware BBEP_YELLOW=2, BBEP_RED=3.
    [ColorScheme.BWRY, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
      ['yellow', 255, 255, 0],
      ['red', 255, 0, 0],
    ]],
    [ColorScheme.BWGBRY, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
      ['yellow', 255, 255, 0],
      ['red', 255, 0, 0],
      ['blue', 0, 0, 255],
      ['green', 0, 255, 0],
    ]],
    [ColorScheme.GRAYSCALE_4, [
      ['black', 0, 0, 0],
      ['gray1', 85, 85, 85],
      ['gray2', 170, 170, 170],
      ['white', 255, 255, 255],
    ]],
    [ColorScheme.GRAYSCALE_16, [
      ['black', 0, 0, 0],
      ['gray1', 17, 17, 17],
      ['gray2', 34, 34, 34],
      ['gray3', 51, 51, 51],
      ['gray4', 68, 68, 68],
      ['gray5', 85, 85, 85],
      ['gray6', 102, 102, 102],
      ['gray7', 119, 119, 119],
      ['gray8', 136, 136, 136],
      ['gray9', 153, 153, 153],
      ['gray10', 170, 170, 170],
      ['gray11', 187, 187, 187],
      ['gray12', 204, 204, 204],
      ['gray13', 221, 221, 221],
      ['gray14', 238, 238, 238],
      ['white', 255, 255, 255],
    ]],
    [ColorScheme.SEVEN_COLOR, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
      ['yellow', 255, 255, 0],
      ['red', 255, 0, 0],
      ['blue', 0, 0, 255],
      ['green', 0, 255, 0],
      ['orange', 255, 128, 0],
    ]],
    [ColorScheme.BWGBRY_SPLIT, [
      ['black', 0, 0, 0],
      ['white', 255, 255, 255],
      ['yellow', 255, 255, 0],
      ['red', 255, 0, 0],
      ['blue', 0, 0, 255],
      ['green', 0, 255, 0],
    ]],
  ];

  it.each(EXPECTED_PALETTES)('scheme %s has exact palette key order and RGB values', (scheme, expected) => {
    const actual = Object.entries(getPalette(scheme).colors).map(
      ([name, c]) => [name, c.r, c.g, c.b],
    );
    expect(actual).toEqual(expected);
  });

  it('pins every scheme in EXPECTED_PALETTES', () => {
    // Guards the table above against a newly added scheme slipping past it untested.
    const covered = EXPECTED_PALETTES.map(([scheme]) => scheme).sort((a, b) => a - b);
    const all = Object.values(ColorScheme)
      .filter((v): v is ColorScheme => typeof v === 'number')
      .sort((a, b) => a - b);
    expect(covered).toEqual(all);
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

/**
 * Cross-language contract for RGBA → RGB compositing.
 *
 * The same fixed input and the same literal expected bytes are asserted in
 * `packages/python/tests/test_dithering.py::TestCompositeRgba` and in
 * `packages/rust/core/src/composite.rs::cross_language_reference_vector`.
 * The three copies are written independently — none is generated from another —
 * so a divergence in any binding fails here.
 */
describe('RGBA compositing (shared core implementation)', () => {
  // 12 pixels: mid-gray at alpha 0/1/127/128/254/255, a saturated color at the
  // same rounding-sensitive alphas, plus two arbitrary mixed pixels.
  const RGBA = new Uint8Array([
    128, 128, 128, 0,
    128, 128, 128, 1,
    128, 128, 128, 127,
    128, 128, 128, 128,
    128, 128, 128, 254,
    128, 128, 128, 255,
    0, 64, 200, 1,
    0, 64, 200, 127,
    0, 64, 200, 128,
    0, 64, 200, 254,
    17, 200, 3, 63,
    250, 5, 130, 191,
  ]);

  const EXPECTED_RGB = [
    255, 255, 255,
    255, 255, 255,
    192, 192, 192,
    191, 191, 191,
    128, 128, 128,
    128, 128, 128,
    254, 254, 255,
    128, 160, 228,
    127, 159, 227,
    1, 65, 200,
    196, 241, 193,
    251, 68, 161,
  ];

  it('produces the exact expected RGB bytes', () => {
    expect(Array.from(compositeRgba(RGBA))).toEqual(EXPECTED_RGB);
  });

  it('leaves fully opaque pixels bit-identical to their input', () => {
    const opaque = new Uint8Array([12, 34, 56, 255, 200, 199, 198, 255]);
    expect(Array.from(compositeRgba(opaque))).toEqual([12, 34, 56, 200, 199, 198]);
  });

  it('throws when the buffer length is not a multiple of 4', () => {
    for (const len of [1, 2, 3, 5, 7]) {
      expect(() => compositeRgba(new Uint8Array(len))).toThrow(/multiple of 4/);
    }
  });

  it('accepts an empty buffer', () => {
    expect(Array.from(compositeRgba(new Uint8Array(0)))).toEqual([]);
  });
});

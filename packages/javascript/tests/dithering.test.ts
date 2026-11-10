import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { ditherImage, DitherMode, Palette } from '../src';
import type { DbsParams, DitherOptions } from '../src';
import { createTestImage, createGradient, createTransparentTestImage } from './fixtures';
// Imported after '../src' so that core.ts has already run initSync.
import { composite_rgba as compositeRgba } from '../src/wasm-core/epaper_dithering_wasm.js';

const EXAMPLES = resolve(__dirname, '../../../examples/palettes');
const example = (name: string) => Palette.fromJson(readFileSync(resolve(EXAMPLES, `${name}.json`), 'utf8'));
const MONO = example('ideal_mono');
const BWR = example('ideal_bwr');
const SPECTRA_7_3_6COLOR = example('spectra_7_3_6color');

describe('Dithering Algorithms', () => {
  it.each(Object.values(DitherMode))(
    'produces valid output for mode %s',
    (mode) => {
      const image = createGradient(10, 10);
      const result = ditherImage(image, BWR, { mode });

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
        const baseline = ditherImage(image, BWR, { mode: DitherMode.NONE });
        const differsFromBaseline = result.indices.some((v, i) => v !== baseline.indices[i]);
        expect(differsFromBaseline).toBe(true);
      }
    }
  );

  it('per-mode outputs are not all identical (guards against the mode option being dropped)', () => {
    const image = createGradient(10, 10);
    const outputs = Object.values(DitherMode).map((mode) => ditherImage(image, BWR, { mode }).indices);

    const distinctModePairs = outputs.some((indices, i) =>
      outputs.slice(i + 1).some((other) => !indices.every((v, idx) => v === other[idx]))
    );
    expect(distinctModePairs).toBe(true);
  });

  it.each(['mono', 'bwr', 'bwy', 'bwry', 'bwgbry', 'grayscale4', 'grayscale16', 'seven_color', 'grayscale8'])(
    'works with ideal palette %s',
    (name) => {
      const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });
      const palette = example(`ideal_${name}`);
      const result = ditherImage(image, palette, { mode: DitherMode.BURKES });

      expect(result.palette.length).toBe(palette.length);
      expect(Math.max(...result.indices)).toBeLessThan(palette.length);
    }
  );

  it('default mode is BURKES', () => {
    const image = createTestImage(10, 10, { r: 128, g: 128, b: 128 });

    const withDefault = ditherImage(image, BWR);
    const withBurkes = ditherImage(image, BWR, { mode: DitherMode.BURKES });

    expect(withDefault.indices).toEqual(withBurkes.indices);
  });

  it('serpentine=true and serpentine=false produce different results on gradient', () => {
    const image = createGradient(50, 50);

    const withSerpentine    = ditherImage(image, MONO, { mode: DitherMode.FLOYD_STEINBERG, serpentine: true });
    const withoutSerpentine = ditherImage(image, MONO, { mode: DitherMode.FLOYD_STEINBERG, serpentine: false });

    let differences = 0;
    for (let i = 0; i < withSerpentine.indices.length; i++) {
      if (withSerpentine.indices[i] !== withoutSerpentine.indices[i]) differences++;
    }
    expect(differences).toBeGreaterThan(0);
  });

  it('alpha compositing: fully transparent red is treated as white', () => {
    // Fully transparent red (alpha=0) should composite to white
    const image = createTransparentTestImage(4, 4, { r: 255, g: 0, b: 0 }, 0);
    const result = ditherImage(image, MONO, { mode: DitherMode.NONE });

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

    const resultOpaque = ditherImage(opaque, MONO, { mode: DitherMode.NONE });
    const resultNearly = ditherImage(nearlyTransparent, MONO, { mode: DitherMode.NONE });

    expect(resultOpaque.indices[0]).toBe(0);    // black
    expect(resultNearly.indices[0]).toBe(1);    // white
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

  it.each([DitherMode.ORDERED, DitherMode.BURKES, DitherMode.FLOYD_STEINBERG])(
    'pins exact display colors inside mixed measured images for mode %s',
    (mode) => {
      const green = { r: 0, g: 255, b: 0 };
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

  it('measured palettes return measured preview colors', () => {
    const image = createTestImage(1, 1, { r: 255, g: 0, b: 0 });
    const result = ditherImage(image, SPECTRA_7_3_6COLOR, { mode: DitherMode.NONE });

    expect(result.palette[3]).toEqual(SPECTRA_7_3_6COLOR.previewRgb()[3]);
  });
});

describe('DitherMode', () => {
  it('uses the core mode names', () => {
    expect(Object.values(DitherMode)).toEqual([
      'none', 'burkes', 'ordered', 'floyd_steinberg', 'atkinson',
      'stucki', 'sierra', 'sierra_lite', 'jarvis_judice_ninke', 'dizzy',
    ]);
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
    expect(() => ditherImage(shortImage, BWR, { mode: DitherMode.BURKES })).toThrow(
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

describe('Dizzy dithering', () => {
  const rampImage = () => {
    const data = new Uint8ClampedArray(4 * 4 * 4);
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        const v = x * 60 + y * 5;
        const o = (y * 4 + x) * 4;
        data[o] = v; data[o + 1] = v; data[o + 2] = v; data[o + 3] = 255;
      }
    }
    return { data, width: 4, height: 4 };
  };

  it('matches the Rust reference vector byte for byte', () => {
    // Snapping off: the vector pins the raw algorithm, shared with Rust and Python.
    const result = ditherImage(rampImage(), BWR, { mode: DitherMode.DIZZY, snap: false });
    // Frozen literals, identical to the Rust and Python suites.
    expect(Array.from(result.indices)).toEqual([0, 0, 1, 0, 0, 2, 2, 1, 0, 1, 1, 1, 0, 0, 2, 1]);
  });

  it('is deterministic', () => {
    const a = ditherImage(rampImage(), BWR, { mode: DitherMode.DIZZY });
    const b = ditherImage(rampImage(), BWR, { mode: DitherMode.DIZZY });
    expect(Array.from(a.indices)).toEqual(Array.from(b.indices));
  });
});

describe('DBS refinement', () => {
  // Grayscale gradient on BWR: enough structure that DBS changes the halftone.
  const image = createGradient(40, 30);
  const indices = (options: DitherOptions = {}) => Array.from(ditherImage(image, BWR, options).indices);

  it('changes the output for an ideal palette', () => {
    expect(indices({ dbs: true })).not.toEqual(indices());
  });

  it('changes the output for a measured palette', () => {
    const plain = ditherImage(image, SPECTRA_7_3_6COLOR).indices;
    const refined = ditherImage(image, SPECTRA_7_3_6COLOR, { dbs: true }).indices;
    expect(Array.from(refined)).not.toEqual(Array.from(plain));
  });

  it('true means default parameters', () => {
    expect(indices({ dbs: true })).toEqual(indices({ dbs: {} }));
    expect(indices({ dbs: true })).toEqual(indices({ dbs: { viewingDistanceCm: 40, ppi: 127, maxPasses: 10 } }));
  });

  it('false and omitted match plain dithering', () => {
    expect(indices({ dbs: false })).toEqual(indices());
  });

  it('passes viewing geometry through', () => {
    expect(indices({ dbs: { viewingDistanceCm: 15 } })).not.toEqual(indices({ dbs: { viewingDistanceCm: 150 } }));
  });

  it('zero passes matches plain dithering', () => {
    expect(indices({ dbs: { maxPasses: 0 } })).toEqual(indices());
  });

  it.each([
    [{ ppi: 0 }, /ppi/],
    [{ ppi: Number.NaN }, /ppi/],
    [{ viewingDistanceCm: -1 }, /viewingDistanceCm/],
    [{ maxPasses: -1 }, /maxPasses/],
    [{ maxPasses: 1.5 }, /maxPasses/],
  ] as [DbsParams, RegExp][])('rejects invalid parameters %o', (dbs, message) => {
    expect(() => ditherImage(image, BWR, { dbs })).toThrow(message);
  });
});

import { describe, it, expect } from 'vitest';
import { precomputePaletteLab, matchPixelLch } from '../src/color_space_lab';
import { SRGB_TO_LINEAR_LUT } from '../src/color_space';

function srgbToLinear(r: number, g: number, b: number): [number, number, number] {
  return [SRGB_TO_LINEAR_LUT[r], SRGB_TO_LINEAR_LUT[g], SRGB_TO_LINEAR_LUT[b]];
}

describe('precomputePaletteLab', () => {
  it('returns arrays of correct length', () => {
    const palette: Array<[number, number, number]> = [
      [0, 0, 0], [1, 1, 1], [1, 0, 0],
    ];
    const result = precomputePaletteLab(palette);
    expect(result.L.length).toBe(3);
    expect(result.a.length).toBe(3);
    expect(result.b.length).toBe(3);
    expect(result.C.length).toBe(3);
  });

  it('black has L≈0', () => {
    const result = precomputePaletteLab([[0, 0, 0]]);
    expect(result.L[0]).toBeCloseTo(0, 1);
  });

  it('white has L≈100', () => {
    const result = precomputePaletteLab([[1, 1, 1]]);
    expect(result.L[0]).toBeCloseTo(100, 1);
  });

  it('chroma is non-negative', () => {
    const palette: Array<[number, number, number]> = [
      [0, 0, 0], [1, 1, 1], [1, 0, 0], [0, 1, 0], [0, 0, 1],
    ];
    const result = precomputePaletteLab(palette);
    for (let i = 0; i < palette.length; i++) {
      expect(result.C[i]).toBeGreaterThanOrEqual(0);
    }
  });
});

describe('matchPixelLch', () => {
  const bwrPaletteLinear: Array<[number, number, number]> = [
    [0, 0, 0],           // black
    [1, 1, 1],           // white
    [1, 0, 0],           // red (linear)
  ];

  it('pure black maps to black index', () => {
    const { L, a, b, C } = precomputePaletteLab(bwrPaletteLinear);
    expect(matchPixelLch(0, 0, 0, L, a, b, C)).toBe(0);
  });

  it('pure white maps to white index', () => {
    const { L, a, b, C } = precomputePaletteLab(bwrPaletteLinear);
    expect(matchPixelLch(1, 1, 1, L, a, b, C)).toBe(1);
  });

  it('pure red maps to red index', () => {
    const { L, a, b, C } = precomputePaletteLab(bwrPaletteLinear);
    expect(matchPixelLch(1, 0, 0, L, a, b, C)).toBe(2);
  });

  it('green input on BWR palette maps to black or white — not red (hue preservation)', () => {
    const { L, a, b, C } = precomputePaletteLab(bwrPaletteLinear);
    // Green in linear: sRGB(0, 200, 0)
    const [r, g, bl] = srgbToLinear(0, 200, 0);
    const idx = matchPixelLch(r, g, bl, L, a, b, C);
    // Must NOT match red (index 2) — green hue is closer to black/white than red
    expect(idx).not.toBe(2);
  });

  it('green input on SPECTRA_7_3_6COLOR maps to green index (critical LCH regression)', () => {
    // SPECTRA palette: black=0, white=1, yellow=2, red=3, blue=4, green=5
    const spectraLinear: Array<[number, number, number]> = [
      srgbToLinear(26,  13,  35 ),  // 0: black
      srgbToLinear(185, 202, 205),  // 1: white
      srgbToLinear(202, 184, 0  ),  // 2: yellow
      srgbToLinear(121, 9,   0  ),  // 3: red
      srgbToLinear(0,   69,  139),  // 4: blue
      srgbToLinear(40,  82,  57 ),  // 5: green
    ];
    const { L, a, b, C } = precomputePaletteLab(spectraLinear);

    // Bright saturated green — should map to green (index 5), not yellow (index 2)
    const [r, g, bl] = srgbToLinear(100, 255, 40);
    const idx = matchPixelLch(r, g, bl, L, a, b, C);
    expect(idx).toBe(5);
    expect(idx).not.toBe(2); // not yellow
  });
});

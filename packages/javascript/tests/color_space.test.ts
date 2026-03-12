import { describe, it, expect } from 'vitest';
import { SRGB_TO_LINEAR_LUT, linearToSrgbByte } from '../src/color_space';

describe('SRGB_TO_LINEAR_LUT', () => {
  it('black stays black', () => {
    expect(SRGB_TO_LINEAR_LUT[0]).toBe(0.0);
  });

  it('white stays white', () => {
    expect(SRGB_TO_LINEAR_LUT[255]).toBeCloseTo(1.0, 10);
  });

  it('128 is approximately 0.2158 (nonlinear midpoint)', () => {
    expect(SRGB_TO_LINEAR_LUT[128]).toBeCloseTo(0.2158, 3);
  });

  it('is monotonically non-decreasing', () => {
    for (let i = 1; i < 256; i++) {
      expect(SRGB_TO_LINEAR_LUT[i]).toBeGreaterThanOrEqual(SRGB_TO_LINEAR_LUT[i - 1]);
    }
  });

  it('has 256 entries', () => {
    expect(SRGB_TO_LINEAR_LUT.length).toBe(256);
  });
});

describe('linearToSrgbByte', () => {
  it('0.0 → 0', () => {
    expect(linearToSrgbByte(0.0)).toBe(0);
  });

  it('1.0 → 255', () => {
    expect(linearToSrgbByte(1.0)).toBe(255);
  });

  it('0.5 → ~188', () => {
    const v = linearToSrgbByte(0.5);
    expect(v).toBeGreaterThanOrEqual(185);
    expect(v).toBeLessThanOrEqual(189);
  });

  it('roundtrip: linearToSrgbByte(LUT[i]) ≈ i for all 0–255', () => {
    for (let i = 0; i < 256; i++) {
      const roundtripped = linearToSrgbByte(SRGB_TO_LINEAR_LUT[i]);
      expect(Math.abs(roundtripped - i)).toBeLessThanOrEqual(1);
    }
  });
});

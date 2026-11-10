import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { ditherImage, DitherMode, Palette, VERSION } from '../src';
import type { PaletteEntry } from '../src';
import { createGradient, createTestImage } from './fixtures';
import pkg from '../package.json';
import { PNG } from 'pngjs';

const ROOT = resolve(__dirname, '../../..');
const EXAMPLES = resolve(ROOT, 'examples/palettes');
const FIXTURES = resolve(ROOT, 'packages/rust/core/tests/fixtures');

const example = (name: string) => Palette.fromJson(readFileSync(resolve(EXAMPLES, `${name}.json`), 'utf8'));

describe('Palette', () => {
  it.each(readdirSync(EXAMPLES).filter((f) => f.endsWith('.json')))('loads example %s', (file) => {
    const palette = Palette.fromJson(readFileSync(resolve(EXAMPLES, file), 'utf8'));
    expect(palette.length).toBeGreaterThanOrEqual(2);
  });

  it('round-trips through JSON', () => {
    const palette = example('spectra_7_3_6color');
    const again = Palette.fromJson(palette.toJson());
    expect(again.entries).toEqual(palette.entries);
    expect(again.measured).toBe(true);
  });

  it('exposes entries with appearance and ink', () => {
    const black = example('ideal_bwr').entries[0];
    expect(black.name).toBe('black');
    expect(black.nominal).toBe('#000000');
    expect(black.appearance[0]).toBeCloseTo(0, 6);
  });

  it('finds inks by name', () => {
    const palette = example('ideal_bwry');
    expect(palette.indexOf('red')).toBe(3);
    expect(palette.indexOf('blue')).toBeUndefined();
  });

  it('rejects an unnormalised white and names the fix', () => {
    const entries: PaletteEntry[] = [
      { appearance: [68.26, 8.33, 5.98], name: 'white', nominal: '#ffffff' },
      { appearance: [30.47, 8.7, 4.58], name: 'black', nominal: '#000000' },
    ];
    expect(() => new Palette(entries, true)).toThrow(/from_absolute_lab/);
  });

  it('rejects duplicate ink names', () => {
    const entries: PaletteEntry[] = [
      { appearance: [0, 0, 0], name: 'black', nominal: '#000000' },
      { appearance: [20, 0, 0], name: 'black', nominal: '#111111' },
    ];
    expect(() => new Palette(entries)).toThrow(/black/);
  });

  it('normalises absolute measurements with fromAbsoluteLab', () => {
    const palette = Palette.fromAbsoluteLab([
      { appearance: [30.47, 8.7, 4.58], name: 'black', nominal: '#000000' },
      { appearance: [68.26, 8.33, 5.98], name: 'white', nominal: '#ffffff' },
    ], 1, true);
    const [l, a, b] = palette.entries[1].appearance;
    expect(l).toBeCloseTo(100, 9);
    expect(a).toBeCloseTo(0, 9);
    expect(b).toBeCloseTo(0, 9);
    expect(palette.entries[0].appearance[0]).toBeGreaterThan(30.47);
    expect(palette.measured).toBe(true);
  });

  it('builds from sRGB with fromSrgb, normalising to the white ink', () => {
    const palette = Palette.fromSrgb([
      { rgb: { r: 26, g: 13, b: 35 }, name: 'black', nominal: '#000000' },
      { rgb: { r: 185, g: 202, b: 205 }, name: 'white', nominal: '#ffffff' },
      { rgb: { r: 121, g: 9, b: 0 } },
    ]);
    expect(palette.previewRgb()[1]).toEqual({ r: 255, g: 255, b: 255 });
    expect(palette.indexOf('white')).toBe(1);
    expect(palette.length).toBe(3);
  });

  it('rejects a bad white index', () => {
    expect(() => Palette.fromAbsoluteLab([{ appearance: [50, 0, 0] }, { appearance: [60, 0, 0] }], 5)).toThrow(/out of range/);
  });

  it('rejects malformed JSON', () => {
    expect(() => Palette.fromJson('{')).toThrow();
  });
});

describe('ditherImage with palettes', () => {
  it('accepts mode names and defaults to burkes', () => {
    const image = createGradient(16, 4);
    const palette = example('ideal_mono');
    const byDefault = ditherImage(image, palette);
    const byName = ditherImage(image, palette, { mode: 'burkes' });
    const byEnum = ditherImage(image, palette, { mode: DitherMode.BURKES });
    expect(byName.indices).toEqual(byDefault.indices);
    expect(byEnum.indices).toEqual(byDefault.indices);
  });

  it('throws on an unknown mode name', () => {
    expect(() => ditherImage(createGradient(4, 4), example('ideal_mono'), { mode: 'bayer' as DitherMode })).toThrow(/bayer/);
  });

  it('returns the palette preview colours', () => {
    const palette = example('spectra_7_3_6color');
    const result = ditherImage(createTestImage(4, 4, { r: 128, g: 128, b: 128 }), palette);
    expect(result.palette).toEqual(palette.previewRgb());
  });

  it('keeps pure black dashboard text solid', () => {
    const png = PNG.sync.read(readFileSync(resolve(FIXTURES, 'images/success/dashboard_photo.png')));
    const palette = Palette.fromJson(readFileSync(resolve(FIXTURES, 'palettes/bwry16_odbeeba6.json'), 'utf8'));
    const result = ditherImage({ width: png.width, height: png.height, data: new Uint8ClampedArray(png.data) }, palette);
    const black = palette.indexOf('black');
    for (let i = 0; i < png.width * png.height; i++) {
      if (png.data[i * 4] === 0 && png.data[i * 4 + 1] === 0 && png.data[i * 4 + 2] === 0) {
        expect(result.indices[i]).toBe(black);
      }
    }
  });

  it('snaps exact ink colours by default and not with snap: false', () => {
    const palette = example('spectra_7_3_6color');
    const red = createTestImage(2, 2, { r: 255, g: 0, b: 0 });
    expect(Array.from(ditherImage(red, palette, { mode: 'none' }).indices)).toEqual([3, 3, 3, 3]);
    const unsnapped = ditherImage(red, palette, { mode: 'none', snap: false, anchor: false });
    expect(unsnapped.indices.length).toBe(4);
  });

  it('throws when the image buffer has the wrong length', () => {
    const image = { width: 3, height: 3, data: new Uint8ClampedArray(10) };
    expect(() => ditherImage(image, example('ideal_mono'))).toThrow(/width × height × 4/);
  });
});

describe('VERSION', () => {
  it('matches package.json', () => {
    expect(VERSION).toBe(pkg.version);
  });
});

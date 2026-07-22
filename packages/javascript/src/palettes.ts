import type { ColorPalette } from './types';

/**
 * E-paper display color schemes.
 *
 * Integer values are a firmware wire contract. Canonical source of truth:
 * `enum ColorScheme` in `opendisplay-protocol/src/opendisplay_structs.h`, which
 * names `packages/rust/core/src/palettes.rs` as its designated mirror.
 */
export enum ColorScheme {
  MONO        = 0,
  BWR         = 1,
  BWY         = 2,
  BWRY        = 3,
  BWGBRY      = 4,
  GRAYSCALE_4 = 5,
  GRAYSCALE_16 = 6,
  /**
   * 7-color Spectra/ACeP. Protocol v2 reassigned value 7 from the former
   * GRAYSCALE_8 (a mistake — gray8 is not a real panel scheme; removed, not
   * renumbered). Ink order is BWGBRY plus orange, matching the bb_epaper
   * logical ink indices.
   */
  SEVEN_COLOR = 7,
  /** Spectra 6 nibbles, left-half plane then right-half plane (dual-CS panels). */
  BWGBRY_SPLIT = 8,
}

const PALETTES: Record<ColorScheme, ColorPalette> = {
  [ColorScheme.MONO]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
  [ColorScheme.BWR]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      red: { r: 255, g: 0, b: 0 },
    },
    accent: 'red',
  },
  [ColorScheme.BWY]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0 },
    },
    accent: 'yellow',
  },
  [ColorScheme.BWRY]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0 },
      red: { r: 255, g: 0, b: 0 },
    },
    accent: 'red',
  },
  [ColorScheme.BWGBRY]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0 },
      red: { r: 255, g: 0, b: 0 },
      blue: { r: 0, g: 0, b: 255 },
      green: { r: 0, g: 255, b: 0 },
    },
    accent: 'red',
  },
  [ColorScheme.GRAYSCALE_4]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      gray1: { r: 85, g: 85, b: 85 },
      gray2: { r: 170, g: 170, b: 170 },
      white: { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
  [ColorScheme.SEVEN_COLOR]: {
    colors: {
      black:  { r: 0,   g: 0,   b: 0   },
      white:  { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0   },
      red:    { r: 255, g: 0,   b: 0   },
      blue:   { r: 0,   g: 0,   b: 255 },
      green:  { r: 0,   g: 255, b: 0   },
      orange: { r: 255, g: 128, b: 0   },
    },
    accent: 'red',
  },
  // Same inks as BWGBRY; only the firmware-side plane packing differs.
  [ColorScheme.BWGBRY_SPLIT]: {
    colors: {
      black:  { r: 0,   g: 0,   b: 0   },
      white:  { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0   },
      red:    { r: 255, g: 0,   b: 0   },
      blue:   { r: 0,   g: 0,   b: 255 },
      green:  { r: 0,   g: 255, b: 0   },
    },
    accent: 'red',
  },
  [ColorScheme.GRAYSCALE_16]: {
    colors: {
      black:  { r: 0,   g: 0,   b: 0   },
      gray1:  { r: 17,  g: 17,  b: 17  },
      gray2:  { r: 34,  g: 34,  b: 34  },
      gray3:  { r: 51,  g: 51,  b: 51  },
      gray4:  { r: 68,  g: 68,  b: 68  },
      gray5:  { r: 85,  g: 85,  b: 85  },
      gray6:  { r: 102, g: 102, b: 102 },
      gray7:  { r: 119, g: 119, b: 119 },
      gray8:  { r: 136, g: 136, b: 136 },
      gray9:  { r: 153, g: 153, b: 153 },
      gray10: { r: 170, g: 170, b: 170 },
      gray11: { r: 187, g: 187, b: 187 },
      gray12: { r: 204, g: 204, b: 204 },
      gray13: { r: 221, g: 221, b: 221 },
      gray14: { r: 238, g: 238, b: 238 },
      white:  { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
};

/** Get color palette for a color scheme */
export function getPalette(scheme: ColorScheme): ColorPalette {
  return PALETTES[scheme];
}

/** Get number of colors in a color scheme */
export function getColorCount(scheme: ColorScheme): number {
  return Object.keys(PALETTES[scheme].colors).length;
}

/**
 * Create ColorScheme from firmware integer value.
 *
 * Membership test, not a range check: the enum is not guaranteed to stay a
 * contiguous 0..N run (protocol v2 already removed a value), so a range check
 * would accept reserved gaps and reject newly added values.
 */
const VALID_SCHEME_VALUES: ReadonlySet<number> = new Set(
  Object.values(ColorScheme).filter((v): v is ColorScheme => typeof v === 'number'),
);

export function fromValue(value: number): ColorScheme {
  if (!VALID_SCHEME_VALUES.has(value)) {
    throw new Error(`Invalid color scheme value: ${value}`);
  }
  return value as ColorScheme;
}

// =============================================================================
// Measured Palettes for Specific E-Paper Displays
// =============================================================================
//
// These constants provide measured RGB values from real e-paper displays.
// Pass them directly to ditherImage() instead of a ColorScheme enum.
//
// Usage:
//   import { ditherImage, SPECTRA_7_3_6COLOR } from '@opendisplay/epaper-dithering';
//   const result = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR);
//
// RGB values are defined once in packages/rust/core/src/measured_palettes.rs (single
// source of truth) and generated into ./palettes.generated.ts by
// packages/rust/core/examples/gen_ts_palettes.rs; CI runs that generator in `--check`
// mode to catch drift. This mirrors how the Python package derives its constants from
// the same Rust CATALOG via FFI at import time -- TypeScript can't do that at import
// time (WASM init order), so it does the equivalent at build time instead.
//
// TO ADD A NEW DISPLAY: add the palette + CATALOG entry in measured_palettes.rs, then
// regenerate (`cargo run --manifest-path packages/rust/core/Cargo.toml --example
// gen_ts_palettes -- --write`) and commit the updated palettes.generated.ts.
// =============================================================================

export {
  SPECTRA_7_3_6COLOR,
  SPECTRA_7_3_6COLOR_V2,
  MONO_4_26,
  BWRY_4_2,
  BWRY_3_97,
  SOLUM_BWR,
  HANSHOW_BWR,
  HANSHOW_BWY,
} from './palettes.generated';

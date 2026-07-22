import type { ColorPalette } from './types';
import { SCHEME_PALETTES } from './palettes.generated';

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

/**
 * Idealized (pure sRGB) palette for every ColorScheme, keyed by firmware wire value.
 *
 * NOT hand-written. Generated from Rust's `PALETTE_*` statics + `ColorScheme::color_names()`
 * by `packages/rust/core/examples/gen_ts_palettes.rs`, whose `--check` mode gates CI.
 * This matters because `ditherImage()` gets palette *indices* from the WASM core (computed
 * against those Rust statics) but returns the *colors* from this table: if the two ever
 * disagreed, a consumer would render or encode the wrong ink at the right index, and every
 * count-, length- and accent-based assertion would still pass.
 *
 * The generated record is typed `Record<number, ColorPalette>` (it cannot import the
 * `ColorScheme` enum without creating a circular ES module), so it is narrowed here.
 */
const PALETTES = SCHEME_PALETTES as Record<ColorScheme, ColorPalette>;

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

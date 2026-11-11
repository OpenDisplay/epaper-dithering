import type { ImageBuffer, PaletteImageBuffer } from './types';
import { DitherMode } from './enums';
import type { Palette } from './palette';
import {
  initSync,
  dither_image as wasmDitherImage,
  composite_rgba as wasmCompositeRgba,
} from './wasm-core/epaper_dithering_wasm.js';
import wasmBytes from './wasm-core/epaper_dithering_wasm_bg.wasm';

// Synchronous WASM initialization from the inlined binary, once at module load time, via
// wasm-bindgen's own `initSync` (built with `wasm-pack --target web`).
initSync({ module: wasmBytes });

/**
 * Direct Binary Search refinement settings. DBS improves a dithered image by repeatedly
 * changing pixels to other inks or swapping them with neighbours whenever that lowers the
 * error as perceived through a model of the eye at the given viewing geometry.
 */
export interface DbsParams {
  /** Distance between viewer and panel in cm. Larger values blur more. Default: `40`. */
  viewingDistanceCm?: number;
  /** Panel pixel density. A 7.3" 800×480 panel is about 127 ppi. Default: `127`. */
  ppi?: number;
  /** Upper bound on full-image passes; stops early once nothing improves. Default: `10`. */
  maxPasses?: number;
}

/**
 * Options for `ditherImage`. All fields are optional — defaults are sensible.
 *
 * Pipeline (each step is a no-op at its identity value):
 * `exposure → saturation → shadows/highlights → tone → anchor → gamut → match + diffuse → dbs`.
 * Ink snapping is decided on the original image, before any of these steps.
 */
export interface DitherOptions {
  /** Dithering algorithm, as a `DitherMode` or its name. Default: `'burkes'`. */
  mode?: DitherMode | `${DitherMode}`;
  /** Alternate row scan direction for error diffusion. Default: `true`. */
  serpentine?: boolean;
  /** Linear-RGB exposure multiplier. 1.0 = no change, 2.0 = +1 stop. Default: `1.0`. */
  exposure?: number;
  /** OKLab saturation multiplier. 1.0 = no change, 0.0 = grayscale. Default: `1.0`. */
  saturation?: number;
  /** Shadow lift strength (S-curve lower half). 0.0 = off, 1.0 = strong. Default: `0.0`. */
  shadows?: number;
  /** Highlight compression strength (S-curve upper half). 0.0 = off, 1.0 = strong. Default: `0.0`. */
  highlights?: number;
  /** Black-point lift, `0` (off) to `1`: remaps lightness onto the panel's darkest and lightest colours, keeping hue. Default: `0`. */
  tone?: number;
  /** Gamut compression, `0` (off) to `1`: pulls out-of-gamut colours toward the palette's gamut. Default: `0`. */
  gamut?: number;
  /**
   * Direct Binary Search refinement after dithering: `false` disables, `true` uses default
   * {@link DbsParams}. Slow — seconds on a full panel, and it blocks the calling thread, so
   * run it in a Web Worker in the browser. Default: `false`.
   */
  dbs?: boolean | DbsParams;
  /** Map each named ink's nominal colour onto its appearance first; colours far from every ink are left alone. Default: `true`. */
  anchor?: boolean;
  /** A pixel exactly equal to a named ink's nominal takes that ink, undithered. Default: `true`. */
  snap?: boolean;
}

/**
 * Dither an RGBA image to a palette.
 *
 * @param image     Input RGBA image. Alpha is composited on white.
 * @param palette   The panel's palette.
 * @param options   Per-call overrides — see {@link DitherOptions}.
 * @returns Palette-indexed image buffer whose `palette` holds each entry's preview colour.
 */
export function ditherImage(
  image: ImageBuffer,
  palette: Palette,
  options: DitherOptions = {},
): PaletteImageBuffer {
  const {
    mode = DitherMode.BURKES,
    serpentine = true,
    exposure = 1.0,
    saturation = 1.0,
    shadows = 0.0,
    highlights = 0.0,
    tone = 0,
    gamut = 0,
    dbs = false,
    anchor = true,
    snap = true,
  } = options;
  const dbsArgs = parseDbs(dbs);

  const expectedLength = image.width * image.height * 4;
  if (image.data.length !== expectedLength) {
    throw new Error(
      `image data length (${image.data.length}) does not match width × height × 4 ` +
      `(${image.width} × ${image.height} × 4 = ${expectedLength})`,
    );
  }

  const rgba = new Uint8Array(image.data.buffer, image.data.byteOffset, image.data.byteLength);
  const pixels = wasmCompositeRgba(rgba);

  const indices = wasmDitherImage(
    pixels, image.width, palette.wasm,
    String(mode), serpentine,
    exposure, saturation, shadows, highlights,
    parseStrength('tone', tone), parseStrength('gamut', gamut),
    ...dbsArgs,
    anchor, snap,
  );

  return { width: image.width, height: image.height, indices, palette: palette.previewRgb() };
}

/**
 * Map the `dbs` option onto the binding's `(dbs_passes, viewing_distance_cm, ppi)` arguments;
 * `dbs_passes = undefined` disables DBS. Validated here because wasm-bindgen silently coerces
 * numbers into `u32` (e.g. `-1` → 4294967295).
 */
function parseDbs(dbs: boolean | DbsParams): [number | undefined, number, number] {
  if (dbs === false) return [undefined, 40, 127];
  const { viewingDistanceCm = 40, ppi = 127, maxPasses = 10 } = dbs === true ? {} : dbs;
  for (const [name, value] of [['viewingDistanceCm', viewingDistanceCm], ['ppi', ppi]] as const) {
    if (!(Number.isFinite(value) && value > 0)) {
      throw new Error(`dbs.${name} must be finite and greater than 0, got ${value}`);
    }
  }
  if (!(Number.isInteger(maxPasses) && maxPasses >= 0 && maxPasses <= 0xffffffff)) {
    throw new Error(`dbs.maxPasses must be a non-negative integer, got ${maxPasses}`);
  }
  return [maxPasses, viewingDistanceCm, ppi];
}

/** Validate a 0–1 strength option (wasm-bindgen would silently accept anything numeric). */
function parseStrength(name: string, value: unknown): number {
  if (typeof value !== 'number' || !(value >= 0 && value <= 1)) {
    throw new Error(`${name} must be a number from 0 to 1, got ${String(value)}`);
  }
  return value;
}

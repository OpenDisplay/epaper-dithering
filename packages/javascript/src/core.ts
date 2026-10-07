import type { ImageBuffer, PaletteImageBuffer, ColorPalette } from './types';
import { DitherMode } from './enums';
import { ColorScheme, getPalette } from './palettes';
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
 * `exposure → saturation → shadows/highlights → tone → gamut → dither → dbs`.
 */
export interface DitherOptions {
  /** Dithering algorithm. Default: `DitherMode.BURKES`. */
  mode?: DitherMode;
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
  /** Dynamic-range compression: `0.0`/`'off'` disables, `'auto'` opts in. Default: `0.0`. */
  tone?: number | 'auto' | 'off';
  /** Gamut compression: `0.0`/`'off'` disables, `'auto'` opts in. Default: `0.0`. */
  gamut?: number | 'auto' | 'off';
  /**
   * Direct Binary Search refinement after dithering: `false` disables, `true` uses default
   * {@link DbsParams}. Slow — seconds on a full panel, and it blocks the calling thread, so
   * run it in a Web Worker in the browser. Default: `false`.
   */
  dbs?: boolean | DbsParams;
}

/**
 * Apply dithering to an RGBA image for an e-paper display.
 *
 * @param image     Input RGBA image. Alpha is composited on white.
 * @param palette   Target palette: `ColorScheme` enum (idealized) or `ColorPalette` (measured).
 * @param options   Per-call overrides — see {@link DitherOptions}.
 * @returns Palette-indexed image buffer.
 */
export function ditherImage(
  image: ImageBuffer,
  palette: ColorScheme | ColorPalette,
  options: DitherOptions = {},
): PaletteImageBuffer {
  const {
    mode = DitherMode.BURKES,
    serpentine = true,
    exposure = 1.0,
    saturation = 1.0,
    shadows = 0.0,
    highlights = 0.0,
    tone = 0.0,
    gamut = 0.0,
    dbs = false,
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

  // Idealized schemes don't have a measured display range, so tone/gamut don't apply.
  const isScheme = typeof palette === 'number';
  const toneArg  = isScheme ? 0.0 : parseCompression(tone);
  const gamutArg = isScheme ? 0.0 : parseCompression(gamut);

  let schemeId: number | undefined = undefined;
  let paletteBytes: Uint8Array;
  let accentIdx = 0;
  let outputColors: { r: number; g: number; b: number }[];

  if (isScheme) {
    schemeId = palette as number;
    paletteBytes = new Uint8Array(0);
    outputColors = Object.values(getPalette(palette).colors);
  } else {
    schemeId = palette.scheme ?? undefined;
    const colors = Object.values(palette.colors);
    paletteBytes = new Uint8Array(colors.flatMap(c => [c.r, c.g, c.b]));
    accentIdx = Object.keys(palette.colors).indexOf(palette.accent);
    if (accentIdx < 0) {
      throw new Error(
        `accent color '${palette.accent}' not found in palette colors [${Object.keys(palette.colors).join(', ')}]`,
      );
    }
    outputColors = colors;
  }

  const indices = wasmDitherImage(
    pixels, image.width,
    schemeId, paletteBytes, accentIdx,
    mode as number, serpentine,
    exposure, saturation, shadows, highlights,
    toneArg, gamutArg,
    ...dbsArgs,
  );

  return { width: image.width, height: image.height, indices, palette: outputColors };
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

/** Map `'auto'` → `undefined` (Rust None = auto), `'off'` → 0.0, number → pass through. */
function parseCompression(v: number | 'auto' | 'off'): number | undefined {
  if (v === 'auto') return undefined;
  if (v === 'off')  return 0.0;
  return v;
}

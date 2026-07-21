import type { ImageBuffer, PaletteImageBuffer, ColorPalette } from './types';
import { DitherMode } from './enums';
import { ColorScheme, getPalette } from './palettes';
import {
  dither_image as wasmDitherImage,
  composite_rgba as wasmCompositeRgba,
  __wbg_set_wasm,
  __wbindgen_init_externref_table,
  __wbindgen_cast_0000000000000001,
} from './wasm-core/epaper_dithering_wasm_bg.js';
import wasmBytes from './wasm-core/epaper_dithering_wasm_bg.wasm';

// Synchronous WASM initialization — runs once at module load time.
const wasmModule = new WebAssembly.Module(wasmBytes as unknown as ArrayBuffer);
const wasmInstance = new WebAssembly.Instance(wasmModule, {
  './epaper_dithering_wasm_bg.js': {
    __wbindgen_init_externref_table,
    __wbindgen_cast_0000000000000001,
  },
});
__wbg_set_wasm(wasmInstance.exports);
(wasmInstance.exports as Record<string, () => void>).__wbindgen_start?.();

/**
 * Options for `ditherImage`. All fields are optional — defaults are sensible.
 *
 * Pre-processing pipeline (each step is a no-op at its identity value):
 * `exposure → saturation → shadows/highlights → tone → gamut → dither`.
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
  } = options;

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
  );

  return { width: image.width, height: image.height, indices, palette: outputColors };
}

/** Map `'auto'` → `undefined` (Rust None = auto), `'off'` → 0.0, number → pass through. */
function parseCompression(v: number | 'auto' | 'off'): number | undefined {
  if (v === 'auto') return undefined;
  if (v === 'off')  return 0.0;
  return v;
}

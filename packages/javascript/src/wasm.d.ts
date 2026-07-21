/** esbuild binary loader inlines .wasm files as Uint8Array */
declare module '*.wasm' {
  const bytes: Uint8Array;
  export default bytes;
}

/**
 * Hand-written types for the wasm-bindgen-generated `_bg.js` shim. wasm-pack only
 * emits `.d.ts` for the main module; the `_bg.js` exports we need for synchronous
 * initialization are declared here.
 */
declare module '*epaper_dithering_wasm_bg.js' {
  export function composite_rgba(rgba: Uint8Array): Uint8Array;
  export function dither_image(
    pixels: Uint8Array, width: number,
    scheme_id: number | undefined, palette_bytes: Uint8Array, accent_idx: number,
    mode_id: number, serpentine: boolean,
    exposure: number, saturation: number, shadows: number, highlights: number,
    tone?: number, gamut?: number,
  ): Uint8Array;
  export function measured_palettes(): string;
  export function __wbg_set_wasm(wasm: unknown): void;
  export function __wbindgen_init_externref_table(): void;
  export function __wbindgen_cast_0000000000000001(arg0: number, arg1: number): unknown;
}

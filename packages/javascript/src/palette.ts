import type { RGB } from './types';
import { WasmPalette } from './wasm-core/epaper_dithering_wasm.js';

/** One palette colour, in output-index order. */
export interface PaletteEntry {
  /** How the colour looks on the panel: CIELAB (D65) `[L*, a*, b*]`, normalised to the panel's white. */
  appearance: [number, number, number];
  /** Ink name, e.g. `"black"`. Only for pure inks; mixes leave it unset. */
  name?: string;
  /** The `#rrggbb` colour that requests this ink. Required exactly when `name` is set. */
  nominal?: string;
}

interface PaletteFile {
  format: 'epaper-palette';
  version: 1;
  measured: boolean;
  source?: string;
  entries: PaletteEntry[];
}

/**
 * A validated palette. All validation happens in the Rust core; constructors throw an
 * `Error` naming the problem (for an unnormalised white, it names `from_absolute_lab`).
 */
export class Palette {
  /** @internal */
  wasm!: WasmPalette;
  private file!: PaletteFile;

  /** Build from entries whose appearances are already normalised to the panel's white. */
  constructor(entries: PaletteEntry[], measured = false) {
    const file: PaletteFile = { format: 'epaper-palette', version: 1, measured, entries };
    this.init(WasmPalette.fromJson(JSON.stringify(file)));
  }

  /** Load an `epaper-palette` JSON document. */
  static fromJson(json: string): Palette {
    const palette = Object.create(Palette.prototype) as Palette;
    palette.init(WasmPalette.fromJson(json));
    return palette;
  }

  private init(wasm: WasmPalette): void {
    this.wasm = wasm;
    this.file = JSON.parse(wasm.toJson()) as PaletteFile;
  }

  /** Every entry, in output-index order. */
  get entries(): PaletteEntry[] {
    return this.file.entries.map((e) => ({ ...e, appearance: [...e.appearance] as [number, number, number] }));
  }

  /** Whether the appearances were measured on this unit (metadata only). */
  get measured(): boolean {
    return this.file.measured;
  }

  /** Free-text note of where the palette came from. */
  get source(): string | undefined {
    return this.file.source;
  }

  get length(): number {
    return this.wasm.length;
  }

  /** Output index of the ink with this name. */
  indexOf(name: string): number | undefined {
    return this.wasm.indexOf(name);
  }

  /** sRGB of each entry's appearance: what the panel shows, for previews. */
  previewRgb(): RGB[] {
    const flat = this.wasm.previewRgb();
    const out: RGB[] = [];
    for (let i = 0; i < flat.length; i += 3) out.push({ r: flat[i], g: flat[i + 1], b: flat[i + 2] });
    return out;
  }

  /** Serialise as an `epaper-palette` JSON document. */
  toJson(): string {
    return this.wasm.toJson();
  }
}

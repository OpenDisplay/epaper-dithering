# @opendisplay/epaper-dithering

[![npm](https://img.shields.io/npm/v/@opendisplay/epaper-dithering?style=flat-square)](https://www.npmjs.com/package/@opendisplay/epaper-dithering)

Dithering for e-paper/e-ink displays, powered by a Rust/WASM core: an image and your panel's palette
in, palette indices out. Works in the browser and in Node.js (≥18).

## Features

- **Rust/WASM core**, bundled inline: no async init, no external files.
- **Palettes are data**: describe your panel in an `epaper-palette` JSON file (or build one in code). The library ships no built-in palettes, so any panel works, including per-unit measurements.
- **Requested inks stay solid**: a pixel that asks for an ink (e.g. `#000000` text) comes out as that ink, undithered, even though the panel's real black is lighter.
- **OKLab colour matching**: weighted Cartesian OKLab, which preserves hue without the achromatic-attractor bug of LCH-weighted approaches.
- **10 dithering algorithms**, from ordered dithering to Jarvis-Judice-Ninke and Dizzy.
- **Pre-dither adjustments**: exposure, saturation, shadows, highlights, dynamic-range (tone) compression and gamut compression.
- **DBS refinement**: an optional Direct Binary Search pass that optimises the result for how it is perceived.

## Installation

```bash
npm install @opendisplay/epaper-dithering
# or
bun add @opendisplay/epaper-dithering
```

## Quick Start

### Browser (Canvas API)

```typescript
import { ditherImage, Palette } from '@opendisplay/epaper-dithering';

const palette = Palette.fromJson(await (await fetch('/my_panel.json')).text());

const img = new Image();
img.src = 'photo.jpg';
await img.decode();

const canvas = document.createElement('canvas');
canvas.width = img.width;
canvas.height = img.height;
const ctx = canvas.getContext('2d')!;
ctx.drawImage(img, 0, 0);
const imageData = ctx.getImageData(0, 0, img.width, img.height);

const dithered = ditherImage(
  { width: imageData.width, height: imageData.height, data: imageData.data },
  palette,
  { mode: 'floyd_steinberg' },
);

// Render result in the panel's preview colours
const out = ctx.createImageData(dithered.width, dithered.height);
for (let i = 0; i < dithered.indices.length; i++) {
  const c = dithered.palette[dithered.indices[i]];
  out.data[i * 4] = c.r; out.data[i * 4 + 1] = c.g;
  out.data[i * 4 + 2] = c.b; out.data[i * 4 + 3] = 255;
}
ctx.putImageData(out, 0, 0);
```

### Node.js (with sharp)

```typescript
import { readFileSync } from 'node:fs';
import sharp from 'sharp';
import { ditherImage, Palette } from '@opendisplay/epaper-dithering';

const palette = Palette.fromJson(readFileSync('my_panel.json', 'utf8'));

const { data, info } = await sharp('photo.jpg')
  .ensureAlpha()
  .raw()
  .toBuffer({ resolveWithObject: true });

const dithered = ditherImage(
  { width: info.width, height: info.height, data: new Uint8ClampedArray(data) },
  palette,
);
// dithered.indices: one palette index per pixel, in palette entry order
```

## Palettes

A palette is an ordered list of entries; the position of an entry is the index written to the
output. Each entry has an `appearance` (CIELAB D65 `[L*, a*, b*]`, **normalised so the panel's white
is L\* 100, a\* = b\* = 0**) and, for pure inks only, a `name` and the `nominal` colour that requests
it.

```typescript
import { Palette } from '@opendisplay/epaper-dithering';

// From an epaper-palette JSON document
const fromFile = Palette.fromJson(json);

// From entries in code
const palette = new Palette([
  { appearance: [100, 0, 0], name: 'white', nominal: '#ffffff' },
  { appearance: [18.2, 1.1, -0.4], name: 'black', nominal: '#000000' },
  { appearance: [52.0, 8.9, 2.1] },             // an unnamed mix
], /* measured */ true);

palette.length;         // 3
palette.indexOf('black'); // 1
palette.previewRgb();   // [{ r, g, b }, ...] what the panel shows, per entry
palette.toJson();       // save it as a palette file
```

From measurements, use `Palette.fromAbsoluteLab(entries, whiteIndex, measured)` (absolute CIELAB,
normalised so `entries[whiteIndex]` becomes L\* 100) or `Palette.fromSrgb([{ rgb: { r, g, b }, name?, nominal? }, ...])`.

Constructors throw on invalid palettes: fewer than 2 or more than 256 entries, an appearance that is
not finite, duplicate ink names or nominals, malformed files, or a `white` entry more than ΔE 0.5
from L\* 100 (the message tells you to normalise with `from_absolute_lab`, i.e. `fromAbsoluteLab`). The repository's
[`examples/palettes/`](../../examples/palettes/) folder has starting points (the idealised 6.x
schemes and several measured panels); they are not part of the package. The file format is
described in the [Python README](../python/README.md#palettes).

**Named inks** drive two options, both on by default:

- `snap`: a pixel exactly equal to an ink's `nominal` takes that ink, undithered and excluded from
  error diffusion. Decided on the original image, before any adjustment.
- `anchor`: colours are mapped so each ink's nominal lands on its appearance (black-point
  compensation plus a local correction around each ink), so near-ink colours follow smoothly and
  photos keep their shadows.

## API Reference

### `ditherImage(image, palette, options?)`

```typescript
ditherImage(image: ImageBuffer, palette: Palette, options?: DitherOptions): PaletteImageBuffer
```

| `options` field | Type | Default | Description |
|---|---|---|---|
| `mode` | `DitherMode \| string` | `'burkes'` | Dithering algorithm (see below) |
| `serpentine` | `boolean` | `true` | Alternate row direction to reduce artifacts |
| `exposure` | `number` | `1.0` | Linear-RGB exposure multiplier. `2.0` = +1 stop, `0.5` = −1 stop |
| `saturation` | `number` | `1.0` | OKLab saturation multiplier. `0.0` = greyscale, `>1` = boost |
| `shadows` | `number` | `0.0` | Shadow lift strength. `0.0` = off, `1.0` = strong |
| `highlights` | `number` | `0.0` | Highlight compression strength. `0.0` = off, `1.0` = strong |
| `tone` | `number \| 'auto' \| 'off'` | `0.0` | Dynamic-range compression |
| `gamut` | `number \| 'auto' \| 'off'` | `0.0` | Gamut compression |
| `dbs` | `boolean \| DbsParams` | `false` | Direct Binary Search refinement. `true` = default parameters |
| `anchor` | `boolean` | `true` | Map ink nominals onto their appearances before matching |
| `snap` | `boolean` | `true` | Pure ink colours take that ink, undithered |

Pipeline: `exposure → saturation → shadows/highlights → anchor → tone → gamut → match + diffuse → dbs`.
Anchoring already brings black and white into the panel's range, so `tone: 'auto'` usually finds
nothing left to compress.

Returns a `PaletteImageBuffer` whose `palette` holds each entry's preview colour. Throws if
`image.data.length` is not `width × height × 4`, on an unknown mode name, or on invalid DBS
parameters.

#### DBS refinement

DBS repeatedly changes pixels to other inks, or swaps them with neighbours, whenever that lowers the
error as perceived through a model of the eye. It is much slower than plain dithering (seconds on a
full panel in a single-threaded browser) and blocks the calling thread, so run it in a Web Worker.

```typescript
// Default geometry: 40 cm from a ~127 ppi panel (7.3" 800×480)
ditherImage(imageBuffer, palette, { dbs: true });

// Match your setup
ditherImage(imageBuffer, palette, { dbs: { viewingDistanceCm: 150, ppi: 150, maxPasses: 10 } });
```

| `DbsParams` field | Default | Description |
|---|---|---|
| `viewingDistanceCm` | `40` | Distance between viewer and panel. Larger values blur more |
| `ppi` | `127` | Panel pixel density |
| `maxPasses` | `10` | Upper bound on refinement passes; stops early once nothing improves |

Snapped pixels are never changed by DBS.

### Dither modes

`DitherMode` is a string enum; you can pass either the member or its name.

| Mode | Notes |
|---|---|
| `'none'` | Nearest colour, no diffusion. For already-quantised graphics; flat mid-tones can land on unexpected inks |
| `'ordered'` | Bayer 4×4 |
| `'burkes'` | Default, general purpose |
| `'floyd_steinberg'`, `'atkinson'`, `'stucki'`, `'sierra'`, `'sierra_lite'`, `'jarvis_judice_ninke'` | Error diffusion kernels |
| `'dizzy'` | Pseudo-random traversal, no directional structure; ignores `serpentine` |

### Types

```typescript
interface ImageBuffer {
  width: number;
  height: number;
  data: Uint8ClampedArray; // RGBA, row-major
}

interface PaletteImageBuffer {
  width: number;
  height: number;
  indices: Uint8Array; // palette index per pixel
  palette: RGB[];      // preview colour per palette entry
}

interface PaletteEntry {
  appearance: [number, number, number]; // CIELAB, normalised to the panel's white
  name?: string;
  nominal?: string;                     // '#rrggbb', required with name
}
```

## Migrating from 6.x

| 6.x | 7.x |
|---|---|
| `ditherImage(img, ColorScheme.BWR)` | `ditherImage(img, Palette.fromJson(idealBwrJson))` (from `examples/palettes/ideal_bwr.json`) |
| `SPECTRA_7_3_6COLOR`, `SPECTRA_7_3_6COLOR_V2` | `examples/palettes/spectra_7_3_6color*.json`; the other 6.x "measured" constants were unmeasured placeholders and are gone |
| `ColorPalette` objects | `new Palette(entries)` |
| `DitherMode.BURKES === 1` | `DitherMode.BURKES === 'burkes'` |
| `getPalette`, `getColorCount`, `fromValue` | `palette.previewRgb()`, `palette.length` |

The 6.x measured constants were relative to the camera, not to the panel's white; the exported files
are normalised, so the same panel now renders brighter and keeps its shadows.

## Preview Tool

An interactive browser tool for comparing dithering modes and palettes.

**Local** (against your working branch, using the repo's example palettes):
```bash
cd packages/javascript
bun run dev
# → http://localhost:3456/packages/javascript/dev.html
```

## Development

```bash
bun install

# When Rust source changes, rebuild the WASM (from repo root):
wasm-pack build packages/rust/wasm --target web --out-dir ../../javascript/src/wasm-core

bun run test        # vitest (needs Node ≥18)
bun run build       # tsup → dist/
bun run type-check
```

## Related Projects

- **Python**: [`epaper-dithering`](https://pypi.org/project/epaper-dithering/) — Python package, shares the same Rust core

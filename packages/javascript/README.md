# @opendisplay/epaper-dithering

[![npm](https://img.shields.io/npm/v/@opendisplay/epaper-dithering?style=flat-square)](https://www.npmjs.com/package/@opendisplay/epaper-dithering)

High-quality dithering algorithms for e-paper/e-ink displays, powered by a Rust/WASM core. Works in both browser and Node.js environments.

## Features

- **Rust/WASM Core**: Compiled Rust logic bundled inline — no async init, no external files, works everywhere
- **9 Dithering Algorithms**: From fast ordered dithering to high-quality error diffusion
- **9 Color Schemes**: MONO, BWR, BWY, BWRY, BWGBRY (Spectra 6), GRAYSCALE\_4, GRAYSCALE\_16, SEVEN\_COLOR (Spectra/ACeP 7), BWGBRY\_SPLIT
- **Measured Palettes**: Use real display-calibrated colors for accurate dithering (SPECTRA\_7\_3\_6COLOR\_V2, BWRY\_3\_97, and more)
- **OKLab Color Matching**: Weighted Cartesian OKLab — preserves hue without the achromatic-attractor bug that plagues LCH-weighted approaches
- **Pre-dither Adjustments**: Per-image exposure, saturation, shadows, highlights, dynamic-range compression, and gamut compression — all orthogonal knobs
- **Serpentine Scanning**: Alternates row direction to eliminate directional artifacts
- **Universal**: Works in browser (Canvas API) and Node.js (≥18)
- **Zero Dependencies**: WASM binary bundled inline, no image library required

## Installation

```bash
npm install @opendisplay/epaper-dithering
# or
bun add @opendisplay/epaper-dithering
```

## Quick Start

### Browser (Canvas API)

```typescript
import { ditherImage, ColorScheme, DitherMode } from '@opendisplay/epaper-dithering';

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
  ColorScheme.BWR,
  { mode: DitherMode.FLOYD_STEINBERG },
);

// Render result
const out = ctx.createImageData(dithered.width, dithered.height);
for (let i = 0; i < dithered.indices.length; i++) {
  const c = dithered.palette[dithered.indices[i]];
  out.data[i * 4] = c.r; out.data[i * 4 + 1] = c.g;
  out.data[i * 4 + 2] = c.b; out.data[i * 4 + 3] = 255;
}
ctx.putImageData(out, 0, 0);
```

### Measured Palettes

Standard `ColorScheme` values use ideal sRGB colors (e.g. white = 255,255,255). Real e-paper displays reflect significantly less light. Use a measured `ColorPalette` for accurate dithering:

```typescript
import { ditherImage, SPECTRA_7_3_6COLOR, BWRY_3_97 } from '@opendisplay/epaper-dithering';

const dithered = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR, { mode: DitherMode.BURKES });

// Opt in when you want automatic tone/gamut compression for photos
const compressed = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR, {
  mode: DitherMode.BURKES,
  tone: 'auto',
  gamut: 'auto',
});
```

Available measured palettes: `SPECTRA_7_3_6COLOR_V2`, `SPECTRA_7_3_6COLOR`, `BWRY_3_97`, `MONO_4_26`, `BWRY_4_2`, `SOLUM_BWR`, `HANSHOW_BWR`, `HANSHOW_BWY`.

### Node.js (with sharp)

```typescript
import sharp from 'sharp';
import { ditherImage, ColorScheme, DitherMode } from '@opendisplay/epaper-dithering';

const { data, info } = await sharp('photo.jpg')
  .ensureAlpha()
  .raw()
  .toBuffer({ resolveWithObject: true });

const dithered = ditherImage(
  { width: info.width, height: info.height, data: new Uint8ClampedArray(data) },
  ColorScheme.BWR,
  { mode: DitherMode.BURKES },
);

const rgbaBuffer = Buffer.alloc(dithered.width * dithered.height * 4);
for (let i = 0; i < dithered.indices.length; i++) {
  const c = dithered.palette[dithered.indices[i]];
  rgbaBuffer[i * 4] = c.r; rgbaBuffer[i * 4 + 1] = c.g;
  rgbaBuffer[i * 4 + 2] = c.b; rgbaBuffer[i * 4 + 3] = 255;
}

await sharp(rgbaBuffer, { raw: { width: dithered.width, height: dithered.height, channels: 4 } })
  .png()
  .toFile('dithered.png');
```

## API Reference

### `ditherImage(image, colorScheme, options?)`

```typescript
ditherImage(image: ImageBuffer, palette: ColorScheme | ColorPalette, options?: DitherOptions): PaletteImageBuffer
```

| `options` field | Type | Default | Description |
|---|---|---|---|
| `mode` | `DitherMode` | `BURKES` | Dithering algorithm |
| `serpentine` | `boolean` | `true` | Alternate row direction to reduce artifacts |
| `exposure` | `number` | `1.0` | Linear-RGB exposure multiplier. `2.0` = +1 stop, `0.5` = −1 stop |
| `saturation` | `number` | `1.0` | OKLab saturation multiplier. `0.0` = grayscale, `>1` = boost. Hue-preserving |
| `shadows` | `number` | `0.0` | Shadow lift strength (S-curve lower half). `0.0` = off, `1.0` = strong |
| `highlights` | `number` | `0.0` | Highlight compression strength (S-curve upper half). `0.0` = off, `1.0` = strong |
| `tone` | `number \| 'auto' \| 'off'` | `0.0` | Dynamic range compression. `0.0`/`'off'` = disabled; `'auto'` = histogram-based; numeric = fixed strength. Ignored for `ColorScheme` |
| `gamut` | `number \| 'auto' \| 'off'` | `0.0` | Pre-dither gamut compression. `0.0`/`'off'` = disabled; `'auto'` = activate when image exceeds palette gamut; numeric = fixed. Ignored for `ColorScheme` |

Pre-processing pipeline: `exposure → saturation → shadows/highlights → tone → gamut → dither`. Each step is a no-op at its identity value.

`DitherMode.NONE` performs direct nearest-color mapping without error diffusion or ordered dithering. It is intended for already-quantized graphics, not continuous-tone photos: because there is no error diffusion, on limited palettes (especially BWR) a continuous-tone image or a large flat mid-tone area can map to an unexpected ink — for example, a solid mid-gray region can render as solid red. Use an error-diffusion mode (e.g. `FLOYD_STEINBERG`, `BURKES`) for photographic input. Built-in measured palettes carry their canonical firmware `scheme`, so pure display colors map to the corresponding firmware palette index even when measured RGB values are used for matching.

For built-in measured palettes, exact canonical display colors are also protected in ordered and error-diffusion modes when pre-processing is off: an image made entirely of display colors is returned as a direct palette-index map, and exact display-color pixels inside a mixed image keep their canonical index instead of being rematched to the measured RGB palette. Pre-processing runs before that exact-pixel check, so explicit `tone: 'auto'`, `gamut: 'auto'`, or other adjustments may intentionally alter those pixels first.

Returns `PaletteImageBuffer`.

### Color Schemes

```typescript
enum ColorScheme {
  MONO         = 0,  // Black & White (2 colors)
  BWR          = 1,  // Black, White, Red (3 colors)
  BWY          = 2,  // Black, White, Yellow (3 colors)
  BWRY         = 3,  // Black, White, Yellow, Red (4 colors)
  BWGBRY       = 4,  // Black, White, Yellow, Red, Blue, Green (6 colors)
  GRAYSCALE_4  = 5,  // 4-level grayscale
  GRAYSCALE_16 = 6,  // 16-level grayscale
  SEVEN_COLOR  = 7,  // Spectra/ACeP 7-color (BWGBRY + orange)
  BWGBRY_SPLIT = 8,  // Spectra 6, split left/right plane packing
}
```

### Dither Modes

| Mode | Quality | Speed | Notes |
|---|---|---|---|
| `NONE` | — | Fastest | Direct palette mapping |
| `ORDERED` | Low | Very fast | 4×4 Bayer matrix |
| `SIERRA_LITE` | Medium | Fast | 3-neighbor kernel |
| `FLOYD_STEINBERG` | Good | Medium | Most popular |
| `BURKES` | Good | Medium | **Default** |
| `ATKINSON` | Good | Medium | Classic Mac aesthetic |
| `SIERRA` | High | Medium | — |
| `STUCKI` | Very high | Slow | — |
| `JARVIS_JUDICE_NINKE` | Highest | Slowest | — |

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
  palette: RGB[];      // sRGB colors
}

interface ColorPalette {
  readonly colors: Record<string, RGB>;
  readonly accent: string;
  readonly scheme?: number;
}
```

## Preview Tool

An interactive browser tool for comparing dithering modes and palettes:

**Hosted** (always latest release): https://opendisplay.github.io/epaper-dithering/

**Local** (against your working branch):
```bash
cd packages/javascript
bun run dev
# → http://localhost:3456/dev.html
```

Features: drag & drop or paste from clipboard, live re-render on every setting change, timing display, palette swatch preview.

## Development

```bash
bun install

# When Rust source changes, rebuild the WASM (from repo root):
wasm-pack build packages/rust/wasm --target bundler --out-dir ../../javascript/src/wasm-core

bun run test        # vitest
bun run build       # tsup → dist/
bun run type-check
```

## Related Projects

- **Python**: [`epaper-dithering`](https://pypi.org/project/epaper-dithering/) — Python package, shares the same Rust core
- **OpenDisplay**: [`py-opendisplay`](https://github.com/OpenDisplay-org/py-opendisplay) — Python library for OpenDisplay BLE devices

# @opendisplay/epaper-dithering

[![npm](https://img.shields.io/npm/v/@opendisplay/epaper-dithering?style=flat-square)](https://www.npmjs.com/package/@opendisplay/epaper-dithering)

High-quality dithering algorithms for e-paper/e-ink displays, implemented in TypeScript. Works in both browser and Node.js environments.

## Features

- **9 Dithering Algorithms**: From fast ordered dithering to high-quality error diffusion
- **8 Color Schemes**: MONO, BWR, BWY, BWRY, BWGBRY (Spectra 6), GRAYSCALE\_4/8/16
- **Measured Palettes**: Use real display-calibrated colors for accurate dithering (SPECTRA\_7\_3\_6COLOR, BWRY\_3\_97, and more)
- **LCH Color Matching**: Perceptual LAB color space with hue-weighted distance — hue errors can't be recovered by error diffusion, so they're prioritized
- **Serpentine Scanning**: Alternates row direction to eliminate directional artifacts
- **Universal**: Works in browser (Canvas API) and Node.js (with sharp/jimp)
- **Zero Dependencies**: Pure TypeScript, no image library dependencies
- **Fast**: 256-entry sRGB LUT, pre-computed palette LAB arrays, typed array pixel buffers

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
  DitherMode.FLOYD_STEINBERG,
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

// Automatically applies tone compression to fit the display's actual dynamic range
const dithered = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR, DitherMode.BURKES);
```

Available measured palettes: `SPECTRA_7_3_6COLOR`, `BWRY_3_97`, `MONO_4_26`, `BWRY_4_2`, `SOLUM_BWR`, `HANSHOW_BWR`, `HANSHOW_BWY`.

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
  DitherMode.BURKES,
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

### `ditherImage(image, colorScheme, mode?, serpentine?)`

| Parameter | Type | Default | Description |
|---|---|---|---|
| `image` | `ImageBuffer` | — | RGBA input image |
| `colorScheme` | `ColorScheme \| ColorPalette` | — | Target palette (enum or measured) |
| `mode` | `DitherMode` | `BURKES` | Dithering algorithm |
| `serpentine` | `boolean` | `true` | Alternate row direction to reduce artifacts |

Returns `PaletteImageBuffer`.

### Color Schemes

```typescript
enum ColorScheme {
  MONO         = 0,  // Black & White (2 colors)
  BWR          = 1,  // Black, White, Red (3 colors)
  BWY          = 2,  // Black, White, Yellow (3 colors)
  BWRY         = 3,  // Black, White, Red, Yellow (4 colors)
  BWGBRY       = 4,  // Black, White, Green, Blue, Red, Yellow (6 colors)
  GRAYSCALE_4  = 5,  // 4-level grayscale
  GRAYSCALE_8  = 6,  // 8-level grayscale
  GRAYSCALE_16 = 7,  // 16-level grayscale
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
}
```

## Local Development / Preview

A browser-based preview tool is included at [`dev.html`](./dev.html).

```bash
cd packages/javascript
bun run dev
# opens http://localhost:3456/dev.html
```

Features: 
- drag & drop or paste from clipboard
- live re-render on setting change
- timing display
- palette swatch preview.

## Related Projects

- **Python**: [`epaper-dithering`](https://pypi.org/project/epaper-dithering/) — Python implementation (feature superset)
- **OpenDisplay**: [`py-opendisplay`](https://github.com/OpenDisplay-org/py-opendisplay) — Python library for OpenDisplay BLE devices

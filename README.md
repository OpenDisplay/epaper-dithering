# epaper-dithering

[![PyPI](https://img.shields.io/pypi/v/epaper-dithering?style=flat-square)](https://pypi.org/project/epaper-dithering/)
[![npm](https://img.shields.io/npm/v/@opendisplay/epaper-dithering?style=flat-square)](https://www.npmjs.com/package/@opendisplay/epaper-dithering)
[![Python Tests](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-test.yml?style=flat-square&label=tests)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-test.yml)
[![Python Lint](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-lint.yml?style=flat-square&label=lint)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-lint.yml)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json&style=flat-square)](https://github.com/astral-sh/ruff)
[![mypy](https://img.shields.io/badge/mypy-strict-blue?style=flat-square)](https://mypy.readthedocs.io/)

A monorepo containing dithering algorithm implementations for e-paper/e-ink displays in multiple languages.

## Packages

### Python (`packages/python/`)

Dithering algorithms for e-paper displays. Published to PyPI as `epaper-dithering`.

**Installation:**
```bash
pip install epaper-dithering
```

**Usage:**
```python
from PIL import Image
from epaper_dithering import dither_image, ColorScheme, DitherMode

img = Image.open("photo.jpg")
dithered = dither_image(img, ColorScheme.BWR, DitherMode.FLOYD_STEINBERG)
dithered.save("output.png")
```

See [`packages/python/README.md`](packages/python/README.md) for detailed documentation.

### JavaScript/TypeScript (`packages/javascript/`)

Dithering algorithms for e-paper displays in TypeScript. Published to npm as `@opendisplay/epaper-dithering`.

**Installation:**
```bash
npm install @opendisplay/epaper-dithering
# or
bun add @opendisplay/epaper-dithering
```

**Usage (Browser):**
```typescript
import { ditherImage, ColorScheme, DitherMode } from '@opendisplay/epaper-dithering';

// Create ImageBuffer from Canvas
const canvas = document.createElement('canvas');
const ctx = canvas.getContext('2d')!;
ctx.drawImage(img, 0, 0);
const imageData = ctx.getImageData(0, 0, img.width, img.height);

const imageBuffer = {
  width: imageData.width,
  height: imageData.height,
  data: imageData.data,
};

const dithered = ditherImage(imageBuffer, ColorScheme.BWR, DitherMode.FLOYD_STEINBERG);
```

**Usage (Node.js with sharp):**
```typescript
import sharp from 'sharp';
import { ditherImage, ColorScheme, DitherMode } from '@opendisplay/epaper-dithering';

const { data, info } = await sharp('photo.jpg')
  .ensureAlpha()
  .raw()
  .toBuffer({ resolveWithObject: true });

const imageBuffer = {
  width: info.width,
  height: info.height,
  data: new Uint8ClampedArray(data),
};

const dithered = ditherImage(imageBuffer, ColorScheme.BWR, DitherMode.BURKES);
```

See [`packages/javascript/README.md`](packages/javascript/README.md) for detailed documentation.

## Features

- **9 Dithering Algorithms**: NONE, BURKES, ORDERED, FLOYD_STEINBERG, ATKINSON, STUCKI, SIERRA, SIERRA_LITE, JARVIS_JUDICE_NINKE
- **6 Color Schemes**: MONO, BWR, BWY, BWRY, BWGBRY (Spectra 6), GRAYSCALE_4
- **Multiple Languages**: Python and JavaScript/TypeScript implementations
- **Universal**: Works in Python, Node.js, and browser environments
- **Type-Safe**: Full type coverage in both languages

## Development

### Python Development

```bash
cd packages/python
uv sync --all-extras
uv run pytest tests/ -v
uv run ruff check src/ tests/
uv run mypy src/epaper_dithering
```

### JavaScript Development

```bash
cd packages/javascript
bun install
bun test
bun run build
```

### Repository Structure

```
epaper-dithering/
├── packages/
│   ├── python/          # Python implementation
│   │   ├── src/
│   │   ├── tests/
│   │   └── pyproject.toml
│   └── javascript/      # TypeScript/JavaScript implementation
│       ├── src/
│       ├── tests/
│       └── package.json
├── fixtures/            # Shared test fixtures
│   ├── images/          # Input test images
│   └── expected/        # Expected dithered outputs
├── .github/
│   └── workflows/       # CI/CD for both packages
├── docs/                # Shared documentation
└── README.md
```

## Related Projects

- **py-opendisplay**: [Python library for OpenDisplay BLE e-paper devices](https://github.com/OpenDisplay-org/py-opendisplay)

## Future Plans

- Add s-curve tone mapping for better contrast
- Rust implementation with bindings for Python/JavaScript/other languages
- Web-based demo/playground


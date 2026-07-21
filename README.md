# epaper-dithering

[![PyPI](https://img.shields.io/pypi/v/epaper-dithering?style=flat-square)](https://pypi.org/project/epaper-dithering/)
[![npm](https://img.shields.io/npm/v/@opendisplay/epaper-dithering?style=flat-square)](https://www.npmjs.com/package/@opendisplay/epaper-dithering)
[![Python Tests](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-test.yml?style=flat-square&label=python)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-test.yml)
[![JS Tests](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/javascript-test.yml?style=flat-square&label=javascript)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/javascript-test.yml)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json&style=flat-square)](https://github.com/astral-sh/ruff)

Dithering algorithms for e-paper/e-ink displays. Rust core with Python and JavaScript/TypeScript bindings.

## Examples

![Frankfurt at night — original vs Spectra 6-color with auto tone + gamut compression](docs/examples/frankfurt_before_after.png)

**Dithering algorithms** (Spectra 6-color, Marienplatz U-Bahn station):

![All dithering algorithms compared](docs/examples/algorithms_grid.png)

**Color schemes** (Burkes, river scene):

![All color schemes compared](docs/examples/color_schemes_grid.png)

## Packages

### Python (`packages/python/`)

Published to PyPI as [`epaper-dithering`](https://pypi.org/project/epaper-dithering/).

```bash
pip install epaper-dithering
```

```python
from PIL import Image
from epaper_dithering import dither_image, ColorScheme, DitherMode, SPECTRA_7_3_6COLOR_V2

img = Image.open("photo.jpg")

# Idealized palette
dithered = dither_image(img, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG)

# Measured palette — calibrated color matching
dithered = dither_image(img, SPECTRA_7_3_6COLOR_V2)

# Opt in to automatic tone + gamut compression for photos
dithered = dither_image(img, SPECTRA_7_3_6COLOR_V2, tone="auto", gamut="auto")
```

See [`packages/python/README.md`](packages/python/README.md) for full documentation.

### JavaScript/TypeScript (`packages/javascript/`)

Published to npm as [`@opendisplay/epaper-dithering`](https://www.npmjs.com/package/@opendisplay/epaper-dithering).

```bash
npm install @opendisplay/epaper-dithering
```

```typescript
import { ditherImage, ColorScheme, DitherMode, SPECTRA_7_3_6COLOR_V2 } from '@opendisplay/epaper-dithering';

// ImageBuffer from Canvas API or Node.js (sharp, etc.)
const dithered = ditherImage(imageBuffer, ColorScheme.BWR, { mode: DitherMode.BURKES });

// Measured palette — calibrated color matching
const dithered = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR_V2);

// Opt in to automatic tone + gamut compression for photos
const compressed = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR_V2, { tone: 'auto', gamut: 'auto' });
```

See [`packages/javascript/README.md`](packages/javascript/README.md) for full documentation.

## Features

- **Rust Core**: All dithering logic in `packages/rust/core/` — shared by both packages
- **9 Dithering Algorithms**: NONE, ORDERED, BURKES, FLOYD_STEINBERG, ATKINSON, STUCKI, SIERRA, SIERRA_LITE, JARVIS_JUDICE_NINKE
- **9 Color Schemes**: MONO, BWR, BWY, BWRY, BWGBRY (Spectra 6), GRAYSCALE_4, GRAYSCALE_16, SEVEN_COLOR (Spectra/ACeP 7), BWGBRY_SPLIT
- **Measured Palettes**: Calibrated RGB values for real displays, linked to their canonical firmware palette
- **OKLab Color Matching**: Weighted Cartesian OKLab — preserves hue without the achromatic-attractor bug of LCH-weighted approaches
- **Pre-dither Knobs**: Per-image exposure, saturation, shadows, highlights, and gamut compression — all orthogonal

## Repository Structure

```
epaper-dithering/
├── packages/
│   ├── rust/
│   │   ├── core/        # Shared Rust algorithms (OKLab, dithering, tone/gamut)
│   │   └── wasm/        # wasm-bindgen bindings for JavaScript
│   ├── python/          # PyO3/maturin extension + Python wrapper
│   └── javascript/      # TypeScript wrapper + bundled WASM
├── docs/                # Calibration and color science documentation
└── .github/workflows/   # CI for all packages
```

## Development

```bash
# Python (requires Rust toolchain: https://rustup.rs)
cd packages/python
uv sync --all-extras
uv run maturin develop --release
uv run pytest tests/ -v

# JavaScript
cd packages/javascript
bun install
bun run test

# Rust
cd packages/rust
cargo test --workspace
```

## Related Projects

- **py-opendisplay**: [Python library for OpenDisplay BLE e-paper devices](https://github.com/OpenDisplay-org/py-opendisplay)

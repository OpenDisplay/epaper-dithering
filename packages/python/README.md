# epaper-dithering

[![PyPI](https://img.shields.io/pypi/v/epaper-dithering?style=flat-square)](https://pypi.org/project/epaper-dithering/)
[![Python](https://img.shields.io/pypi/pyversions/epaper-dithering?style=flat-square)](https://pypi.org/project/epaper-dithering/)
[![License](https://img.shields.io/github/license/OpenDisplay-org/epaper-dithering?style=flat-square)](LICENSE)
[![Tests](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-test.yml?style=flat-square&label=tests)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-test.yml)
[![Lint](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-lint.yml?style=flat-square&label=lint)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-lint.yml)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json&style=flat-square)](https://github.com/astral-sh/ruff)
[![mypy](https://img.shields.io/badge/mypy-strict-blue?style=flat-square)](https://mypy.readthedocs.io/)

Dithering algorithms optimized for e-ink/e-paper displays with limited color palettes.

## Installation

```bash
# With uv
uv add epaper-dithering

# With pip
pip install epaper-dithering
```

## Features

- **Rust Core**: All dithering runs in a compiled Rust extension — fast enough for 800×480 images in ~30ms
- **Perceptually Correct**: Weighted Cartesian OKLab color matching — preserves hue without the achromatic-attractor bug that plagues LCH-weighted approaches
- **10 Dithering Algorithms**: From simple ordered dithering to high-quality Jarvis-Judice-Ninke
- **8 Color Schemes**: Support for mono, 3-color, 4-color, 6-color, and grayscale e-paper displays
- **Pre-dither Adjustments**: Per-image exposure, saturation, shadows, highlights, dynamic-range compression, and gamut compression — all orthogonal knobs you can mix freely
- **Serpentine Scanning**: Reduces directional artifacts in error diffusion (enabled by default)
- **RGBA Support**: Automatic compositing on white background for transparent images

## Quick Start

```python
from PIL import Image
from epaper_dithering import dither_image, ColorScheme, DitherMode

# Load your image
image = Image.open("photo.jpg")

# Apply dithering for a black/white/red display
dithered = dither_image(image, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG)

# Save result
dithered.save("output.png")
```

All arguments after `color_scheme` are keyword-only:

```python
dither_image(
    image, palette,
    *,
    mode=DitherMode.BURKES,    # algorithm
    serpentine=True,           # alternate row scan direction
    exposure=1.0,              # linear-RGB multiplier (1.0 = no change)
    saturation=1.0,            # OKLab saturation (1.0 = no change, 0.0 = grayscale)
    shadows=0.0,               # shadow lift, S-curve lower half
    highlights=0.0,            # highlight compression, S-curve upper half
    tone="auto",               # dynamic-range compression: "auto" | 0.0–1.0
    gamut="auto",              # gamut compression: "auto" | 0.0–1.0
)
```

## Supported Color Schemes

- **MONO** - Black and white (1-bit)
- **BWR** - Black, white, red (3-color)
- **BWY** - Black, white, yellow (3-color)
- **BWRY** - Black, white, yellow, red (4-color)
- **BWGBRY** - Black, white, yellow, red, blue, green (6-color Spectra)
- **GRAYSCALE_4** - 4-level grayscale (2-bit)
- **SEVEN_COLOR** - 7-color Spectra/ACeP (BWGBRY + orange)
- **BWGBRY_SPLIT** - Spectra 6, split left/right plane packing (dual-CS panels)
- **GRAYSCALE_16** - 16-level grayscale (4-bit, e.g. Waveshare 6" HD)

## Dithering Algorithms

| Algorithm | Quality | Speed | Best For |
|-----------|---------|-------|----------|
| NONE | Lowest | Fastest | Testing, simple graphics |
| ORDERED | Low | Very Fast | Patterns, textures |
| SIERRA_LITE | Medium | Fast | Quick results |
| BURKES | Good | Medium | General purpose (default) |
| FLOYD_STEINBERG | Good | Medium | Popular standard |
| SIERRA | High | Medium | Balanced quality |
| ATKINSON | Good | Medium | High contrast, artistic |
| STUCKI | Very High | Slow | Maximum quality |
| JARVIS_JUDICE_NINKE | Highest | Slowest | Smooth gradients |
| DIZZY | Good | Medium | Error diffusion with pseudo-random traversal — no directional structure. Ignores `serpentine` |

## Usage Examples

### Basic Usage

```python
from PIL import Image
from epaper_dithering import dither_image, ColorScheme, DitherMode

img = Image.open("photo.jpg")

result = dither_image(img, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG)
result.save("dithered.png")
```

### All Color Schemes

```python
# Black and white only
dithered = dither_image(img, ColorScheme.MONO)

# Black, white, and red (common for e-paper tags)
dithered = dither_image(img, ColorScheme.BWR)

# Grayscale (4 levels)
dithered = dither_image(img, ColorScheme.GRAYSCALE_4)

# 6-color display (Spectra)
dithered = dither_image(img, ColorScheme.BWGBRY)
```

### Advanced Options

#### Serpentine Scanning

By default, error diffusion algorithms use serpentine scanning (alternating scan direction per row) to reduce directional artifacts and "worm" patterns. You can disable this for raster scanning:

```python
# Default: serpentine scanning (recommended for best quality)
result = dither_image(img, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG, serpentine=True)

# Disable serpentine for raster scanning (left-to-right only)
result = dither_image(img, ColorScheme.BWR, mode=DitherMode.FLOYD_STEINBERG, serpentine=False)
```

Note: The `serpentine` parameter only affects error diffusion algorithms (Floyd-Steinberg, Burkes, Atkinson, Sierra, Sierra Lite, Stucki, Jarvis-Judice-Ninke). It has no effect on NONE and ORDERED modes.

#### Tone Compression (Dynamic Range)

E-paper displays can't reproduce the full luminance range of digital images. Pure white on a display is much darker than (255, 255, 255), and pure black is lighter than (0, 0, 0). Without tone compression, dithering tries to represent unreachable brightness levels, causing large accumulated errors and noisy output.

Tone compression remaps image luminance to the display's actual range before dithering. Based on [`fast_compress_dynamic_range()`](https://github.com/aitjcize/esp32-photoframe) from esp32-photoframe by aitjcize. It is off by default (`tone=0.0`) and only applies when using measured `ColorPalette` instances:

- **`0.0` / `"off"`** (default): Disable tone compression.
- **`"auto"`**: Analyze the image histogram and remap its actual luminance range to the display range. Maximizes contrast by stretching only the used range.
- **`0.0-1.0`**: Fixed linear compression strength. `1.0` maps the full [0,1] range to the display range.

```python
from epaper_dithering import dither_image, SPECTRA_7_3_6COLOR, DitherMode

# Default: tone compression off
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG)

# Auto tone compression
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG, tone="auto")

# Fixed linear compression
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG, tone=1.0)

# Disable tone compression explicitly
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG, tone="off")
```

Note: `tone` has no effect when using theoretical `ColorScheme` palettes (e.g., `ColorScheme.BWR`), since their black/white values already span the full range.

#### Gamut Compression

Some images contain highly saturated colors that a limited palette simply cannot reproduce (e.g. vivid purple on a BWGBRY display). Without gamut compression, the ditherer tries to mix palette colors to approximate the hue — often producing muddy results. Gamut compression pre-blends out-of-gamut pixels toward the nearest palette color before dithering, giving error diffusion a better starting point.

```python
# Default: gamut compression off
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES)

# Auto gamut compression
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES, gamut="auto")

# Fixed strength (0.7–0.9 recommended for very saturated images)
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES, gamut=0.8)

# Disable
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.BURKES, gamut="off")
```

Note: `gamut` also has no effect for theoretical `ColorScheme` palettes.

`DitherMode.NONE` performs direct nearest-color mapping without error diffusion or ordered dithering. It is intended for already-quantized graphics, not continuous-tone photos: because there is no error diffusion, on limited palettes (especially BWR) a continuous-tone image or a large flat mid-tone area can map to an unexpected ink — for example, a solid mid-gray region can render as solid red. Use an error-diffusion mode (e.g. `FLOYD_STEINBERG`, `BURKES`) for photographic input. For built-in measured palettes, pure canonical display colors such as `(255, 0, 0)` map directly to the corresponding firmware palette index even though matching uses measured display RGB values.

For built-in measured palettes, exact canonical display colors are also protected in ordered and error-diffusion modes when pre-processing is off: an image made entirely of display colors is returned as a direct palette-index map, and exact display-color pixels inside a mixed image keep their canonical index instead of being rematched to the measured RGB palette. Pre-processing runs before that exact-pixel check, so explicit `tone="auto"`/`gamut="auto"` or other adjustments may intentionally alter those pixels first.

#### Per-Image Tonal Adjustments

`exposure`, `saturation`, `shadows`, and `highlights` let you tweak the image *before* tone/gamut compression. Each is independent — set just the ones you want. All default to identity (no effect).

```python
# Brighten and boost saturation for vivid output
result = dither_image(img, SPECTRA_7_3_6COLOR, exposure=1.3, saturation=1.4)

# Lift shadows on a dark image
result = dither_image(img, SPECTRA_7_3_6COLOR, shadows=0.5)

# Compress highlights on an overexposed image
result = dither_image(img, SPECTRA_7_3_6COLOR, highlights=0.7)

# Combine for a "vivid photo" look
result = dither_image(img, SPECTRA_7_3_6COLOR,
                      exposure=1.1, saturation=1.3, shadows=0.3, highlights=0.5)
```

Pipeline order: `exposure → saturation → shadows/highlights → tone → gamut → dither`.

#### RGBA Images

Images with transparency (RGBA mode) are automatically composited on a white background, matching the typical appearance of e-paper displays:

```python
# RGBA images are handled automatically
rgba_img = Image.open("transparent.png")  # Has alpha channel
result = dither_image(rgba_img, ColorScheme.BWR)
# Transparent areas become white
```

## Measured Display Colors

For the most accurate dithering, use measured RGB values from your specific e-paper display instead of theoretical pure RGB colors.

### Why Measure?

E-paper displays use reflective technology, making colors **30-87% darker** than pure RGB:
- Pure RGB White: (255, 255, 255)  →  Real display: ~(180-200, 180-200, 180-200)
- Pure RGB Red: (255, 0, 0)  →  Real display: ~(115-125, 10-20, 0-10)

Using measured values ensures dithered images match your display's actual appearance.

### Using Pre-defined Measured Palettes

The library includes measured palettes for common displays:

```python
from epaper_dithering import dither_image, SPECTRA_7_3_6COLOR, DitherMode

# Use measured palette for Spectra 7.3" 6-color display
result = dither_image(img, SPECTRA_7_3_6COLOR, mode=DitherMode.FLOYD_STEINBERG)
```

**Available measured palettes:**
- `SPECTRA_7_3_6COLOR` - 7.3" Spectra™ 6-color (BWGBRY), v1 measurement
- `SPECTRA_7_3_6COLOR_V2` - 7.3" Spectra™ 6-color (BWGBRY), v2 measurement (recommended)
- `MONO_4_26` - 4.26" Monochrome
- `BWRY_4_2` - 4.2" BWRY
- `BWRY_3_97` - 3.97" BWRY
- `SOLUM_BWR` - Solum BWR
- `HANSHOW_BWR` - Hanshow BWR
- `HANSHOW_BWY` - Hanshow BWY

See [CALIBRATION.md](docs/CALIBRATION.md) for measuring your specific display.

### Creating Custom Measured Palettes

Measure your display and create a custom palette:

```python
from epaper_dithering import dither_image, ColorPalette, DitherMode

# Your measured RGB values
my_display = ColorPalette(
    colors={
        'black': (5, 5, 5),           # Measured from your display
        'white': (185, 190, 180),     # Much darker than (255,255,255)
        'red': (120, 15, 5),          # Much darker than (255,0,0)
    },
    accent='red'
)

# Use it directly
result = dither_image(img, my_display, mode=DitherMode.FLOYD_STEINBERG)
```

Built-in measured palettes store the canonical firmware `ColorScheme` they are based on. Custom measured palettes may omit it; in that case direct mapping and exact-color bypass use the custom measured RGB values.

### Measurement Quick Start

1. **Display full-screen color patches** on your e-paper
2. **Photograph** in consistent lighting (avoid shadows/reflections)
3. **Sample RGB values** from center using photo editor
4. **Average 5+ samples** per color
5. **Create ColorPalette** with measured values

See [docs/CALIBRATION.md](docs/CALIBRATION.md) for detailed measurement procedures, including camera calibration, colorimeter usage, and validation techniques.

## Development

```bash
# Install dependencies (requires Rust toolchain: https://rustup.rs)
uv sync --all-extras

# Build and install the Rust extension (required before running tests)
uv run maturin develop --release

# Run tests
uv run pytest tests/ -v

# Run tests with coverage
uv run pytest tests/ --cov=src/epaper_dithering

# Lint
uv run ruff check src/ tests/

# Type check
uv run mypy src/epaper_dithering
```

## Credits

Measured color calibration techniques and reference measurements inspired by:
- [esp32-photoframe](https://github.com/aitjcize/esp32-photoframe) by aitjcize - Measured palette methodology, dynamic range compression algorithm, and reference values for Waveshare 7.3" displays

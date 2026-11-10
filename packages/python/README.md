# epaper-dithering

[![PyPI](https://img.shields.io/pypi/v/epaper-dithering?style=flat-square)](https://pypi.org/project/epaper-dithering/)
[![Python](https://img.shields.io/pypi/pyversions/epaper-dithering?style=flat-square)](https://pypi.org/project/epaper-dithering/)
[![License](https://img.shields.io/github/license/OpenDisplay-org/epaper-dithering?style=flat-square)](LICENSE)
[![Tests](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-test.yml?style=flat-square&label=tests)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-test.yml)
[![Lint](https://img.shields.io/github/actions/workflow/status/OpenDisplay-org/epaper-dithering/python-lint.yml?style=flat-square&label=lint)](https://github.com/OpenDisplay-org/epaper-dithering/actions/workflows/python-lint.yml)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json&style=flat-square)](https://github.com/astral-sh/ruff)
[![mypy](https://img.shields.io/badge/mypy-strict-blue?style=flat-square)](https://mypy.readthedocs.io/)

Dithering for e-ink/e-paper displays: an image and your panel's palette in, palette indices out.

## Installation

```bash
# With uv
uv add epaper-dithering

# With pip
pip install epaper-dithering
```

## Features

- **Rust core**: all dithering runs in a compiled Rust extension, about 30 ms for an 800×480 image.
- **Palettes are data**: describe your panel in an `epaper-palette` JSON file (or build one in code). The library ships no built-in palettes, so any panel works, including per-unit measurements.
- **Requested inks stay solid**: a pixel that asks for an ink (e.g. `#000000` text on a dashboard) comes out as that ink, undithered, even though the panel's real black is lighter than `#000000`.
- **Perceptually correct matching**: weighted Cartesian OKLab, which preserves hue without the achromatic-attractor bug of LCH-weighted approaches.
- **10 dithering algorithms**, from ordered dithering to Jarvis-Judice-Ninke and Dizzy.
- **Pre-dither adjustments**: exposure, saturation, shadows, highlights and gamut compression.
- **DBS refinement**: an optional Direct Binary Search pass that optimises the result for how it is perceived at a given viewing distance.
- **RGBA support**: transparent areas are composited on white.

## Quick Start

```python
from PIL import Image
from epaper_dithering import DitherOptions, Palette, dither_image

palette = Palette.load("my_panel.json")  # see "Palettes" below
img = Image.open("photo.jpg")

result = dither_image(img, palette)  # defaults
result = dither_image(img, palette, DitherOptions(mode="floyd_steinberg"))  # pick an algorithm
result.save("dithered.png")
```

`dither_image` returns a palette-mode (`"P"`) image: each pixel is an index into your palette, and
the image's PIL palette holds each entry's preview colour (what the panel shows).

## Palettes

A palette is an ordered list of entries. The position of an entry is the index written to the
output, so list entries in the order your display driver expects.

Each entry has:

| Field | Meaning |
|---|---|
| `appearance` | How the colour looks on the panel, as CIELAB (D65) `(L*, a*, b*)`, **normalised so the panel's white is L\* 100, a\* = b\* = 0**. |
| `name` *(optional)* | Ink identity, e.g. `"black"`. Only for pure inks; in-between mixes leave it unset. |
| `nominal` *(with `name`)* | The colour that requests this ink, e.g. `"#ff0000"`. |

and the palette has a `measured` flag and an optional `source` note (metadata only).

### Loading a palette file

```python
from epaper_dithering import Palette

palette = Palette.load("examples/palettes/spectra_7_3_6color.json")
palette.index_of("red")  # 3
palette.preview_rgb()  # [(r, g, b), ...] what the panel shows, per entry
```

The repository's [`examples/palettes/`](../../examples/palettes/) folder has starting points: the
idealised 6.x schemes (`ideal_bwr.json`, `ideal_bwry.json`, …) and measured palettes for several real
panels. They are not part of the package; copy the one you need.

The file format:

```json
{
  "format": "epaper-palette",
  "version": 1,
  "measured": true,
  "source": "who measured it, how, when",
  "entries": [
    { "appearance": [100.0, 0.0, 0.0], "name": "white", "nominal": "#ffffff" },
    { "appearance": [18.2, 1.1, -0.4], "name": "black", "nominal": "#000000" },
    { "appearance": [52.0, 8.9, 2.1] }
  ]
}
```

### Building a palette from measurements

Measurements are usually absolute: paper white comes out at L\* 60–80, not 100. Give them to
`from_absolute_lab` together with the index of the white entry; it adapts every entry so white
becomes exactly L\* 100, a\* = b\* = 0:

```python
from epaper_dithering import Palette, PaletteEntry

palette = Palette.from_absolute_lab(
    [
        PaletteEntry((68.26, 8.33, 5.98), name="white", nominal="#ffffff"),
        PaletteEntry((3.19, 4.55, 1.38), name="black", nominal="#000000"),
        PaletteEntry((35.44, 44.96, 28.61), name="red", nominal="#ff0000"),
        PaletteEntry((49.58, 4.63, 1.68)),  # an unnamed mix
    ],
    white_index=0,
    measured=True,
)
print(palette.to_json())  # save it as a palette file
```

`Palette(entries)` itself never guesses: if the entry named `white` is more than ΔE 0.5 from
L\* 100, it raises `PaletteError` and tells you to use `from_absolute_lab`. Palettes without a
`white` entry are taken as already normalised. If you only have sRGB values, use
`Palette.from_srgb([((r, g, b), name, nominal), ...])`; it normalises to the `white` ink if there is
one.

`PaletteError` (a `ValueError`) is also raised for fewer than 2 or more than 256 entries, appearances
that are not finite, duplicate
ink names or nominals, and malformed or unsupported files.

### How named inks behave

Real inks are never pure: a panel's black might measure L\* 10, its red a dull brick. Two options,
both on by default, make sure colours that *ask for* an ink get it:

- **Snapping** (`snap=True`): a pixel exactly equal to an ink's `nominal` takes that ink, undithered,
  and neither receives nor passes on dithering error. This is decided on the original image, so it
  still holds when you also adjust exposure or tone for a photo in the same image.
- **Anchoring** (`anchor=True`): before matching, colours are mapped so each ink's nominal lands
  exactly on its appearance (black-point compensation between the inks named for `#000000` and
  `#ffffff`, plus a local correction around each ink that fades with distance). Near-ink colours and anti-aliased edges
  follow smoothly, and photos keep their tonal range instead of crushing shadows into black.

Unnamed entries (the mixes of a 16-colour mode, say) take part in dithering but are never targets of
either.

## Options

```python
from epaper_dithering import DbsParams, DitherMode, DitherOptions

options = DitherOptions(
    mode=DitherMode.BURKES,  # or a name: "burkes"
    serpentine=True,
    exposure=1.0,  # linear-RGB multiplier, 2.0 = +1 stop
    saturation=1.0,  # OKLab chroma multiplier, 0.0 = greyscale
    shadows=0.0,  # shadow lift, 0.0–1.0
    highlights=0.0,  # highlight compression, 0.0–1.0
    gamut=0.0,  # gamut compression: 0.0/"off", "auto", or 0.0–1.0
    dbs=None,  # None/False, True (defaults), or DbsParams(...)
    anchor=True,
    snap=True,
)
```

Pipeline order: `exposure → saturation → shadows/highlights → anchor → gamut → match + diffuse → dbs`.
Anchoring includes black-point compensation: pure black and white land on the panel's own black and
white (the inks named for `#000000`/`#ffffff`, or else its darkest and lightest entries), so the
image's tonal range is mapped onto the panel's. It replaces 6.x's dynamic-range ("tone") compression.

### Dithering algorithms

| Algorithm | Quality | Speed | Best For |
|-----------|---------|-------|----------|
| `none` | Lowest | Fastest | Already-quantised graphics |
| `ordered` | Low | Very Fast | Patterns, textures |
| `sierra_lite` | Medium | Fast | Quick results |
| `burkes` | Good | Medium | General purpose (default) |
| `floyd_steinberg` | Good | Medium | Popular standard |
| `sierra` | High | Medium | Balanced quality |
| `atkinson` | Good | Medium | High contrast, artistic |
| `stucki` | Very High | Slow | Maximum quality |
| `jarvis_judice_ninke` | Highest | Slowest | Smooth gradients |
| `dizzy` | Good | Medium | Pseudo-random traversal, no directional structure; ignores `serpentine`. Algorithm by [Liam Appelbe](https://liamappelbe.medium.com/dizzy-dithering-2ae76dbceba1) |

`none` maps each pixel to its nearest colour without diffusion. On limited palettes a flat mid-tone
can land on an unexpected ink (a mid-grey area rendering as solid red), so use an error-diffusion
mode for photos.

### DBS refinement

Direct Binary Search runs after dithering and repeatedly changes pixels to other inks, or swaps them
with neighbours, whenever that lowers the error as perceived through a model of the eye at your
viewing distance and panel resolution. It is slower (about 0.3 s per 800×480 panel on a multi-core
machine, versus ~30 ms) and releases the GIL while it runs. Snapped pixels are never changed.

```python
from epaper_dithering import DbsParams, DitherOptions

# Default viewing geometry: 40 cm from a ~127 ppi panel (7.3" 800×480)
result = dither_image(img, palette, DitherOptions(dbs=True))

# A wall-mounted 13.3" panel viewed from 1.5 m
result = dither_image(img, palette, DitherOptions(dbs=DbsParams(viewing_distance_cm=150, ppi=150)))
```

## Migrating from 6.x

7.0 is a clean break: the library no longer knows about any firmware or ships any palette.

| 6.x | 7.x |
|---|---|
| `dither_image(img, ColorScheme.BWR)` | `dither_image(img, Palette.load(".../ideal_bwr.json"))` |
| `SPECTRA_7_3_6COLOR`, `SPECTRA_7_3_6COLOR_V2` | `examples/palettes/spectra_7_3_6color*.json`; the other 6.x "measured" constants were unmeasured placeholders and are gone |
| `ColorPalette(colors={...}, accent=...)` | `Palette.from_srgb([...])`, or better `Palette.from_absolute_lab([...])` from your measurements |
| keyword options (`mode=`, `dbs=` …) | `DitherOptions(mode=..., dbs=...)` |
| `tone=` (dynamic-range compression) | removed: anchoring compensates the black point, on by default |
| `DitherMode.BURKES == 1` | `DitherMode.BURKES == "burkes"` |
| exact "canonical" colours pass through | named inks with a `nominal` snap (and anchor) |

The 6.x measured constants were relative to the camera, not to the panel's white. The exported
files are normalised to white, so the same panel now renders brighter and keeps its shadows.

## Measuring a panel

1. Show full-screen colour patches on the panel (`tools/generate_patches.py` in the repo makes them).
2. Photograph them in even, consistent light.
3. Sample and average each patch, and convert to CIELAB.
4. Build the palette with `Palette.from_absolute_lab`, naming the pure inks.

See [docs/CALIBRATION.md](docs/CALIBRATION.md) for the full procedure.

## Development

```bash
# Install dependencies (requires Rust toolchain: https://rustup.rs)
uv sync --all-extras

# Build and install the Rust extension (required before running tests)
uv run maturin develop --release

# Run tests
uv run pytest tests/ -v

# Lint and type-check
uv run prek run --all-files --config .pre-commit-config.yaml
```

## Credits

Measured colour calibration techniques and reference measurements inspired by:
- [esp32-photoframe](https://github.com/aitjcize/esp32-photoframe) by aitjcize - Measured palette methodology, dynamic range compression algorithm, and reference values for Waveshare 7.3" displays

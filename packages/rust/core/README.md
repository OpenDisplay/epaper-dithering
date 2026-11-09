# epaper-dithering-core

High-quality dithering for e-paper / e-ink displays.

- **Weighted Cartesian OKLab color matching** — perceptually accurate, hue-preserving, no achromatic-attractor bug
- **7 error diffusion kernels** — Floyd-Steinberg, Atkinson, Burkes, Stucki, Sierra, Sierra Lite, Jarvis-Judice-Ninke
- **Ordered (Bayer 4×4) dithering** — perceptually-correct sRGB-space thresholding, parallelized via rayon
- **Measured palettes** — calibrated colors for real displays (Spectra 7.3", BWRY 3.97", and more)
- **Pre-dither knobs** — exposure, saturation, shadows, highlights, dynamic-range compression, gamut compression
- **Serpentine scanning** — reduces directional artifacts in error diffusion

## Examples

![Frankfurt at night — Spectra 6-color, auto tone + gamut](../../../docs/examples/frankfurt_before_after.png)

## Usage

```rust
use epaper_dithering_core::{
    dither, DitherConfig,
    enums::DitherMode,
    palettes::ColorScheme,
    types::ImageBuffer,
};

// pixels: flat RGB bytes, row-major (width × height × 3)
let img = ImageBuffer::new(&pixels, width);
let indices = dither(&img, ColorScheme::Bwr, DitherConfig {
    mode: DitherMode::FloydSteinberg,
    ..Default::default()
});
```

With a measured palette and pre-dither adjustments:

```rust
use epaper_dithering_core::{
    dither, DitherConfig,
    enums::{DitherMode, ToneCompression, GamutCompression},
    measured_palettes::SPECTRA_7_3_6COLOR,
    types::ImageBuffer,
};

let img = ImageBuffer::new(&pixels, width);
let indices = dither(&img, &SPECTRA_7_3_6COLOR, DitherConfig {
    mode: DitherMode::Stucki,
    saturation: 1.3,           // boost saturation
    shadows: 0.4,              // lift shadows
    tone:  ToneCompression::Auto, // opt in for photos
    gamut: GamutCompression::Auto,
    ..Default::default()
});
```

`DitherConfig` defaults: `Burkes`, `serpentine: true`, `exposure: 1.0`, `saturation: 1.0`,
`shadows: 0.0`, `highlights: 0.0`, `tone: Fixed(0.0)`, `gamut: None`.

Pipeline order: `exposure → saturation → shadows/highlights → tone → gamut → dither`.

`DitherMode::None` performs direct nearest-color mapping without error diffusion or ordered dithering. It is intended for already-quantized graphics, not continuous-tone photos: with no error diffusion, on limited palettes (especially BWR) a continuous-tone image or a large flat mid-tone area can map to an unexpected ink — for example, a solid mid-gray region can render as solid red. Use an error-diffusion mode (e.g. `FloydSteinberg`, `Burkes`) for photographic input. `dither_with_canonical` lets measured palettes use calibrated RGB values for matching while preserving the canonical display palette for exact-color bypass and firmware indices. The two palettes are aligned by index: entry *i* of `canonical_palette` must be the ideal color of the same ink as entry *i* of `matching_palette`, since an exact pixel is given that index.

With `dither_with_canonical`, exact canonical display colors are also protected in ordered and error-diffusion modes when pre-processing is off: an image made entirely of display colors is returned as a direct palette-index map, and exact display-color pixels inside a mixed image keep their canonical index instead of being rematched to the measured RGB palette. Pre-processing runs before that exact-pixel check, so explicit tone/gamut compression or other adjustments may intentionally alter those pixels first.

## Related packages

- Python: [`epaper-dithering`](https://pypi.org/project/epaper-dithering/)
- JavaScript: [`@opendisplay/epaper-dithering`](https://www.npmjs.com/package/@opendisplay/epaper-dithering)

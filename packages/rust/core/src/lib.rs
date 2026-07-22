pub mod algorithms;
pub mod color_space;
pub mod color_space_lab;
pub mod composite;
pub mod dizzy;
pub mod enums;
pub mod error;
pub mod kernels;
pub mod measured_palettes;
pub mod palettes;
pub mod tone_map;
pub mod types;

use crate::color_space::{linear_channel_to_srgb, srgb_channel_to_linear};
use crate::enums::{DitherMode, GamutCompression, ToneCompression};
use crate::palettes::Palette;
use crate::types::ImageBuffer;

/// Configuration for `dither()`. All fields have sensible defaults.
///
/// Construct with struct-update syntax:
/// ```ignore
/// dither(&img, palette, DitherConfig { mode: DitherMode::Burkes, ..Default::default() });
/// ```
///
/// Pre-processing pipeline (applied in order, each step is a no-op at its identity value):
/// `exposure → saturation → shadows/highlights → tone → gamut → dither`.
#[derive(Debug, Clone, PartialEq)]
pub struct DitherConfig {
    /// Dithering algorithm.
    pub mode: DitherMode,
    /// Use serpentine scanning for error diffusion (alternates row direction). Ignored for
    /// `None` and `Ordered`.
    pub serpentine: bool,
    /// Linear-RGB exposure multiplier. 1.0 = no change, 2.0 = +1 stop, 0.5 = -1 stop.
    pub exposure: f64,
    /// OKLab saturation multiplier. 1.0 = no change, 0.0 = grayscale, >1.0 = boost.
    /// Hue-preserving.
    pub saturation: f64,
    /// Shadow lift strength (lower-half S-curve). 0.0 = identity, 1.0 = strong lift.
    pub shadows: f64,
    /// Highlight compression strength (upper-half S-curve). 0.0 = identity, 1.0 = strong.
    pub highlights: f64,
    /// Dynamic-range compression. Auto = histogram-based; Fixed(s) = manual blend strength.
    pub tone: ToneCompression,
    /// Gamut compression. Auto = full strength on out-of-gamut; Fixed(s) = manual.
    pub gamut: GamutCompression,
}

impl Default for DitherConfig {
    fn default() -> Self {
        Self {
            mode:       DitherMode::Burkes,
            serpentine: true,
            exposure:   1.0,
            saturation: 1.0,
            shadows:    0.0,
            highlights: 0.0,
            tone:       ToneCompression::Fixed(0.0),
            gamut:      GamutCompression::None,
        }
    }
}

fn dispatch(
    img: &ImageBuffer,
    p: &Palette,
    canonical: &Palette,
    mode: DitherMode,
    serpentine: bool,
    pin_exact_pixels: bool,
) -> Vec<u8> {
    match mode {
        DitherMode::None => algorithms::direct_map(img.data, p, canonical),
        DitherMode::Ordered if pin_exact_pixels => {
            algorithms::ordered_dither_with_canonical(img.data, img.width, p, canonical)
        }
        DitherMode::Ordered => algorithms::ordered_dither(img.data, img.width, p),
        DitherMode::Dizzy if pin_exact_pixels => {
            dizzy::dizzy_dither_with_canonical(img.data, img.width, img.height, p, canonical)
        }
        DitherMode::Dizzy => dizzy::dizzy_dither(img.data, img.width, img.height, p),
        DitherMode::FloydSteinberg
        | DitherMode::Burkes
        | DitherMode::Atkinson
        | DitherMode::Stucki
        | DitherMode::Sierra
        | DitherMode::SierraLite
        | DitherMode::JarvisJudiceNinke => {
            // Every arm in this group has `kernel() == Some(_)` by construction
            // (see `DitherMode::kernel` in enums.rs); `None`/`Ordered` are handled above.
            let kernel = mode
                .kernel()
                .expect("kernel-based DitherMode variants always return Some from kernel()");
            if pin_exact_pixels {
                algorithms::error_diffusion_dither_with_canonical(
                    img.data,
                    img.width,
                    img.height,
                    p,
                    canonical,
                    kernel,
                    serpentine,
                )
            } else {
                algorithms::error_diffusion_dither(
                    img.data,
                    img.width,
                    img.height,
                    p,
                    kernel,
                    serpentine,
                )
            }
        }
    }
}

fn needs_preprocess(c: &DitherConfig) -> bool {
    (c.exposure - 1.0).abs() > 1e-9
        || (c.saturation - 1.0).abs() > 1e-9
        || c.shadows > 0.0
        || c.highlights > 0.0
        || !matches!(c.tone, ToneCompression::Fixed(s) if s <= 0.0)
        || !matches!(c.gamut, GamutCompression::None)
}

/// Dither an image for e-paper display.
///
/// Returns palette indices (one `u8` per pixel, length = `width × height`).
pub fn dither(img: &ImageBuffer, palette: impl AsRef<Palette>, config: DitherConfig) -> Vec<u8> {
    let p = palette.as_ref();
    dither_impl(img, p, p, config, false)
}

/// Dither an image using one palette for color matching and another for exact
/// already-displayable RGB passthrough.
///
/// Measured palettes should be supplied as `matching_palette`; the ideal display
/// color scheme should be supplied as `canonical_palette`.
pub fn dither_with_canonical(
    img: &ImageBuffer,
    matching_palette: impl AsRef<Palette>,
    canonical_palette: impl AsRef<Palette>,
    config: DitherConfig,
) -> Vec<u8> {
    let p = matching_palette.as_ref();
    let canonical = canonical_palette.as_ref();
    dither_impl(img, p, canonical, config, true)
}

fn dither_impl(
    img: &ImageBuffer,
    p: &Palette,
    canonical: &Palette,
    config: DitherConfig,
    pin_exact_pixels: bool,
) -> Vec<u8> {
    if !needs_preprocess(&config) {
        if config.mode != DitherMode::None
            && let Some(indices) = algorithms::try_exact_palette_map(img.data, canonical)
        {
            return indices;
        }
        return dispatch(img, p, canonical, config.mode, config.serpentine, pin_exact_pixels);
    }

    // Convert sRGB bytes → linear, apply pre-processing pipeline, convert back.
    let mut linear: Vec<[f64; 3]> = img
        .data
        .chunks_exact(3)
        .map(|c| [
            srgb_channel_to_linear(c[0]),
            srgb_channel_to_linear(c[1]),
            srgb_channel_to_linear(c[2]),
        ])
        .collect();

    tone_map::apply_exposure(&mut linear, config.exposure);
    tone_map::adjust_saturation(&mut linear, config.saturation);
    tone_map::apply_shadows_highlights(&mut linear, config.shadows, config.highlights);
    config.tone.apply(&mut linear, p);
    config.gamut.apply(&mut linear, p);

    let processed: Vec<u8> = linear
        .iter()
        .flat_map(|&[r, g, b]| [
            linear_channel_to_srgb(r),
            linear_channel_to_srgb(g),
            linear_channel_to_srgb(b),
        ])
        .collect();

    let processed_img = ImageBuffer::new(&processed, img.width);
    dispatch(&processed_img, p, canonical, config.mode, config.serpentine, pin_exact_pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measured_palettes::SPECTRA_7_3_6COLOR;
    use crate::palettes::ColorScheme;

    fn pixels(rgb: [u8; 3], count: usize) -> Vec<u8> {
        std::iter::repeat_n(rgb, count).flatten().collect()
    }

    #[test]
    fn default_config_has_preprocessing_off() {
        let config = DitherConfig::default();
        assert!(matches!(config.tone, ToneCompression::Fixed(s) if s == 0.0));
        assert_eq!(config.gamut, GamutCompression::None);
        assert!(!needs_preprocess(&config));
    }

    /// Pins `ToneCompression::default()` and `GamutCompression::default()` to the values
    /// `DitherConfig::default()` actually uses for those fields, so the two defaults can
    /// never silently drift apart again (see audit finding L3).
    #[test]
    fn enum_defaults_match_dither_config_default() {
        let config = DitherConfig::default();
        assert_eq!(ToneCompression::default(), config.tone);
        assert_eq!(GamutCompression::default(), config.gamut);
    }

    /// Proves `DitherMode::Dizzy` is actually wired to the dizzy algorithm and not
    /// accidentally aliased to another dispatch arm: on identical non-uniform input,
    /// its output must differ from Burkes error diffusion.
    #[test]
    fn dizzy_output_differs_from_burkes() {
        let pixels: Vec<u8> = (0u8..=255)
            .flat_map(|v| [v, v / 2, 255 - v])
            .cycle()
            .take(16 * 16 * 3)
            .collect();
        let img = ImageBuffer::new(&pixels, 16);

        let burkes = dither(&img, &SPECTRA_7_3_6COLOR, DitherConfig { mode: DitherMode::Burkes, ..Default::default() });
        let dizzy = dither(&img, &SPECTRA_7_3_6COLOR, DitherConfig { mode: DitherMode::Dizzy, ..Default::default() });

        assert_ne!(dizzy, burkes, "dizzy dithering should differ from Burkes on non-uniform input");
    }

    #[test]
    fn none_uses_canonical_exact_colors_with_measured_palette() {
        let image = pixels([255, 0, 0], 4);
        let img = ImageBuffer::new(&image, 2);
        let output = dither_with_canonical(
            &img,
            &SPECTRA_7_3_6COLOR,
            ColorScheme::Bwgbry.palette(),
            DitherConfig { mode: DitherMode::None, ..Default::default() },
        );
        assert_eq!(output, vec![3, 3, 3, 3]);
    }

    #[test]
    fn exact_canonical_colors_bypass_error_diffusion() {
        let image = [
            [0, 0, 0],
            [255, 255, 255],
            [255, 255, 0],
            [255, 0, 0],
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let img = ImageBuffer::new(&image, 2);
        let output = dither_with_canonical(
            &img,
            &SPECTRA_7_3_6COLOR,
            ColorScheme::Bwry.palette(),
            DitherConfig { mode: DitherMode::Burkes, ..Default::default() },
        );
        assert_eq!(output, vec![0, 1, 2, 3]);
    }

    #[test]
    fn exact_canonical_pixels_are_pinned_inside_mixed_error_diffusion_image() {
        let mut image = pixels([128, 128, 128], 8);
        image[0..3].copy_from_slice(&[0, 255, 0]);
        image[9..12].copy_from_slice(&[0, 255, 0]);
        let img = ImageBuffer::new(&image, 4);

        for mode in [
            DitherMode::Burkes,
            DitherMode::FloydSteinberg,
            DitherMode::Atkinson,
            DitherMode::Stucki,
            DitherMode::Sierra,
            DitherMode::SierraLite,
            DitherMode::JarvisJudiceNinke,
        ] {
            let output = dither_with_canonical(
                &img,
                &SPECTRA_7_3_6COLOR,
                ColorScheme::Bwgbry.palette(),
                DitherConfig { mode, ..Default::default() },
            );
            assert_eq!(output[0], 5, "{mode:?} should pin exact green at pixel 0");
            assert_eq!(output[3], 5, "{mode:?} should pin exact green at pixel 3");
        }
    }

    #[test]
    fn exact_canonical_pixels_are_pinned_inside_mixed_ordered_image() {
        let mut image = pixels([128, 128, 128], 8);
        image[0..3].copy_from_slice(&[0, 255, 0]);
        image[9..12].copy_from_slice(&[0, 255, 0]);
        let img = ImageBuffer::new(&image, 4);

        let output = dither_with_canonical(
            &img,
            &SPECTRA_7_3_6COLOR,
            ColorScheme::Bwgbry.palette(),
            DitherConfig { mode: DitherMode::Ordered, ..Default::default() },
        );
        assert_eq!(output[0], 5);
        assert_eq!(output[3], 5);
    }

    #[test]
    fn exact_bypass_is_skipped_when_preprocessing_is_enabled() {
        let image = pixels([255, 0, 0], 4);
        let img = ImageBuffer::new(&image, 2);
        let output = dither_with_canonical(
            &img,
            &SPECTRA_7_3_6COLOR,
            ColorScheme::Bwgbry.palette(),
            DitherConfig {
                mode: DitherMode::Burkes,
                tone: ToneCompression::Fixed(1.0),
                ..Default::default()
            },
        );
        assert_eq!(output.len(), 4);
    }
}

//! Dithering for e-paper displays: an image and a palette in, palette indices out.
//!
//! Build a [`palette::Palette`] (from a JSON file, from measurements, or from sRGB) and
//! call [`dither`]. The library has no built-in palettes.

mod anchor;
pub mod algorithms;
pub mod color_space;
pub mod color_space_lab;
pub mod composite;
pub mod dbs;
pub mod dizzy;
pub mod enums;
pub mod error;
pub mod kernels;
pub mod lab;
pub mod palette;
mod palette_file;
pub mod srgb_palette;
pub mod tone_map;
pub mod types;

use rayon::prelude::*;

use crate::color_space::{linear_channel_to_srgb, srgb_to_linear_lut};
use crate::dbs::DbsParams;
use crate::enums::DitherMode;
use crate::palette::Palette;
use crate::srgb_palette::SrgbPalette;
use crate::types::ImageBuffer;

#[cfg(test)]
use crate::{error::DitherError, palette::PaletteEntry};

/// Options for [`dither`]. All fields have sensible defaults.
///
/// Construct with struct-update syntax:
/// ```ignore
/// dither(&img, &palette, &DitherOptions { mode: DitherMode::Burkes, ..Default::default() });
/// ```
///
/// Pipeline (applied in order, each step is a no-op at its identity value):
/// `exposure → saturation → shadows/highlights → tone → anchor → gamut → match + diffuse → dbs`.
/// Ink snapping is decided on the original input, before any of these steps.
#[derive(Debug, Clone, PartialEq)]
pub struct DitherOptions {
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
    /// Black-point lift, 0.0 (off) to 1.0: remaps lightness onto the panel's darkest and
    /// lightest entries, keeping hue. Brings out shadow detail at the cost of deep blacks.
    pub tone: f64,
    /// Gamut compression strength, 0.0 (off) to 1.0: pulls out-of-gamut colours toward the
    /// palette's gamut before matching.
    pub gamut: f64,
    /// Optional Direct Binary Search refinement of the dithered output (see `dbs`). Slow:
    /// seconds on a full panel, so `None` by default.
    pub dbs: Option<DbsParams>,
    /// Map each named ink's nominal colour onto its measured appearance before matching;
    /// colours near an ink follow, colours far from every ink are left alone.
    pub anchor: bool,
    /// A pixel exactly equal to a named ink's nominal takes that ink, undithered.
    pub snap: bool,
}

impl Default for DitherOptions {
    fn default() -> Self {
        Self {
            mode:       DitherMode::Burkes,
            serpentine: true,
            exposure:   1.0,
            saturation: 1.0,
            shadows:    0.0,
            highlights: 0.0,
            tone:       0.0,
            gamut:      0.0,
            dbs:        None,
            anchor:     true,
            snap:       true,
        }
    }
}

fn dispatch(
    img: &ImageBuffer,
    p: &SrgbPalette,
    snap: Option<&[Option<u8>]>,
    mode: DitherMode,
    serpentine: bool,
) -> Vec<u8> {
    match mode {
        DitherMode::None => algorithms::direct_map(img.data, p, snap),
        DitherMode::Ordered => match snap {
            Some(s) => algorithms::ordered_dither_snapped(img.data, img.width, p, s),
            None => algorithms::ordered_dither(img.data, img.width, p),
        },
        DitherMode::Dizzy => match snap {
            Some(s) => dizzy::dizzy_dither_snapped(img.data, img.width, img.height, p, s),
            None => dizzy::dizzy_dither(img.data, img.width, img.height, p),
        },
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
            match snap {
                Some(s) => algorithms::error_diffusion_dither_snapped(
                    img.data, img.width, img.height, p, s, kernel, serpentine,
                ),
                None => algorithms::error_diffusion_dither(img.data, img.width, img.height, p, kernel, serpentine),
            }
        }
    }
}

fn needs_preprocess(o: &DitherOptions) -> bool {
    (o.exposure - 1.0).abs() > 1e-9
        || (o.saturation - 1.0).abs() > 1e-9
        || o.shadows > 0.0
        || o.highlights > 0.0
        || o.tone > 0.0
        || o.gamut > 0.0
}

/// Per pixel: the index of the ink whose nominal equals the pixel exactly, else `None`.
/// `None` overall when the palette has no inks, so callers skip the masked paths.
pub(crate) fn snap_mask(pixels: &[u8], palette: &Palette) -> Option<Vec<Option<u8>>> {
    let inks: Vec<(u8, [u8; 3])> = palette.inks().map(|(i, ink)| (i, ink.nominal)).collect();
    if inks.is_empty() {
        return None;
    }
    Some(
        pixels
            .as_chunks::<3>()
            .0
            .iter()
            .map(|rgb| inks.iter().find(|(_, nominal)| nominal == rgb).map(|&(i, _)| i))
            .collect(),
    )
}

/// Dither `img` to `palette`. Returns one palette index per pixel, row-major.
pub fn dither(img: &ImageBuffer, palette: &Palette, options: &DitherOptions) -> Vec<u8> {
    let p = SrgbPalette::from(palette);
    let mask = if options.snap { snap_mask(img.data, palette) } else { None };
    if let Some(m) = &mask
        && let Some(all) = m.iter().copied().collect::<Option<Vec<u8>>>()
    {
        return all;
    }
    let snap = mask.as_deref();
    let anchoring = options.anchor && anchor::is_active(palette);

    if !needs_preprocess(options) && !anchoring {
        let indices = dispatch(img, &p, snap, options.mode, options.serpentine);
        return refine(img, &p, indices, options.dbs, snap);
    }

    // Convert sRGB bytes → linear, apply pre-processing pipeline, convert back.
    let lut = srgb_to_linear_lut();
    let mut linear: Vec<[f64; 3]> =
        img.data.as_chunks::<3>().0.iter().map(|c| c.map(|v| lut[v as usize])).collect();

    tone_map::apply_exposure(&mut linear, options.exposure);
    tone_map::adjust_saturation(&mut linear, options.saturation);
    tone_map::apply_shadows_highlights(&mut linear, options.shadows, options.highlights);
    tone_map::compress_dynamic_range(&mut linear, &p, options.tone.min(1.0));
    if anchoring {
        anchor::anchor_map(&mut linear, palette);
    }
    tone_map::gamut_compress(&mut linear, &p, options.gamut.min(1.0));

    // Three `powf` per pixel: by far the most expensive part of this round trip, so in parallel.
    let mut processed = vec![0u8; img.data.len()];
    processed
        .par_chunks_exact_mut(3)
        .zip(linear.par_iter())
        .for_each(|(out, px)| out.copy_from_slice(&px.map(linear_channel_to_srgb)));

    let processed_img = ImageBuffer { data: &processed, width: img.width, height: img.height };
    let indices = dispatch(&processed_img, &p, snap, options.mode, options.serpentine);
    refine(&processed_img, &p, indices, options.dbs, snap)
}

/// Final pipeline step: optional DBS refinement against the (pre-processed) target image.
/// Snapped pixels stay pinned.
fn refine(
    img: &ImageBuffer,
    p: &SrgbPalette,
    mut indices: Vec<u8>,
    params: Option<DbsParams>,
    snap: Option<&[Option<u8>]>,
) -> Vec<u8> {
    let Some(params) = params else { return indices };
    let pinned: Option<Vec<bool>> = snap.map(|s| s.iter().map(Option::is_some).collect());
    dbs::dbs_refine(img.data, img.width, img.height, p, &mut indices, pinned.as_deref(), &params);
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Ink;

    fn pixels(rgb: [u8; 3], count: usize) -> Vec<u8> {
        std::iter::repeat_n(rgb, count).flatten().collect()
    }

    fn img(data: &[u8], width: usize) -> ImageBuffer<'_> {
        ImageBuffer::new(data, width).unwrap()
    }

    /// Measured Spectra 6: black, white, yellow, red, blue, green, each an ink with its ideal nominal.
    fn spectra6() -> Palette {
        Palette::from_json(include_str!("../../../../examples/palettes/spectra_7_3_6color.json")).unwrap()
    }

    /// The same measured colours with no ink identity: nothing snaps, nothing anchors.
    fn spectra6_unnamed() -> Palette {
        let entries = spectra6().entries().iter().map(|e| PaletteEntry { appearance: e.appearance, ink: None }).collect();
        Palette::new(entries, true).unwrap()
    }

    fn green_dots() -> Vec<u8> {
        let mut image = pixels([128, 128, 128], 8);
        image[0..3].copy_from_slice(&[0, 255, 0]);
        image[9..12].copy_from_slice(&[0, 255, 0]);
        image
    }

    #[test]
    fn default_options_have_preprocessing_off_and_inks_on() {
        let o = DitherOptions::default();
        assert!(o.anchor);
        assert!(o.snap);
        assert!(!needs_preprocess(&o));
    }

    #[test]
    fn tone_and_gamut_default_to_off() {
        let o = DitherOptions::default();
        assert_eq!((o.tone, o.gamut), (0.0, 0.0));
    }

    #[test]
    fn dizzy_output_differs_from_burkes() {
        let data: Vec<u8> = (0u8..=255).flat_map(|v| [v, v / 2, 255 - v]).cycle().take(16 * 16 * 3).collect();
        let p = spectra6_unnamed();
        let burkes = dither(&img(&data, 16), &p, &DitherOptions { mode: DitherMode::Burkes, ..Default::default() });
        let dizzy = dither(&img(&data, 16), &p, &DitherOptions { mode: DitherMode::Dizzy, ..Default::default() });
        assert_ne!(dizzy, burkes);
    }

    #[test]
    fn snap_none_mode_uses_ink_nominals() {
        let data = pixels([255, 0, 0], 4);
        let out = dither(&img(&data, 2), &spectra6(), &DitherOptions { mode: DitherMode::None, ..Default::default() });
        assert_eq!(out, vec![3, 3, 3, 3]);
    }

    #[test]
    fn snap_inks_bypass_error_diffusion() {
        let data: Vec<u8> = [[0, 0, 0], [255, 255, 255], [255, 255, 0], [255, 0, 0]].into_iter().flatten().collect();
        let out = dither(&img(&data, 2), &spectra6(), &DitherOptions::default());
        assert_eq!(out, vec![0, 1, 2, 3]);
    }

    #[test]
    fn snap_pins_inks_inside_mixed_error_diffusion_image() {
        let data = green_dots();
        for mode in [
            DitherMode::Burkes,
            DitherMode::FloydSteinberg,
            DitherMode::Atkinson,
            DitherMode::Stucki,
            DitherMode::Sierra,
            DitherMode::SierraLite,
            DitherMode::JarvisJudiceNinke,
            DitherMode::Dizzy,
        ] {
            let out = dither(&img(&data, 4), &spectra6(), &DitherOptions { mode, ..Default::default() });
            assert_eq!((out[0], out[3]), (5, 5), "{mode:?} should pin exact green");
        }
    }

    #[test]
    fn snap_pins_inks_inside_mixed_ordered_image() {
        let data = green_dots();
        let out = dither(&img(&data, 4), &spectra6(), &DitherOptions { mode: DitherMode::Ordered, ..Default::default() });
        assert_eq!((out[0], out[3]), (5, 5));
    }

    #[test]
    fn snap_survives_preprocessing() {
        let data = green_dots();
        let o = DitherOptions { exposure: 1.3, saturation: 0.5, tone: 1.0, gamut: 1.0, ..Default::default() };
        let out = dither(&img(&data, 4), &spectra6(), &o);
        assert_eq!((out[0], out[3]), (5, 5));
    }

    #[test]
    fn snap_pins_inks_under_dbs() {
        let data = green_dots();
        let o = DitherOptions { dbs: Some(DbsParams::default()), ..Default::default() };
        let out = dither(&img(&data, 4), &spectra6(), &o);
        assert_eq!((out[0], out[3]), (5, 5));
    }

    #[test]
    fn snap_false_disables_passthrough() {
        let data = pixels([255, 0, 0], 4);
        let with = dither(&img(&data, 2), &spectra6(), &DitherOptions { mode: DitherMode::None, anchor: false, ..Default::default() });
        let without = dither(&img(&data, 2), &spectra6(), &DitherOptions { mode: DitherMode::None, anchor: false, snap: false, ..Default::default() });
        let unnamed = dither(&img(&data, 2), &spectra6_unnamed(), &DitherOptions { mode: DitherMode::None, ..Default::default() });
        assert_eq!(with, vec![3; 4]);
        assert_eq!(without, unnamed);
    }

    #[test]
    fn dbs_refinement_lowers_perceived_error() {
        let data: Vec<u8> = (0..24 * 16)
            .flat_map(|i| [(i % 24 * 10) as u8, (i / 24 * 16) as u8, 200 - (i % 24 * 8) as u8])
            .collect();
        let params = DbsParams::default();
        let p = spectra6_unnamed();
        let plain = dither(&img(&data, 24), &p, &DitherOptions::default());
        let refined = dither(&img(&data, 24), &p, &DitherOptions { dbs: Some(params), ..Default::default() });
        let srgb = SrgbPalette::from(&p);
        let err = |idx: &[u8]| dbs::perceived_error(&data, 24, 16, &srgb, idx, &params);
        assert!(err(&refined) < err(&plain), "{} !< {}", err(&refined), err(&plain));
    }

    #[test]
    fn tone_finds_black_and_white_by_lightness() {
        // Same three colours in two orders: the output must be the same colours.
        let colours = |order: &[usize]| {
            let all = [[250u8, 250, 250], [200, 30, 30], [20, 20, 20]];
            Palette::from_srgb(order.iter().map(|&i| (all[i], None)).collect(), false).unwrap()
        };
        let a = colours(&[0, 1, 2]);
        let b = colours(&[2, 0, 1]);
        let ramp: Vec<u8> = (0..64u8).flat_map(|v| [v * 4, v * 4, v * 4]).collect();
        let o = DitherOptions { tone: 1.0, ..Default::default() };
        let to_rgb = |p: &Palette, idx: Vec<u8>| idx.iter().map(|&i| p.preview_srgb()[i as usize]).collect::<Vec<_>>();
        assert_eq!(to_rgb(&a, dither(&img(&ramp, 8), &a, &o)), to_rgb(&b, dither(&img(&ramp, 8), &b, &o)));
    }

    #[test]
    fn tone_lifts_shadows_and_is_off_by_default() {
        let data: Vec<u8> = (0..64u8).flat_map(|v| [v, v, v]).collect();
        let p = spectra6_unnamed();
        let off = dither(&img(&data, 8), &p, &DitherOptions::default());
        let on = dither(&img(&data, 8), &p, &DitherOptions { tone: 1.0, ..Default::default() });
        assert_ne!(on, off);
    }

    #[test]
    fn dither_mode_names_round_trip() {
        for m in DitherMode::ALL {
            assert_eq!(m.to_string().parse::<DitherMode>().unwrap(), m);
        }
        assert_eq!("floyd_steinberg".parse::<DitherMode>().unwrap(), DitherMode::FloydSteinberg);
    }

    #[test]
    fn unknown_dither_mode_name_errors() {
        assert_eq!("bayer".parse::<DitherMode>().unwrap_err(), DitherError::UnknownDitherMode("bayer".into()));
    }

    #[test]
    fn image_buffer_rejects_bad_length() {
        assert_eq!(ImageBuffer::new(&[0; 10], 3).unwrap_err(), DitherError::InvalidImage { len: 10, width: 3 });
        assert_eq!(ImageBuffer::new(&[], 0).unwrap_err(), DitherError::InvalidImage { len: 0, width: 0 });
    }

    #[test]
    fn anchor_option_changes_output_for_measured_inks() {
        let data: Vec<u8> = (0..32u8).flat_map(|v| [v * 8, v * 4, 255 - v * 8]).cycle().take(16 * 16 * 3).collect();
        let on = dither(&img(&data, 16), &spectra6(), &DitherOptions { snap: false, ..Default::default() });
        let off = dither(&img(&data, 16), &spectra6(), &DitherOptions { snap: false, anchor: false, ..Default::default() });
        assert_ne!(on, off, "anchoring should move colours for a measured palette");
    }

    #[test]
    fn anchor_option_is_inert_without_inks() {
        let data: Vec<u8> = (0..32u8).flat_map(|v| [v * 8, v * 4, 255 - v * 8]).cycle().take(16 * 16 * 3).collect();
        let on = dither(&img(&data, 16), &spectra6_unnamed(), &DitherOptions::default());
        let off = dither(&img(&data, 16), &spectra6_unnamed(), &DitherOptions { anchor: false, ..Default::default() });
        assert_eq!(on, off);
    }

    #[test]
    fn named_ink_without_snap_or_anchor_is_plain_matching() {
        let p = Palette::from_srgb(vec![([0, 0, 0], Some(Ink { name: "black".into(), nominal: [0, 0, 0] })), ([255, 255, 255], None)], false).unwrap();
        let data = pixels([10, 10, 10], 4);
        assert_eq!(dither(&img(&data, 2), &p, &DitherOptions { mode: DitherMode::None, anchor: false, snap: false, ..Default::default() }), vec![0; 4]);
    }
}

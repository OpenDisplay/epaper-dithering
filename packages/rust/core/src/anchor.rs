//! Anchored mapping: move each named ink's nominal colour onto the colour the panel
//! actually shows for it, and carry the colours around it along smoothly.
//!
//! Method, in CIELAB (not OKLab: its cube root has no linear toe, so near black a single
//! sRGB step is a large OKLab distance and a correction there turns into a visible jump):
//! a **damped local correction** for every named ink, plus a zero correction pinned at
//! pure black and white when no ink is named for them. A pixel moves by
//! `Σ w_k r_k / (Σ w_k + W0)` with `w_k = 1/d_k⁴`, where `r_k` is ink `k`'s appearance minus
//! its nominal. At an ink the weight is unbounded, so the ink lands exactly; far from every
//! ink the `W0` term wins and the correction fades to zero. (Undamped inverse-distance
//! weighting gave far-away colours the *average* correction, muddying whole photos.)
//!
//! Anchoring deliberately does not re-map the image's tonal range: lifting shadows onto the
//! panel's black is the separate, opt-in `tone` step.
//!
//! A no-op when no named ink's appearance differs from its nominal.

use rayon::prelude::*;

use crate::color_space::srgb_channel_to_linear;
use crate::lab::{lab_to_linear, linear_to_lab, Lab};
use crate::palette::Palette;

/// CIELAB distance below which a pixel counts as sitting on an anchor.
const SNAP_DISTANCE: f64 = 1e-9;
/// Displacements smaller than this are treated as zero (idealised palettes).
const NEGLIGIBLE: f64 = 1e-9;
/// CIELAB radius at which an ink's local correction has fallen to half strength. The
/// fourth-power falloff keeps corrections local: at twice this distance an ink pulls with
/// 1/17 of its strength (inverse-square weighting leaked about 15% of the black ink's
/// correction into mid grey).
const LOCAL_RADIUS: f64 = 25.0;

fn linear(rgb: [u8; 3]) -> [f64; 3] {
    rgb.map(srgb_channel_to_linear)
}

/// An anchor: the ink's nominal, and how far its appearance is from it.
struct Local {
    source: Lab,
    residual: [f64; 3],
    target_linear: [f64; 3],
}

struct Mapping {
    locals: Vec<Local>,
}

impl Mapping {
    fn new(palette: &Palette) -> Option<Self> {
        let mut locals: Vec<Local> = palette
            .inks()
            .map(|(i, ink)| {
                let target_linear = linear(palette.preview_srgb()[i as usize]);
                let source = linear_to_lab(linear(ink.nominal));
                let target = linear_to_lab(target_linear);
                Local { source, residual: [target.l - source.l, target.a - source.a, target.b - source.b], target_linear }
            })
            .collect();
        if !locals.iter().any(|l| l.residual.iter().any(|r| r.abs() > NEGLIGIBLE)) {
            return None;
        }
        // Pure black and white stay put unless an ink is named for them, and pin the other
        // inks' corrections so they fade out before reaching the ends of the grey axis.
        for rgb in [[0u8, 0, 0], [255, 255, 255]] {
            if !palette.inks().any(|(_, ink)| ink.nominal == rgb) {
                locals.push(Local { source: linear_to_lab(linear(rgb)), residual: [0.0; 3], target_linear: linear(rgb) });
            }
        }
        Some(Self { locals })
    }

    fn apply(&self, px: [f64; 3]) -> [f64; 3] {
        let x = linear_to_lab(px);
        let mut weight_sum = 1.0 / LOCAL_RADIUS.powi(4);
        let mut shift = [0.0; 3];
        for a in &self.locals {
            let d2 = (x.l - a.source.l).powi(2) + (x.a - a.source.a).powi(2) + (x.b - a.source.b).powi(2);
            if d2 < SNAP_DISTANCE * SNAP_DISTANCE {
                return a.target_linear;
            }
            let w = 1.0 / (d2 * d2);
            weight_sum += w;
            for (acc, r) in shift.iter_mut().zip(a.residual) {
                *acc += w * r;
            }
        }
        let moved = Lab { l: x.l + shift[0] / weight_sum, a: x.a + shift[1] / weight_sum, b: x.b + shift[2] / weight_sum };
        lab_to_linear(moved).map(|c| c.clamp(0.0, 1.0))
    }
}

/// Whether anchoring would change anything for this palette.
pub(crate) fn is_active(palette: &Palette) -> bool {
    Mapping::new(palette).is_some()
}

/// Apply the anchored mapping to linear-RGB pixels in place.
pub(crate) fn anchor_map(pixels: &mut [[f64; 3]], palette: &Palette) {
    let Some(mapping) = Mapping::new(palette) else { return };
    pixels.par_iter_mut().for_each(|px| *px = mapping.apply(*px));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color_space::{linear_channel_to_srgb, srgb_channel_to_linear};
    use crate::lab::{delta_e76, srgb_to_lab};
    use crate::palette::{Ink, Palette, PaletteEntry};

    use crate::palette::fixtures::spectra6;

    fn linear(rgb: [u8; 3]) -> [f64; 3] {
        rgb.map(srgb_channel_to_linear)
    }

    fn lab_of_linear(px: [f64; 3]) -> crate::lab::Lab {
        srgb_to_lab(px.map(linear_channel_to_srgb))
    }

    #[test]
    fn every_ink_nominal_lands_on_its_matching_colour() {
        let p = spectra6();
        for (i, ink) in p.inks() {
            let mut px = vec![linear(ink.nominal)];
            anchor_map(&mut px, &p);
            let target = srgb_to_lab(p.preview_srgb()[i as usize]);
            let de = delta_e76(lab_of_linear(px[0]), target);
            assert!(de < 1.0, "{}: ΔE {de:.3} from its appearance", ink.name);
        }
    }

    #[test]
    fn every_ink_nominal_lands_exactly_in_linear_rgb() {
        let p = spectra6();
        for (i, ink) in p.inks() {
            let mut px = vec![linear(ink.nominal)];
            anchor_map(&mut px, &p);
            let target = linear(p.preview_srgb()[i as usize]);
            for c in 0..3 {
                assert!((px[0][c] - target[c]).abs() < 1e-9, "{} channel {c}: {} vs {}", ink.name, px[0][c], target[c]);
            }
        }
    }

    #[test]
    fn neutral_axis_is_monotonic() {
        let p = spectra6();
        let mut ramp: Vec<[f64; 3]> = (0..=255u8).map(|v| linear([v, v, v])).collect();
        anchor_map(&mut ramp, &p);
        let ls: Vec<f64> = ramp.iter().map(|&px| crate::color_space_lab::rgb_to_oklab(px[0], px[1], px[2]).l).collect();
        for w in ls.windows(2) {
            assert!(w[1] >= w[0] - 1e-12, "lightness fell from {} to {}", w[0], w[1]);
        }
    }

    #[test]
    fn continuous() {
        let p = spectra6();
        let step = 1.0 / 255.0;
        for r in 0..16 {
            for g in 0..16 {
                for b in 0..16 {
                    let base = [r as f64 * 17.0, g as f64 * 17.0, b as f64 * 17.0].map(|v| v / 255.0);
                    let next = [(base[0] + step).min(1.0), base[1], base[2]];
                    let mut px = vec![base.map(crate::color_space::srgb_fraction_to_linear), next.map(crate::color_space::srgb_fraction_to_linear)];
                    anchor_map(&mut px, &p);
                    let de = delta_e76(lab_of_linear(px[0]), lab_of_linear(px[1]));
                    assert!(de < 3.0, "jump of ΔE {de:.2} at {base:?}");
                }
            }
        }
    }

    /// spectra6 with only the inks in `mask` (bit i = entry i) named.
    fn spectra6_subset(mask: u32) -> Palette {
        let entries = spectra6()
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| PaletteEntry { appearance: e.appearance, ink: if mask & (1 << i) != 0 { e.ink.clone() } else { None } })
            .collect();
        Palette::new(entries, true).unwrap()
    }

    #[test]
    fn grey_ramp_stays_monotonic_for_every_subset_of_named_inks() {
        for mask in 0..64u32 {
            let p = spectra6_subset(mask);
            let mut ramp: Vec<[f64; 3]> = (0..=255u8).map(|v| linear([v, v, v])).collect();
            anchor_map(&mut ramp, &p);
            let ls: Vec<f64> = ramp.iter().map(|&px| crate::lab::linear_to_lab(px).l).collect();
            for (i, w) in ls.windows(2).enumerate() {
                assert!(w[1] >= w[0] - 1e-9, "inks {mask:06b}: L* fell from {:.2} to {:.2} at grey {i}", w[0], w[1]);
            }
        }
    }

    #[test]
    fn white_stays_white_for_every_subset_of_named_inks() {
        // Entry 1 is white and the lightest entry, named or not.
        for mask in 0..64u32 {
            let p = spectra6_subset(mask);
            let mut px = vec![linear([255, 255, 255])];
            anchor_map(&mut px, &p);
            let de = delta_e76(lab_of_linear(px[0]), srgb_to_lab([255, 255, 255]));
            assert!(de < 2.0, "inks {mask:06b}: #ffffff moved by ΔE {de:.2}");
        }
    }

    #[test]
    fn no_inks_is_identity() {
        let entries = spectra6().entries().iter().map(|e| PaletteEntry { appearance: e.appearance, ink: None }).collect();
        let p = Palette::new(entries, true).unwrap();
        let original: Vec<[f64; 3]> = (0..64u8).map(|v| linear([v * 4, 255 - v * 4, v])).collect();
        let mut px = original.clone();
        anchor_map(&mut px, &p);
        assert_eq!(px, original);
    }

    #[test]
    fn midtones_far_from_every_ink_barely_move() {
        // Anchoring corrects colours near inks; it must not re-map a photo's whole tonal range.
        let mut px = vec![linear([128, 128, 128])];
        anchor_map(&mut px, &spectra6());
        let de = delta_e76(lab_of_linear(px[0]), srgb_to_lab([128, 128, 128]));
        assert!(de < 3.0, "mid grey moved by ΔE {de:.2}");
    }

    #[test]
    fn ideal_palette_is_identity() {
        let p = Palette::from_srgb(
            vec![
                ([0, 0, 0], Some(Ink { name: "black".into(), nominal: [0, 0, 0] })),
                ([255, 255, 255], Some(Ink { name: "white".into(), nominal: [255, 255, 255] })),
                ([255, 0, 0], Some(Ink { name: "red".into(), nominal: [255, 0, 0] })),
            ],
            false,
        )
        .unwrap();
        let original: Vec<[f64; 3]> = (0..64u8).map(|v| linear([v * 4, 128, 255 - v * 4])).collect();
        let mut px = original.clone();
        anchor_map(&mut px, &p);
        for (a, b) in px.iter().zip(&original) {
            for c in 0..3 {
                assert!((a[c] - b[c]).abs() < 1e-9);
            }
        }
    }
}

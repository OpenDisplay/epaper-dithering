//! Anchored mapping: move each named ink's nominal colour onto the colour the panel
//! actually shows for it, and carry the colours around it along smoothly.
//!
//! Two parts, both in CIELAB. (Not OKLab: its cube root has no linear toe, so near black a
//! single sRGB step is a large OKLab distance, and lifting black into the visible range
//! would turn that into a visible jump. CIELAB's linear segment keeps shadows smooth.)
//!
//! 1. **Global black-point compensation** along the grey axis: pure black (`#000000`) lands
//!    on the ink named for it, or else on the palette's darkest entry; pure white
//!    (`#ffffff`) likewise on the ink named for it or the lightest entry. L\* is rescaled
//!    linearly between the two ends and a\*/b\* get their offsets blended by lightness.
//!    Other named inks never drive this step: fitting it between, say, red and blue would
//!    invert the grey axis. This replaces the old dynamic-range ("tone") compression.
//! 2. **Damped local correction** for every named ink (and a zero correction pinned at an
//!    unnamed black or white end): after step 1 each ink still has a residual
//!    (its target minus where step 1 put its nominal). A pixel moves by
//!    `Σ w_k r_k / (Σ w_k + W0)` with `w_k = 1/d_k²`. At an ink the weight is unbounded, so
//!    the ink lands exactly; far from every ink the `W0` term wins and the correction fades
//!    to zero. (Undamped inverse-distance weighting gives far-away colours the *average* of
//!    all residuals, which muddied whole photos: a saturated ink like `#00ff00` measures
//!    far darker, and that pulled every colour down.)
//!
//! A no-op when nothing moves: the palette's darkest and lightest entries are pure black
//! and white and every named ink's appearance equals its nominal (an idealised palette).

use rayon::prelude::*;

use crate::color_space::srgb_channel_to_linear;
use crate::lab::{lab_to_linear, linear_to_lab, Lab};
use crate::palette::Palette;

/// CIELAB distance below which a pixel counts as sitting on an anchor.
const SNAP_DISTANCE: f64 = 1e-9;
/// Displacements smaller than this are treated as zero (idealised palettes).
const NEGLIGIBLE: f64 = 1e-9;
/// CIELAB radius at which an ink's local correction has fallen to half strength.
const LOCAL_RADIUS: f64 = 25.0;

fn linear(rgb: [u8; 3]) -> [f64; 3] {
    rgb.map(srgb_channel_to_linear)
}

/// Step 1: black-point compensation between two (source → target) anchors.
struct Global {
    dark_source: Lab,
    dark_target: Lab,
    light_source: Lab,
    light_target: Lab,
}

impl Global {
    fn apply(&self, x: Lab) -> Lab {
        let span = self.light_source.l - self.dark_source.l;
        let scale = (self.light_target.l - self.dark_target.l) / span;
        let t = ((x.l - self.dark_source.l) / span).clamp(0.0, 1.0);
        let offset = |dark: f64, light: f64| dark + t * (light - dark);
        Lab {
            l: self.dark_target.l + (x.l - self.dark_source.l) * scale,
            a: x.a + offset(self.dark_target.a - self.dark_source.a, self.light_target.a - self.light_source.a),
            b: x.b + offset(self.dark_target.b - self.dark_source.b, self.light_target.b - self.light_source.b),
        }
    }
}

/// Step 2 anchor: where step 1 puts the ink's nominal, and what is still missing.
struct Local {
    source: Lab,
    residual: [f64; 3],
    target_linear: [f64; 3],
}

struct Mapping {
    global: Option<Global>,
    locals: Vec<Local>,
}

impl Mapping {
    fn new(palette: &Palette) -> Option<Self> {
        let inks: Vec<(Lab, Lab, [f64; 3])> = palette
            .inks()
            .map(|(i, ink)| {
                let target_linear = linear(palette.preview_srgb()[i as usize]);
                (linear_to_lab(linear(ink.nominal)), linear_to_lab(target_linear), target_linear)
            })
            .collect();
        let named = |rgb: [u8; 3]| palette.inks().position(|(_, ink)| ink.nominal == rgb).map(|k| inks[k]);
        // The panel's darkest and lightest colours, as (target Lab, target linear RGB).
        let entries: Vec<(Lab, [f64; 3])> = palette
            .preview_srgb()
            .iter()
            .map(|&rgb| (linear_to_lab(linear(rgb)), linear(rgb)))
            .collect();
        let darkest = *entries.iter().min_by(|a, b| a.0.l.total_cmp(&b.0.l)).expect("palettes have entries");
        let lightest = *entries.iter().max_by(|a, b| a.0.l.total_cmp(&b.0.l)).expect("palettes have entries");
        let black_point = Lab { l: 0.0, a: 0.0, b: 0.0 };
        let white_point = Lab { l: 100.0, a: 0.0, b: 0.0 };
        // Ends of the grey axis: the ink named for pure black/white, else pure black/white
        // mapped onto the palette's darkest/lightest entry (black-point compensation).
        let dark = named([0, 0, 0]).unwrap_or((black_point, darkest.0, darkest.1));
        let light = named([255, 255, 255]).unwrap_or((white_point, lightest.0, lightest.1));
        let moves_ends = dark.0 != dark.1 || light.0 != light.1;
        let rising = light.1.l - dark.1.l > 1.0 && light.0.l - dark.0.l > 1.0;
        let global = (moves_ends && rising).then_some(Global {
            dark_source: dark.0,
            dark_target: dark.1,
            light_source: light.0,
            light_target: light.1,
        });
        let mut locals: Vec<Local> = inks
            .iter()
            .map(|&(source, target, target_linear)| {
                let moved = global.as_ref().map_or(source, |g| g.apply(source));
                Local {
                    source: moved,
                    residual: [target.l - moved.l, target.a - moved.a, target.b - moved.b],
                    target_linear,
                }
            })
            .collect();
        // An unnamed end of the grey axis is still a fixed point: pin it with a zero
        // correction so other inks' local corrections fade out before reaching it.
        for (is_named, end) in [(named([0, 0, 0]).is_some(), dark), (named([255, 255, 255]).is_some(), light)] {
            if !is_named {
                let at = global.as_ref().map_or(end.0, |g| g.apply(end.0));
                locals.push(Local { source: at, residual: [0.0; 3], target_linear: end.2 });
            }
        }
        let local_moves = locals.iter().any(|l| l.residual.iter().any(|r| r.abs() > NEGLIGIBLE));
        (global.is_some() || local_moves).then_some(Self { global, locals })
    }

    fn apply(&self, px: [f64; 3]) -> [f64; 3] {
        let x = linear_to_lab(px);
        let y = self.global.as_ref().map_or(x, |g| g.apply(x));
        let mut weight_sum = 1.0 / (LOCAL_RADIUS * LOCAL_RADIUS);
        let mut shift = [0.0; 3];
        for a in &self.locals {
            let d2 = (y.l - a.source.l).powi(2) + (y.a - a.source.a).powi(2) + (y.b - a.source.b).powi(2);
            if d2 < SNAP_DISTANCE * SNAP_DISTANCE {
                return a.target_linear;
            }
            let w = 1.0 / d2;
            weight_sum += w;
            for (acc, r) in shift.iter_mut().zip(a.residual) {
                *acc += w * r;
            }
        }
        let moved = Lab { l: y.l + shift[0] / weight_sum, a: y.a + shift[1] / weight_sum, b: y.b + shift[2] / weight_sum };
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

    fn spectra6() -> Palette {
        Palette::from_json(include_str!("../../../../examples/palettes/spectra_7_3_6color.json")).unwrap()
    }

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
    fn unnamed_palette_maps_black_and_white_onto_its_darkest_and_lightest_entries() {
        let entries = spectra6().entries().iter().map(|e| PaletteEntry { appearance: e.appearance, ink: None }).collect();
        let p = Palette::new(entries, true).unwrap();
        let darkest = p.preview_srgb().iter().copied().min_by_key(|c| c.iter().map(|&v| v as u32).sum::<u32>()).unwrap();
        let mut px = vec![linear([0, 0, 0]), linear([255, 255, 255])];
        anchor_map(&mut px, &p);
        assert_eq!(px[0], linear(darkest), "#000000 lands on the darkest entry");
        assert_eq!(px[1], linear([255, 255, 255]), "#ffffff stays on the (normalised) white entry");
    }

    #[test]
    fn unnormalised_unnamed_palette_maps_white_onto_its_lightest_entry() {
        let p = Palette::from_srgb(vec![([30, 30, 30], None), ([200, 200, 200], None)], false).unwrap();
        let mut px = vec![linear([0, 0, 0]), linear([255, 255, 255])];
        anchor_map(&mut px, &p);
        assert_eq!(px, vec![linear([30, 30, 30]), linear([200, 200, 200])]);
    }

    #[test]
    fn unnamed_palette_grey_ramp_is_monotonic() {
        let entries = spectra6().entries().iter().map(|e| PaletteEntry { appearance: e.appearance, ink: None }).collect();
        let p = Palette::new(entries, true).unwrap();
        let mut ramp: Vec<[f64; 3]> = (0..=255u8).map(|v| linear([v, v, v])).collect();
        anchor_map(&mut ramp, &p);
        let ls: Vec<f64> = ramp.iter().map(|&px| crate::lab::linear_to_lab(px).l).collect();
        assert!(ls.windows(2).all(|w| w[1] >= w[0] - 1e-9));
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

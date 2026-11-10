//! Anchored mapping: move each named ink's nominal colour onto the colour the panel
//! actually shows for it, and carry the colours around it along smoothly.
//!
//! Two parts, both in CIELAB. (Not OKLab: its cube root has no linear toe, so near black a
//! single sRGB step is a large OKLab distance, and lifting black into the visible range
//! would turn that into a visible jump. CIELAB's linear segment keeps shadows smooth.)
//!
//! 1. **Global black-point compensation** between the darkest and the lightest ink: L\* is
//!    rescaled linearly so the darkest nominal lands on its appearance and the lightest
//!    nominal on its appearance; a\*/b\* get the two inks' offsets blended by lightness.
//!    Monotonic in lightness, smooth, and it moves every colour by a sensible amount.
//! 2. **Damped local correction** for every ink: after step 1 each ink still has a residual
//!    (its target minus where step 1 put its nominal). A pixel moves by
//!    `Σ w_k r_k / (Σ w_k + W0)` with `w_k = 1/d_k²`. At an ink the weight is unbounded, so
//!    the ink lands exactly; far from every ink the `W0` term wins and the correction fades
//!    to zero. (Undamped inverse-distance weighting gives far-away colours the *average* of
//!    all residuals, which muddied whole photos: a saturated ink like `#00ff00` measures
//!    far darker, and that pulled every colour down.)
//!
//! A no-op when the palette has no inks or every appearance equals its nominal.

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
        let moves = inks.iter().any(|(s, t, _)| {
            [t.l - s.l, t.a - s.a, t.b - s.b].iter().any(|d| d.abs() > NEGLIGIBLE)
        });
        if !moves {
            return None;
        }
        let darkest = inks.iter().min_by(|a, b| a.0.l.total_cmp(&b.0.l));
        let lightest = inks.iter().max_by(|a, b| a.0.l.total_cmp(&b.0.l));
        let global = match (darkest, lightest) {
            (Some(d), Some(l)) if l.0.l - d.0.l > 1.0 => Some(Global {
                dark_source: d.0,
                dark_target: d.1,
                light_source: l.0,
                light_target: l.1,
            }),
            _ => None,
        };
        let locals = inks
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
        Some(Self { global, locals })
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

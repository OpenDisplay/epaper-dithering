//! CIELAB (D65) conversions used by runtime palettes.

use crate::color_space::{linear_channel_to_srgb, srgb_channel_to_linear};

/// A CIELAB colour, D65 reference white.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// D65 reference white in XYZ: the row sums of `SRGB_TO_XYZ`, so sRGB white maps to
/// exactly L\* 100, a\* = b\* = 0.
const D65: [f64; 3] = [
    SRGB_TO_XYZ[0][0] + SRGB_TO_XYZ[0][1] + SRGB_TO_XYZ[0][2],
    SRGB_TO_XYZ[1][0] + SRGB_TO_XYZ[1][1] + SRGB_TO_XYZ[1][2],
    SRGB_TO_XYZ[2][0] + SRGB_TO_XYZ[2][1] + SRGB_TO_XYZ[2][2],
];

// Linear sRGB → XYZ (D65), IEC 61966-2-1.
const SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.412_456_4, 0.357_576_1, 0.180_437_5],
    [0.212_672_9, 0.715_152_2, 0.072_175_0],
    [0.019_333_9, 0.119_192_0, 0.950_304_1],
];

// Bradford cone response matrix.
const BRADFORD: [[f64; 3]; 3] = [
    [0.895_1, 0.266_4, -0.161_4],
    [-0.750_2, 1.713_5, 0.036_7],
    [0.038_9, -0.068_5, 1.029_6],
];

fn mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn invert(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    [
        [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
        [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
        [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det],
    ]
}

const EPSILON: f64 = 216.0 / 24_389.0;
const KAPPA: f64 = 24_389.0 / 27.0;

/// XYZ (D65, Y = 1 for white) → CIELAB.
pub fn xyz_to_lab(xyz: [f64; 3]) -> Lab {
    let f = |t: f64| if t > EPSILON { t.cbrt() } else { (KAPPA * t + 16.0) / 116.0 };
    let fx = f(xyz[0] / D65[0]);
    let fy = f(xyz[1] / D65[1]);
    let fz = f(xyz[2] / D65[2]);
    Lab { l: 116.0 * fy - 16.0, a: 500.0 * (fx - fy), b: 200.0 * (fy - fz) }
}

/// CIELAB → XYZ (D65, Y = 1 for white).
pub fn lab_to_xyz(lab: Lab) -> [f64; 3] {
    let fy = (lab.l + 16.0) / 116.0;
    let fx = fy + lab.a / 500.0;
    let fz = fy - lab.b / 200.0;
    let finv = |f: f64| {
        let f3 = f * f * f;
        if f3 > EPSILON { f3 } else { (116.0 * f - 16.0) / KAPPA }
    };
    let y = if lab.l > KAPPA * EPSILON { fy * fy * fy } else { lab.l / KAPPA };
    [finv(fx) * D65[0], y * D65[1], finv(fz) * D65[2]]
}

/// 8-bit sRGB → CIELAB.
pub fn srgb_to_lab(rgb: [u8; 3]) -> Lab {
    let linear = rgb.map(srgb_channel_to_linear);
    xyz_to_lab(mul(&SRGB_TO_XYZ, linear))
}

/// CIELAB → 8-bit sRGB, clamping out-of-gamut colours.
pub fn lab_to_srgb(lab: Lab) -> [u8; 3] {
    let linear = mul(&invert(&SRGB_TO_XYZ), lab_to_xyz(lab));
    linear.map(|c| linear_channel_to_srgb(c.clamp(0.0, 1.0)))
}

/// CIE76 colour difference.
pub fn delta_e76(x: Lab, y: Lab) -> f64 {
    ((x.l - y.l).powi(2) + (x.a - y.a).powi(2) + (x.b - y.b).powi(2)).sqrt()
}

/// Bradford-adapt `colour` so that `white` becomes D65 white (L\* 100, a\* = b\* = 0).
///
/// Both inputs are absolute measurements in the same (D65-referenced) Lab space.
pub fn adapt_to_white(colour: Lab, white: Lab) -> Lab {
    let src = mul(&BRADFORD, lab_to_xyz(white));
    let dst = mul(&BRADFORD, D65);
    let cone = mul(&BRADFORD, lab_to_xyz(colour));
    let scaled = [cone[0] * dst[0] / src[0], cone[1] * dst[1] / src[1], cone[2] * dst[2] / src[2]];
    xyz_to_lab(mul(&invert(&BRADFORD), scaled))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn srgb_white_is_lab_100() {
        let lab = srgb_to_lab([255, 255, 255]);
        assert_relative_eq!(lab.l, 100.0, epsilon = 1e-6);
        assert_relative_eq!(lab.a, 0.0, epsilon = 1e-6);
        assert_relative_eq!(lab.b, 0.0, epsilon = 1e-6);
        assert_eq!(lab_to_srgb(Lab { l: 100.0, a: 0.0, b: 0.0 }), [255, 255, 255]);
    }

    #[test]
    fn srgb_black_is_lab_0() {
        let lab = srgb_to_lab([0, 0, 0]);
        assert_relative_eq!(lab.l, 0.0, epsilon = 1e-6);
        assert_eq!(lab_to_srgb(Lab { l: 0.0, a: 0.0, b: 0.0 }), [0, 0, 0]);
    }

    #[test]
    fn srgb_round_trips_through_lab() {
        for r in (0..=255u16).step_by(17) {
            for g in (0..=255u16).step_by(17) {
                for b in (0..=255u16).step_by(17) {
                    let c = [r as u8, g as u8, b as u8];
                    assert_eq!(lab_to_srgb(srgb_to_lab(c)), c, "round trip of {c:?}");
                }
            }
        }
    }

    #[test]
    fn delta_e76_of_identical_colours_is_zero() {
        let c = Lab { l: 52.0, a: 8.9, b: 2.1 };
        assert_eq!(delta_e76(c, c), 0.0);
        assert_relative_eq!(delta_e76(c, Lab { l: 55.0, a: 12.9, b: 2.1 }), 5.0, epsilon = 1e-12);
    }

    #[test]
    fn adapting_the_white_itself_gives_d65_white() {
        let white = Lab { l: 68.26, a: 8.33, b: 5.98 };
        let w = adapt_to_white(white, white);
        assert_relative_eq!(w.l, 100.0, epsilon = 1e-9);
        assert_relative_eq!(w.a, 0.0, epsilon = 1e-9);
        assert_relative_eq!(w.b, 0.0, epsilon = 1e-9);
    }

    #[test]
    fn adapting_to_d65_white_is_identity() {
        let c = Lab { l: 40.0, a: 20.0, b: -10.0 };
        let out = adapt_to_white(c, Lab { l: 100.0, a: 0.0, b: 0.0 });
        assert_relative_eq!(out.l, c.l, epsilon = 1e-9);
        assert_relative_eq!(out.a, c.a, epsilon = 1e-9);
        assert_relative_eq!(out.b, c.b, epsilon = 1e-9);
    }
}

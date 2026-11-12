//! Dynamic range and gamut compression for measured e-paper palettes.
//!
//! Applied before dithering to map image content into the display's reproducible range.
//! All functions operate on linear RGB pixels (`&mut [[f64; 3]]`).

use rayon::prelude::*;

use crate::color_space::{linear_fraction_to_srgb, srgb_fraction_to_linear};
use crate::color_space_lab::{oklab_to_rgb, rgb_to_oklab, OkLab};
use crate::lab::SRGB_TO_XYZ;
use crate::srgb_palette::SrgbPalette;

/// Relative luminance (CIE Y) of a linear-RGB pixel.
fn luminance([r, g, b]: [f64; 3]) -> f64 {
    let [lum_r, lum_g, lum_b] = SRGB_TO_XYZ[1];
    lum_r * r + lum_g * g + lum_b * b
}

// ── apply_exposure ────────────────────────────────────────────────────────────

/// Apply exposure adjustment (linear multiply on all channels).
///
/// `factor > 1.0` brightens, `< 1.0` darkens. 1.0 is a no-op fast path.
/// Operates on linear RGB pixels.
pub fn apply_exposure(pixels: &mut [[f64; 3]], factor: f64) {
    if (factor - 1.0).abs() < 1e-9 {
        return;
    }
    pixels.par_iter_mut().for_each(|pixel| {
        pixel[0] = (pixel[0] * factor).clamp(0.0, 1.0);
        pixel[1] = (pixel[1] * factor).clamp(0.0, 1.0);
        pixel[2] = (pixel[2] * factor).clamp(0.0, 1.0);
    });
}

// ── adjust_saturation ─────────────────────────────────────────────────────────

/// Adjust saturation in OKLab space. `factor > 1.0` boosts, `< 1.0` reduces, 1.0 is identity.
///
/// Scales `a` and `b` components equally, which scales chroma while keeping the hue
/// (`atan2(b, a)`) unchanged. Operates on linear RGB pixels.
pub fn adjust_saturation(pixels: &mut [[f64; 3]], factor: f64) {
    if (factor - 1.0).abs() < 1e-9 {
        return;
    }
    pixels.par_iter_mut().for_each(|pixel| {
        let lab = rgb_to_oklab(pixel[0], pixel[1], pixel[2]);
        let scaled = OkLab {
            l: lab.l,
            a: (lab.a * factor).clamp(-1.0, 1.0),
            b: (lab.b * factor).clamp(-1.0, 1.0),
        };
        *pixel = oklab_to_rgb(scaled);
    });
}

// ── apply_shadows_highlights ──────────────────────────────────────────────────

/// Apply shadow lift and highlight compression as a luminance-only S-curve.
///
/// `shadows` controls lift on the lower half (0.0 = identity, 1.0 = strong; power = 1 − shadows).
/// `highlights` controls compression on the upper half (0.0 = identity, 1.0 = strong; power = 1 + highlights).
///
/// The curve pivots about **perceptual mid-gray** (sRGB 0.5 ≈ linear 0.214), not linear 0.5
/// (≈ sRGB 188). Pivoting in linear space would put ~74% of the perceptual tonal range in the
/// "shadows" half; gamma-space pivoting splits shadows/highlights at mid-gray as a user expects.
///
/// Each half is independent — `shadows = 0` leaves shadows alone even if `highlights > 0`.
/// Hue is preserved by scaling all three channels by `Y' / Y`.
pub fn apply_shadows_highlights(pixels: &mut [[f64; 3]], shadows: f64, highlights: f64) {
    if shadows <= 0.0 && highlights <= 0.0 {
        return;
    }
    // Guard against degenerate exponents (negative → curve inversion, >1 → runaway).
    let shadows = shadows.clamp(0.0, 1.0);
    let highlights = highlights.clamp(0.0, 1.0);
    pixels.par_iter_mut().for_each(|pixel| {
        let y = luminance(*pixel);
        if y <= 1e-6 {
            return;
        }
        // Apply the S-curve in gamma-encoded space so the pivot is at perceptual mid-gray.
        let g = linear_fraction_to_srgb(y);
        let g_prime = if g <= 0.5 {
            let t = g / 0.5;
            0.5 * t.powf(1.0 - shadows)
        } else {
            let t = (g - 0.5) / 0.5;
            0.5 + 0.5 * t.powf(1.0 + highlights)
        };
        let y_prime = srgb_fraction_to_linear(g_prime);
        let scale = (y_prime / y).clamp(0.0, 1e6);
        pixel[0] = (pixel[0] * scale).clamp(0.0, 1.0);
        pixel[1] = (pixel[1] * scale).clamp(0.0, 1.0);
        pixel[2] = (pixel[2] * scale).clamp(0.0, 1.0);
    });
}

/// Luminance of the palette's darkest and lightest entries: the display's black and
/// white, whatever order the palette lists them in.
fn display_luminance_range(palette: &SrgbPalette) -> (f64, f64) {
    palette
        .linear()
        .into_iter()
        .map(luminance)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| (lo.min(y), hi.max(y)))
}

/// Scale a near-black pixel toward a target luminance while preserving its channel ratios.
/// When the pixel is exactly black, all channels are set to `target_y` (neutral).
fn scale_dark_pixel(pixel: &mut [f64; 3], target_y: f64) {
    let max_ch = pixel[0].max(pixel[1]).max(pixel[2]);
    if max_ch > 1e-12 {
        let scale = target_y / max_ch;
        for c in pixel.iter_mut() {
            *c = (*c * scale).clamp(0.0, 1.0);
        }
    } else {
        *pixel = [target_y; 3];
    }
}

// ── compress_dynamic_range (tone) ─────────────────────────────────────────────

/// Lift the black point: remap luminance from [0, 1] onto the display's
/// [black_Y, white_Y] range (its darkest and lightest entries).
///
/// Only lightness changes: each pixel's channels are scaled together, so hue and chroma
/// ratios are kept. `strength` blends between no change (0.0) and the full remap (1.0).
pub fn compress_dynamic_range(pixels: &mut [[f64; 3]], palette: &SrgbPalette, strength: f64) {
    if strength <= 0.0 {
        return;
    }
    let (black_y, white_y) = display_luminance_range(palette);
    let display_range = white_y - black_y;
    if display_range <= 0.0 {
        return;
    }
    pixels.par_iter_mut().for_each(|pixel| {
        let y = luminance(*pixel);
        let compressed_y = black_y + y * display_range;
        let target_y = y + strength * (compressed_y - y);
        if y > 1e-6 {
            let scale = (target_y / y).clamp(0.0, 1e6);
            for c in pixel.iter_mut() {
                *c = (*c * scale).clamp(0.0, 1.0);
            }
        } else {
            scale_dark_pixel(pixel, black_y * strength);
        }
    });
}

// ── gamut_compress ────────────────────────────────────────────────────────────

const GAMUT_THRESHOLD: f64 = 0.15;
const GAMUT_THRESHOLD_MAX: f64 = 0.45;

/// Nearest point on a line segment [a, b] to point p, in 3D.
/// Returns the interpolation parameter t ∈ [0, 1].
fn nearest_on_segment(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let edge = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let edge_len_sq = edge[0] * edge[0] + edge[1] * edge[1] + edge[2] * edge[2];
    if edge_len_sq < 1e-10 {
        return 0.0;
    }
    let diff = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let dot = diff[0] * edge[0] + diff[1] * edge[1] + diff[2] * edge[2];
    (dot / edge_len_sq).clamp(0.0, 1.0)
}

fn dist_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blend out-of-gamut pixels toward the nearest point on the palette hull in OKLab space.
///
/// Smoothstep from no effect at `GAMUT_THRESHOLD` to full at `GAMUT_THRESHOLD_MAX`.
///
/// Note: searches all palette edges (O(n²) pairs) and relies on `nearest_on_segment`
/// returning an endpoint when the projection falls outside the segment, which covers all
/// vertices. For 4+ color palettes the true nearest point on the convex hull can be
/// interior to a triangular face — this is a known approximation.
pub fn gamut_compress(pixels: &mut [[f64; 3]], palette: &SrgbPalette, strength: f64) {
    if strength <= 0.0 {
        return;
    }

    let pal_linear = palette.linear();
    let pal_lab: Vec<_> = pal_linear
        .iter()
        .map(|&[r, g, b]| rgb_to_oklab(r, g, b))
        .collect();
    let n = pal_linear.len();

    // Precompute the palette-hull edge geometry once (OKLab endpoints + linear endpoints for
    // all i<j pairs) instead of rebuilding these arrays inside the per-pixel loop.
    struct Edge {
        a_lab: [f64; 3],
        b_lab: [f64; 3],
        a_lin: [f64; 3],
        b_lin: [f64; 3],
    }
    let edges: Vec<Edge> = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .map(|(i, j)| Edge {
            a_lab: [pal_lab[i].l, pal_lab[i].a, pal_lab[i].b],
            b_lab: [pal_lab[j].l, pal_lab[j].a, pal_lab[j].b],
            a_lin: pal_linear[i],
            b_lin: pal_linear[j],
        })
        .collect();

    pixels.par_iter_mut().for_each(|pixel| {
        let px_lab = rgb_to_oklab(pixel[0], pixel[1], pixel[2]);
        let px_lab_arr = [px_lab.l, px_lab.a, px_lab.b];

        // Nearest point on any palette edge. Vertices need no separate pass: the clamped
        // projection returns an endpoint when it falls outside the segment.
        let mut best_dist_sq = f64::INFINITY;
        let mut best_target = *pixel;
        for e in &edges {
            let t = nearest_on_segment(px_lab_arr, e.a_lab, e.b_lab);
            let nearest_lab = lerp3(e.a_lab, e.b_lab, t);
            let d = dist_sq(px_lab_arr, nearest_lab);
            if d < best_dist_sq {
                best_dist_sq = d;
                best_target = lerp3(e.a_lin, e.b_lin, t);
            }
        }

        let nearest_dist = best_dist_sq.sqrt();
        let blend = smoothstep(GAMUT_THRESHOLD, GAMUT_THRESHOLD_MAX, nearest_dist) * strength;

        pixel[0] = (pixel[0] + blend * (best_target[0] - pixel[0])).clamp(0.0, 1.0);
        pixel[1] = (pixel[1] + blend * (best_target[1] - pixel[1])).clamp(0.0, 1.0);
        pixel[2] = (pixel[2] + blend * (best_target[2] - pixel[2])).clamp(0.0, 1.0);
    });
}


// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    fn spectra_palette() -> &'static SrgbPalette {
            crate::srgb_palette::fixtures::spectra6()
    }

    #[test]
    fn tone_zero_is_identity() {
        let original = vec![[0.0, 0.0, 0.0], [0.2, 0.1, 0.05], [1.0, 1.0, 1.0]];
        let mut px = original.clone();
        compress_dynamic_range(&mut px, spectra_palette(), 0.0);
        assert_eq!(px, original);
    }

    #[test]
    fn full_tone_maps_black_onto_the_panel_black_and_keeps_white() {
        let pal = spectra_palette();
        let (black_y, white_y) = display_luminance_range(pal);
        let mut px = vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
        compress_dynamic_range(&mut px, pal, 1.0);
        approx::assert_relative_eq!(luminance(px[0]), black_y, epsilon = 1e-6);
        approx::assert_relative_eq!(luminance(px[1]), white_y, epsilon = 1e-6);
    }

    #[test]
    fn tone_keeps_hue_by_scaling_channels_together() {
        let mut px = vec![[0.30, 0.10, 0.05]];
        compress_dynamic_range(&mut px, spectra_palette(), 1.0);
        approx::assert_relative_eq!(px[0][0] / px[0][1], 3.0, epsilon = 1e-9);
        approx::assert_relative_eq!(px[0][1] / px[0][2], 2.0, epsilon = 1e-9);
    }

    #[test]
    fn gamut_compress_identity_on_palette_colors() {
        // A pixel that exactly matches a palette color should be unchanged
        let pal = spectra_palette();
        let mut pixels = vec![pal.linear()[0]];
        let original = pixels[0];
        gamut_compress(&mut pixels, pal, 1.0);
        let d = dist_sq(pixels[0], original);
        assert!(d < 1e-10, "palette color should not be moved: d={d}");
    }

    #[test]
    fn gamut_compress_moves_out_of_gamut_pixel() {
        // Pure linear red [1, 0, 0] is much more vivid than the Spectra palette's desaturated
        // red ink [121, 9, 0] — the OKLab distance to the nearest edge exceeds GAMUT_THRESHOLD,
        // so the smoothstep is non-zero and the pixel must be moved.
        let pal = spectra_palette();
        let mut pixels = vec![[1.0_f64, 0.0, 0.0]]; // pure linear red
        let original = pixels[0];
        gamut_compress(&mut pixels, pal, 1.0);
        let moved = pixels[0].iter().zip(original.iter()).any(|(a, b)| (a - b).abs() > 1e-6);
        assert!(moved, "vivid red should be moved toward the palette by gamut_compress");
    }

    #[test]
    fn exposure_factor_one_is_identity() {
        let mut pixels = vec![[0.5_f64, 0.2, 0.1]];
        let original = pixels.clone();
        apply_exposure(&mut pixels, 1.0);
        assert_eq!(pixels, original);
    }

    #[test]
    fn exposure_factor_greater_than_one_brightens() {
        let mut pixels = vec![[0.2_f64, 0.2, 0.2]];
        apply_exposure(&mut pixels, 2.0);
        assert!(pixels[0][0] > 0.39 && pixels[0][0] < 0.41);
    }

    #[test]
    fn saturation_factor_one_is_identity() {
        let mut pixels = vec![[0.5_f64, 0.2, 0.1]];
        let original = pixels.clone();
        adjust_saturation(&mut pixels, 1.0);
        for (got, expected) in pixels.iter().zip(original.iter()) {
            for (a, b) in got.iter().zip(expected.iter()) {
                assert!((a - b).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn saturation_factor_zero_produces_gray() {
        let mut pixels = vec![[0.8_f64, 0.2, 0.1]];
        adjust_saturation(&mut pixels, 0.0);
        let [r, g, b] = pixels[0];
        assert!(
            (r - g).abs() < 1e-3 && (r - b).abs() < 1e-3,
            "factor=0 must yield neutral gray: [{r}, {g}, {b}]"
        );
    }

    #[test]
    fn shadows_highlights_zero_is_identity() {
        let mut pixels = vec![[0.2_f64, 0.2, 0.2], [0.7, 0.7, 0.7]];
        let original = pixels.clone();
        apply_shadows_highlights(&mut pixels, 0.0, 0.0);
        assert_eq!(pixels, original);
    }

    #[test]
    fn shadows_lifts_only_lower_half() {
        let mut pixels = vec![[0.1_f64, 0.1, 0.1], [0.8, 0.8, 0.8]];
        apply_shadows_highlights(&mut pixels, 0.5, 0.0);
        // Shadow pixel lifted (brighter)
        assert!(pixels[0][0] > 0.1, "shadow should be lifted: {}", pixels[0][0]);
        // Highlight pixel unchanged (highlights=0)
        assert!((pixels[1][0] - 0.8).abs() < 1e-6, "highlight should be unchanged: {}", pixels[1][0]);
    }

}

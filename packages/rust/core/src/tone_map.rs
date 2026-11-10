//! Dynamic range and gamut compression for measured e-paper palettes.
//!
//! Applied before dithering to map image content into the display's reproducible range.
//! All functions operate on linear RGB pixels (`&mut [[f64; 3]]`).

use rayon::prelude::*;

use crate::color_space::{linear_fraction_to_srgb, srgb_channel_to_linear, srgb_fraction_to_linear};
use crate::color_space_lab::{oklab_to_rgb, rgb_to_oklab, OkLab};
use crate::srgb_palette::SrgbPalette;

// ITU-R BT.709 / sRGB luminance coefficients
const LUM_R: f64 = 0.2126729;
const LUM_G: f64 = 0.7151522;
const LUM_B: f64 = 0.0721750;

fn luminance([r, g, b]: [f64; 3]) -> f64 {
    LUM_R * r + LUM_G * g + LUM_B * b
}

fn palette_to_linear(palette: &SrgbPalette) -> Vec<[f64; 3]> {
    palette
        .colors
        .iter()
        .map(|&[r, g, b]| {
            [
                srgb_channel_to_linear(r),
                srgb_channel_to_linear(g),
                srgb_channel_to_linear(b),
            ]
        })
        .collect()
}

/// Luminance of the palette's darkest and lightest entries: the display's black and
/// white, whatever order the palette lists them in.
fn display_luminance_range(palette: &SrgbPalette) -> (f64, f64) {
    palette_to_linear(palette)
        .into_iter()
        .map(luminance)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| (lo.min(y), hi.max(y)))
}

/// Scale a near-black pixel toward a target luminance while preserving its channel ratios
/// (hue/chroma). Used by both dynamic-range compressors for the `y ≈ 0` branch where the
/// `target_y / y` scale would explode. When the pixel is exactly black, all channels are
/// set to `target_y` (neutral).
fn scale_dark_pixel(pixel: &mut [f64; 3], target_y: f64) {
    let max_ch = pixel[0].max(pixel[1]).max(pixel[2]);
    if max_ch > 1e-12 {
        let scale = target_y / max_ch;
        pixel[0] = (pixel[0] * scale).clamp(0.0, 1.0);
        pixel[1] = (pixel[1] * scale).clamp(0.0, 1.0);
        pixel[2] = (pixel[2] * scale).clamp(0.0, 1.0);
    } else {
        pixel[0] = target_y;
        pixel[1] = target_y;
        pixel[2] = target_y;
    }
}

/// Two percentiles from the same data in one sort. Returns (p_low_val, p_high_val).
fn percentile_pair(values: &[f64], p_low: f64, p_high: f64) -> (f64, f64) {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len().saturating_sub(1);
    let idx_lo = ((p_low  / 100.0) * n as f64).round() as usize;
    let idx_hi = ((p_high / 100.0) * n as f64).round() as usize;
    (sorted[idx_lo.min(sorted.len() - 1)], sorted[idx_hi.min(sorted.len() - 1)])
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

// ── compress_dynamic_range ────────────────────────────────────────────────────

/// Remap pixel luminance from [0, 1] to the display's [black_Y, white_Y] range.
///
/// Black and white are the palette's darkest and lightest entries.
/// `strength` blends between no compression (0.0) and full remap (1.0).
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
            pixel[0] = (pixel[0] * scale).clamp(0.0, 1.0);
            pixel[1] = (pixel[1] * scale).clamp(0.0, 1.0);
            pixel[2] = (pixel[2] * scale).clamp(0.0, 1.0);
        } else {
            // Pixel luminance is near zero — preserve channel ratios, scale toward display black.
            scale_dark_pixel(pixel, black_y * strength);
        }
    });
}

// ── auto_compress_dynamic_range ───────────────────────────────────────────────

/// Conditionally compress dynamic range using Reinhard 2004 log-skewness for strength.
///
/// Only compresses when the image genuinely exceeds the display range (±10% tolerance).
/// Strength = clip(skew^1.4, 0, 1) where skew = position of log-average in log range.
pub fn auto_compress_dynamic_range(pixels: &mut [[f64; 3]], palette: &SrgbPalette) {
    let (black_y, white_y) = display_luminance_range(palette);
    let display_range = white_y - black_y;

    if display_range <= 0.0 {
        return;
    }

    let lum_values: Vec<f64> = pixels.iter().map(|&p| luminance(p)).collect();

    let (p_low, p_high) = percentile_pair(&lum_values, 2.0, 98.0);
    let image_range = p_high - p_low;

    if image_range < 1e-6 {
        compress_dynamic_range(pixels, palette, 1.0);
        return;
    }

    const TOLERANCE: f64 = 0.10;
    let fits_shadows = p_low >= black_y - TOLERANCE * display_range;
    let fits_highlights = p_high <= white_y + TOLERANCE * display_range;

    if fits_shadows && fits_highlights {
        return;
    }

    // Reinhard 2004: strength from log-histogram skewness
    let nonzero: Vec<f64> = lum_values.iter().copied().filter(|&y| y > 1e-6).collect();
    let strength = if !nonzero.is_empty() {
        let l_lav = (nonzero.iter().map(|&y| (y + 1e-5).ln()).sum::<f64>() / nonzero.len() as f64).exp();
        let log_min = p_low.max(1e-5).ln();
        let log_max = p_high.max(1e-5).ln();
        let log_range = log_max - log_min;
        if log_range > 1e-6 {
            let skew = (log_max - (l_lav + 1e-5).ln()) / log_range;
            // Clamp before powf: for high-key images the log-average can slightly exceed
            // log_max, making `skew` negative — `(-x).powf(1.4)` is NaN and would poison
            // every output pixel. Clamping first keeps strength in [0, 1].
            skew.clamp(0.0, 1.0).powf(1.4)
        } else {
            1.0
        }
    } else {
        1.0
    };

    // Remap [p_low, p_high] → [black_y, white_y] at computed strength.
    // `normalized` is clamped to [0, 1] so the darkest/brightest percentile outliers
    // saturate at the display black/white points instead of extrapolating past them
    // (unclamped, the bottom 2% would map below display black and crush to pure 0).
    pixels.par_iter_mut().for_each(|pixel| {
        let y = luminance(*pixel);
        let normalized = ((y - p_low) / image_range).clamp(0.0, 1.0);
        let target_y_full = black_y + normalized * display_range;
        let target_y = y + strength * (target_y_full - y);
        if y > 1e-6 {
            let scale = (target_y / y).clamp(0.0, 1e6);
            pixel[0] = (pixel[0] * scale).clamp(0.0, 1.0);
            pixel[1] = (pixel[1] * scale).clamp(0.0, 1.0);
            pixel[2] = (pixel[2] * scale).clamp(0.0, 1.0);
        } else {
            // Near-black: honor strength and preserve chroma (y=0 → normalized=0 → target black_y).
            scale_dark_pixel(pixel, target_y);
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

    let pal_linear = palette_to_linear(palette);
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

        // Find nearest point on any palette edge or vertex
        let mut best_dist_sq = f64::INFINITY;
        let mut best_target = *pixel;

        // Edges (endpoints precomputed above; vertices covered via clamped projection)
        for e in &edges {
            let t = nearest_on_segment(px_lab_arr, e.a_lab, e.b_lab);
            let nearest_lab = lerp3(e.a_lab, e.b_lab, t);
            let d = dist_sq(px_lab_arr, nearest_lab);
            if d < best_dist_sq {
                best_dist_sq = d;
                best_target = lerp3(e.a_lin, e.b_lin, t);
            }
        }

        // Note: vertices are already covered by the edge loop — nearest_on_segment
        // returns the endpoint (t=0 or t=1) when the projection falls outside the segment.

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
    fn compress_strength_zero_is_identity() {
        let mut pixels = vec![[0.8, 0.4, 0.2_f64]];
        let original = pixels[0];
        compress_dynamic_range(&mut pixels, spectra_palette(), 0.0);
        assert_eq!(pixels[0], original);
    }

    #[test]
    fn compress_reduces_highlights() {
        // A bright pixel should be dimmed toward the display white point
        let mut pixels = vec![[1.0, 1.0, 1.0_f64]];
        compress_dynamic_range(&mut pixels, spectra_palette(), 1.0);
        assert!(luminance(pixels[0]) < 1.0, "highlight should be compressed");
    }

    #[test]
    fn gamut_compress_identity_on_palette_colors() {
        // A pixel that exactly matches a palette color should be unchanged
        let pal = spectra_palette();
        let [r, g, b] = pal.colors[0];
        let mut pixels = vec![[
            srgb_channel_to_linear(r),
            srgb_channel_to_linear(g),
            srgb_channel_to_linear(b),
        ]];
        let original = pixels[0];
        gamut_compress(&mut pixels, pal, 1.0);
        let d = dist_sq(pixels[0], original);
        assert!(d < 1e-10, "palette color should not be moved: d={d}");
    }

    #[test]
    fn compress_maps_white_to_display_white() {
        // Full-strength compression should remap luminance-1.0 to the display's white point
        let mut pixels = vec![[1.0, 1.0, 1.0_f64]];
        let pal = spectra_palette();
        compress_dynamic_range(&mut pixels, pal, 1.0);
        let pal_lin = palette_to_linear(pal);
        let white_y = luminance(pal_lin[1]);
        let out_y = luminance(pixels[0]);
        assert!(
            (out_y - white_y).abs() < 1e-6,
            "white pixel should compress to display white point (expected {white_y}, got {out_y})"
        );
    }

    #[test]
    fn compress_maps_black_to_display_black() {
        // Full-strength compression of pure black should yield the display black point
        let mut pixels = vec![[0.0, 0.0, 0.0_f64]];
        let pal = spectra_palette();
        compress_dynamic_range(&mut pixels, pal, 1.0);
        let pal_lin = palette_to_linear(pal);
        let black_y = luminance(pal_lin[0]);
        let out_y = luminance(pixels[0]);
        assert!(
            (out_y - black_y).abs() < 1e-4,
            "black pixel should compress to display black point (expected {black_y}, got {out_y})"
        );
    }

    #[test]
    fn auto_compress_skips_in_range_images() {
        // Images whose luminance already fits within the display range should not be modified
        let pal = spectra_palette();
        let pal_lin = palette_to_linear(pal);
        let black_y = luminance(pal_lin[0]);
        let white_y = luminance(pal_lin[1]);
        // A pixel sitting at the midpoint of the display range is well within bounds
        let mid = (black_y + white_y) / 2.0;
        // Use enough pixels to avoid the flat-image fast path (image_range < 1e-6)
        // by including two distinct luminance values within the display range.
        let lo = black_y + 0.05 * (white_y - black_y);
        let hi = white_y - 0.05 * (white_y - black_y);
        let mut pixels = vec![[lo, lo, lo], [mid, mid, mid], [hi, hi, hi]];
        let original = pixels.clone();
        auto_compress_dynamic_range(&mut pixels, pal);
        for (i, (got, expected)) in pixels.iter().zip(original.iter()).enumerate() {
            let d: f64 = got.iter().zip(expected.iter()).map(|(a, b)| (a - b).abs()).sum();
            assert!(d < 1e-6, "pixel {i} should not be modified by auto-compress: moved by {d}");
        }
    }

    #[test]
    fn auto_compress_does_not_crush_dark_outliers() {
        // A high-key image (bright cluster overshooting white_y, triggering compression) with a
        // couple of near-black outliers far below the 2nd percentile. Pre-fix, the unclamped
        // remap sent `normalized` negative for those outliers, pulling their target luminance
        // below display black and crushing them darker than they started. Post-clamp, a
        // sub-percentile pixel saturates at the display black floor and is never darkened.
        let pal = spectra_palette();

        // 98 bright pixels spread 0.7..1.0 (so p2 lands ~0.7, well above the outliers),
        // plus 2 near-black outliers at y ≈ 0.0004.
        let mut pixels: Vec<[f64; 3]> = (0..98)
            .map(|i| {
                let v = 0.7 + 0.3 * (i as f64 / 97.0);
                [v, v, v]
            })
            .collect();
        let outlier = [0.0004_f64, 0.0004, 0.0004];
        pixels.push(outlier);
        pixels.push(outlier);

        let in_y = luminance(outlier);
        auto_compress_dynamic_range(&mut pixels, pal);
        let out_y = luminance(pixels[98]);
        assert!(
            out_y >= in_y - 1e-9,
            "sub-percentile dark outlier must not be crushed darker (in={in_y:.6}, out={out_y:.6})"
        );
    }

    #[test]
    fn auto_compress_preserves_dark_pixel_chroma() {
        // A near-black but distinctly blue outlier must keep blue dominant after auto-compress,
        // rather than being flattened to neutral display black.
        let pal = spectra_palette();
        // Blue channel 1e-5 → luminance ≈ 7e-7 < 1e-6, so this exercises the near-zero dark
        // branch, which pre-fix flattened the pixel to neutral display black.
        let mut pixels = vec![[1.0_f64, 1.0, 1.0]; 96];
        pixels.extend(std::iter::repeat_n([0.0_f64, 0.0, 1e-5], 4));

        auto_compress_dynamic_range(&mut pixels, pal);

        let [r, g, b] = pixels[96];
        assert!(
            b >= r && b >= g && b > 0.0,
            "blue should remain dominant after auto-compress of a dark blue pixel: [{r}, {g}, {b}]"
        );
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

    #[test]
    fn dark_pixel_chroma_preserved_after_compress() {
        // A near-black but distinctly blue pixel should not become gray after compression
        let pal = spectra_palette();
        // Very dark blue: luminance ≈ 0.07*0.00001 ≈ near zero, strong blue channel
        let mut pixels = vec![[0.0_f64, 0.0, 1e-5]];
        compress_dynamic_range(&mut pixels, pal, 1.0);
        // Blue channel should still dominate (be the largest) after compression
        let [r, g, b] = pixels[0];
        assert!(
            b >= r && b >= g,
            "blue channel should remain dominant after dark pixel compression: [{r}, {g}, {b}]"
        );
    }
}

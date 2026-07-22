//! Error diffusion and ordered dithering on raw RGB pixel buffers.

use crate::color_space::{srgb_channel_to_linear, srgb_fraction_to_linear};
use crate::color_space_lab::{PaletteLab, match_pixel_oklab, rgb_to_oklab, WAB};
use crate::palettes::Palette;
use rayon::prelude::*;

pub use crate::kernels::{
    Kernel, KernelOffset, ATKINSON, BURKES, FLOYD_STEINBERG, JARVIS_JUDICE_NINKE, SIERRA,
    SIERRA_LITE, STUCKI,
};

// ── Palette setup helper ─────────────────────────────────────────────────────

pub(crate) fn build_palette_lab(palette: &Palette) -> (Vec<[f64; 3]>, PaletteLab) {
    let palette_linear: Vec<[f64; 3]> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| {
            [
                srgb_channel_to_linear(r),
                srgb_channel_to_linear(g),
                srgb_channel_to_linear(b),
            ]
        })
        .collect();
    let palette_lab = PaletteLab::from_linear_rgb(&palette_linear);
    (palette_linear, palette_lab)
}

// ── Main function ─────────────────────────────────────────────────────────────

/// Error diffusion dither. Returns palette indices (len = width × height).
pub fn error_diffusion_dither(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    kernel: &Kernel,
    serpentine: bool,
) -> Vec<u8> {
    error_diffusion_dither_impl(pixels, width, height, palette, None, kernel, serpentine)
}

/// Error diffusion with exact canonical display-color pixels pinned.
///
/// Exact canonical pixels are already displayable, so they emit their firmware
/// palette index directly and absorb any accumulated error instead of diffusing
/// it into neighboring pixels.
pub fn error_diffusion_dither_with_canonical(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    canonical_palette: &Palette,
    kernel: &Kernel,
    serpentine: bool,
) -> Vec<u8> {
    error_diffusion_dither_impl(
        pixels,
        width,
        height,
        palette,
        Some(canonical_palette),
        kernel,
        serpentine,
    )
}

fn error_diffusion_dither_impl(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    canonical_palette: Option<&Palette>,
    kernel: &Kernel,
    serpentine: bool,
) -> Vec<u8> {
    let (_palette_linear, palette_lab) = build_palette_lab(palette);

    // Palette sRGB as f64 for error computation
    let palette_srgb_f: Vec<[f64; 3]> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| [r as f64, g as f64, b as f64])
        .collect();

    // Working buffer in sRGB float space [0, 255]; accumulates diffused error.
    let mut buf: Vec<f64> = pixels.iter().map(|&v| v as f64).collect();

    // LUT: u8 sRGB -> linear f64 (avoids powf per pixel in the inner loop)
    let lut: Vec<f64> = (0u8..=255).map(srgb_channel_to_linear).collect();

    let mut output = vec![0u8; width * height];

    for y in 0..height {
        // Serpentine: odd rows scan right-to-left
        let reverse = serpentine && y % 2 == 1;

        for xi in 0..width {
            let x = if reverse { width - 1 - xi } else { xi };
            let idx = (y * width + x) * 3;

            if let Some(canonical_palette) = canonical_palette
                && let Some(exact_idx) =
                    exact_palette_index(&pixels[idx..idx + 3], canonical_palette)
            {
                output[y * width + x] = exact_idx;
                continue;
            }

            let rs = buf[idx].clamp(0.0, 255.0);
            let gs = buf[idx + 1].clamp(0.0, 255.0);
            let bs = buf[idx + 2].clamp(0.0, 255.0);

            let r_lin = lut[rs.round() as usize];
            let g_lin = lut[gs.round() as usize];
            let b_lin = lut[bs.round() as usize];

            let pixel_lab = rgb_to_oklab(r_lin, g_lin, b_lin);
            let best_idx = match_pixel_oklab(pixel_lab, &palette_lab, WAB);

            output[y * width + x] = best_idx as u8;

            // Quantization error in sRGB space
            let err_r = rs - palette_srgb_f[best_idx][0];
            let err_g = gs - palette_srgb_f[best_idx][1];
            let err_b = bs - palette_srgb_f[best_idx][2];

            for offset in kernel.offsets {
                let effective_dx = if reverse { -offset.dx } else { offset.dx };

                let nx = x as i64 + effective_dx as i64;
                let ny = y as i64 + offset.dy as i64;

                if nx >= 0 && nx < width as i64 && ny >= 0 && ny < height as i64 {
                    let ni = (ny as usize * width + nx as usize) * 3;
                    buf[ni] += err_r * offset.weight;
                    buf[ni + 1] += err_g * offset.weight;
                    buf[ni + 2] += err_b * offset.weight;
                }
            }
        }
    }

    output
}

// ── Thin wrappers ─────────────────────────────────────────────────────────────

pub fn floyd_steinberg(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &FLOYD_STEINBERG, serpentine)
}
pub fn atkinson(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &ATKINSON, serpentine)
}
pub fn burkes(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &BURKES, serpentine)
}
pub fn stucki(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &STUCKI, serpentine)
}
pub fn sierra(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &SIERRA, serpentine)
}
pub fn sierra_lite(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &SIERRA_LITE, serpentine)
}
pub fn jarvis_judice_ninke(pixels: &[u8], w: usize, h: usize, palette: &Palette, serpentine: bool) -> Vec<u8> {
    error_diffusion_dither(pixels, w, h, palette, &JARVIS_JUDICE_NINKE, serpentine)
}

// ── Direct palette map (no dithering) ────────────────────────────────────────

/// Returns the palette index if the RGB pixel exactly matches a palette color, or None.
pub(crate) fn exact_palette_index(rgb: &[u8], palette: &Palette) -> Option<u8> {
    palette
        .colors
        .iter()
        .position(|&color| rgb == color)
        .and_then(|idx| u8::try_from(idx).ok())
}

pub fn try_exact_palette_map(pixels: &[u8], canonical_palette: &Palette) -> Option<Vec<u8>> {
    pixels
        .par_chunks(3)
        .map(|rgb| exact_palette_index(rgb, canonical_palette))
        .collect()
}

/// Nearest-color mapping with no dithering. Each pixel maps independently.
pub fn direct_map(pixels: &[u8], palette: &Palette, canonical_palette: &Palette) -> Vec<u8> {
    let (_, palette_lab) = build_palette_lab(palette);
    pixels
        .par_chunks(3)
        .map(|rgb| {
            if let Some(idx) = exact_palette_index(rgb, canonical_palette) {
                return idx;
            }

            let r = srgb_channel_to_linear(rgb[0]);
            let g = srgb_channel_to_linear(rgb[1]);
            let b = srgb_channel_to_linear(rgb[2]);
            let lab = rgb_to_oklab(r, g, b);
            match_pixel_oklab(lab, &palette_lab, WAB) as u8
        })
        .collect()
}

// ── Ordered (Bayer) dithering ─────────────────────────────────────────────────

// 4×4 Bayer matrix, zero-mean thresholds in (-0.5, 0.5). Indexed as [y % 4][x % 4].
// Values = (bayer_entry + 0.5) / 16.0 - 0.5, the standard zero-mean normalization —
// the +0.5 centering makes ordered dithering brightness-neutral (mean threshold = 0).
// Written as exact fractions to keep precision (all are odd multiples of 1/32).
const BAYER_4X4: [[f64; 4]; 4] = [
    [-0.46875,  0.03125, -0.34375,  0.15625],
    [ 0.28125, -0.21875,  0.40625, -0.09375],
    [-0.28125,  0.21875, -0.40625,  0.09375],
    [ 0.46875, -0.03125,  0.34375, -0.15625],
];

/// Threshold amplitude (in sRGB-fraction space) for ordered dither on this palette.
///
/// Ordered dithering trades quantization error for a fixed threshold pattern; the
/// threshold should be scaled to the palette's quantization step. For an evenly-spaced
/// grayscale ramp of `n` levels the step is `1/(n-1)`, so a full ±0.5 threshold is
/// `2·(n-1)×` too large (e.g. 14× for 16-level gray) and swamps fine detail with noise.
///
/// Sparse palettes (mono + color) keep the full ±0.5 spread: their transitions are
/// dominated by black↔white/ink decisions where the wide threshold is the tuned,
/// tested behavior (see the issue-#27 property test).
fn ordered_spread(palette: &Palette) -> f64 {
    let is_grayscale =
        palette.colors.len() >= 3 && palette.colors.iter().all(|&[r, g, b]| r == g && g == b);
    if is_grayscale {
        1.0 / (palette.colors.len() - 1) as f64
    } else {
        1.0
    }
}

/// Ordered (Bayer 4×4) dither. Pixels are independent — parallelized with rayon.
///
/// The Bayer threshold is added in sRGB-fraction space, not linear. Linear-space
/// thresholding produces ~3× the perceptual spread in shadows compared to highlights
/// (the sRGB gamma is convex, so a fixed linear ±0.5 step is huge near 0 and tiny near 1).
/// sRGB-space thresholding gives uniform perceptual dot density across the tonal range,
/// matching how error diffusion already accumulates error. See GitHub issue #27.
///
/// The threshold amplitude is scaled by `ordered_spread` to the palette's quantization
/// step so dense grayscale ramps are not swamped by full-range dither noise.
pub fn ordered_dither(pixels: &[u8], width: usize, palette: &Palette) -> Vec<u8> {
    ordered_dither_impl(pixels, width, palette, None)
}

pub fn ordered_dither_with_canonical(
    pixels: &[u8],
    width: usize,
    palette: &Palette,
    canonical_palette: &Palette,
) -> Vec<u8> {
    ordered_dither_impl(pixels, width, palette, Some(canonical_palette))
}

fn ordered_dither_impl(
    pixels: &[u8],
    width: usize,
    palette: &Palette,
    canonical_palette: Option<&Palette>,
) -> Vec<u8> {
    let (_palette_linear, palette_lab) = build_palette_lab(palette);
    let spread = ordered_spread(palette);

    // Per-threshold gamma LUT: lut[t][v] = srgb_fraction_to_linear((v/255 + threshold_t).clamp).
    // Only 16 thresholds × 256 byte values exist, so this removes all three per-pixel powf
    // calls. Built from the exact same inputs as the scalar path, so results are bit-identical.
    let lut: Vec<[f64; 256]> = (0..16)
        .map(|t| {
            let threshold = BAYER_4X4[t / 4][t % 4] * spread;
            let mut row = [0.0_f64; 256];
            for (v, slot) in row.iter_mut().enumerate() {
                *slot = srgb_fraction_to_linear((v as f64 / 255.0 + threshold).clamp(0.0, 1.0));
            }
            row
        })
        .collect();

    pixels
        .par_chunks(3)
        .enumerate()
        .map(|(i, rgb)| {
            if let Some(canonical_palette) = canonical_palette
                && let Some(idx) = exact_palette_index(rgb, canonical_palette)
            {
                return idx;
            }

            let x = i % width;
            let y = i / width;
            let t = (y % 4) * 4 + (x % 4);

            let lab = rgb_to_oklab(
                lut[t][rgb[0] as usize],
                lut[t][rgb[1] as usize],
                lut[t][rgb[2] as usize],
            );
            match_pixel_oklab(lab, &palette_lab, WAB) as u8
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::ColorScheme;

    fn solid_image(r: u8, g: u8, b: u8, w: usize, h: usize) -> Vec<u8> {
        vec![r, g, b].into_iter().cycle().take(w * h * 3).collect()
    }

    #[test]
    fn solid_white_maps_to_white() {
        let pixels = solid_image(255, 255, 255, 4, 4);
        let out = floyd_steinberg(&pixels, 4, 4, ColorScheme::Mono.palette(), true);
        // White palette index in Mono is 1
        assert!(out.iter().all(|&v| v == 1));
    }

    #[test]
    fn solid_black_maps_to_black() {
        let pixels = solid_image(0, 0, 0, 4, 4);
        let out = floyd_steinberg(&pixels, 4, 4, ColorScheme::Mono.palette(), true);
        assert!(out.iter().all(|&v| v == 0));
    }

    #[test]
    fn output_length_matches_pixels() {
        let pixels = solid_image(128, 64, 200, 10, 7);
        let out = floyd_steinberg(&pixels, 10, 7, ColorScheme::Bwr.palette(), true);
        assert_eq!(out.len(), 10 * 7);
    }

    #[test]
    fn all_algorithms_produce_correct_length() {
        let pixels = solid_image(128, 64, 200, 10, 7);
        let pal = ColorScheme::Bwr.palette();
        assert_eq!(burkes(&pixels, 10, 7, pal, false).len(), 70);
        assert_eq!(atkinson(&pixels, 10, 7, pal, false).len(), 70);
        assert_eq!(stucki(&pixels, 10, 7, pal, false).len(), 70);
        assert_eq!(sierra(&pixels, 10, 7, pal, false).len(), 70);
        assert_eq!(sierra_lite(&pixels, 10, 7, pal, false).len(), 70);
        assert_eq!(jarvis_judice_ninke(&pixels, 10, 7, pal, false).len(), 70);
        assert_eq!(ordered_dither(&pixels, 10, pal).len(), 70);
        assert_eq!(direct_map(&pixels, pal, pal).len(), 70);
    }

    #[test]
    fn direct_map_pure_red_maps_to_red_in_bwr() {
        // BWR palette: index 0=black, 1=white, 2=red — pure red should map to red ink
        let pixels = vec![255u8, 0, 0];
        let out = direct_map(&pixels, ColorScheme::Bwr.palette(), ColorScheme::Bwr.palette());
        assert_eq!(out[0], 2, "pure sRGB red should map to red ink (index 2) in BWR");
    }

    #[test]
    fn direct_map_pure_blue_maps_to_blue_in_bwgbry() {
        // BWGBRY palette order: 0=black, 1=white, 2=yellow, 3=red, 4=blue, 5=green
        let pixels = vec![0u8, 0, 255];
        let out = direct_map(&pixels, ColorScheme::Bwgbry.palette(), ColorScheme::Bwgbry.palette());
        assert_eq!(out[0], 4, "pure sRGB blue should map to blue ink (index 4) in BWGBRY");
    }

    #[test]
    fn serpentine_differs_from_raster_on_gradient() {
        // A gradient (non-uniform content) should produce different output
        // with serpentine=true vs serpentine=false
        let pixels: Vec<u8> = (0u8..=255)
            .flat_map(|v| [v, v / 2, 255 - v])
            .cycle()
            .take(16 * 16 * 3)
            .collect();
        let raster = floyd_steinberg(&pixels, 16, 16, ColorScheme::Mono.palette(), false);
        let serpentine = floyd_steinberg(&pixels, 16, 16, ColorScheme::Mono.palette(), true);
        assert_ne!(raster, serpentine, "serpentine and raster should differ on a gradient");
    }

    /// Property test for issue #27: ordered dither activity should be perceptually uniform
    /// across the tonal range, not skewed toward shadows.
    ///
    /// On a horizontal sRGB ramp (one column per luminance level), each Bayer-period band
    /// of columns contains the same `x % 4` thresholds and therefore the same *kinds* of
    /// dithering decisions. Under linear-space thresholding, the perceptual spread of the
    /// threshold is huge in shadows and tiny in highlights, so transitions cluster heavily
    /// in the dark bands. Under sRGB-space thresholding, transitions distribute roughly
    /// evenly across mid-tone bands.
    ///
    /// We measure transitions (adjacent columns with differing palette indices) per band
    /// in the mid-tone region and assert the max/min ratio stays modest. Empirically the
    /// linear-space implementation produces ratio ≈ 13.5; sRGB-space ≈ 2.2.
    ///
    /// The mono decision midpoint sits at OKLab L=0.5 ≈ sRGB 100, so the top two bands
    /// (sRGB ≳ 192) are already in the highlight roll-off where dither activity legitimately
    /// tapers; we exclude them along with the pure-black first band.
    #[test]
    fn ordered_dither_activity_is_perceptually_uniform() {
        const W: usize = 256;
        const H: usize = 16; // 4 full Bayer cells vertically
        const BANDS: usize = 8;
        const BAND_W: usize = W / BANDS;

        // Horizontal sRGB ramp 0..255.
        let mut pixels = Vec::with_capacity(W * H * 3);
        for _ in 0..H {
            for x in 0..W {
                let v = x as u8;
                pixels.extend_from_slice(&[v, v, v]);
            }
        }
        let out = ordered_dither(&pixels, W, ColorScheme::Mono.palette());

        // Count per-band horizontal transitions (adjacent columns with differing index).
        let mut transitions = [0usize; BANDS];
        for y in 0..H {
            for x in 0..W - 1 {
                if out[y * W + x] != out[y * W + x + 1] {
                    transitions[(x / BAND_W).min(BANDS - 1)] += 1;
                }
            }
        }

        // Mid-tone bands: skip the pure-black first band and the top two near-white bands
        // (past the mono midpoint ≈ sRGB 100) where the threshold clamps and dither
        // activity legitimately falls off.
        let mid = &transitions[1..BANDS - 2];
        let max = *mid.iter().max().unwrap();
        let min = *mid.iter().min().unwrap();
        assert!(min > 0, "every mid-tone band should have at least one transition: {transitions:?}");
        let ratio = max as f64 / min as f64;
        assert!(
            ratio < 5.0,
            "ordered-dither activity is perceptually skewed (max/min = {ratio:.2}); \
             expected sRGB-space ordered dither to spread roughly uniformly across \
             mid-tones. Per-band transitions: {transitions:?}"
        );
    }

    #[test]
    fn bayer_matrix_is_zero_mean() {
        // A biased Bayer matrix systematically darkens (or brightens) every ordered-
        // dithered image. The standard (v + 0.5)/16 - 0.5 normalization sums to zero.
        let sum: f64 = BAYER_4X4.iter().flatten().sum();
        assert!(sum.abs() < 1e-12, "Bayer thresholds must sum to zero, got {sum}");
    }

    #[test]
    fn ordered_dither_is_brightness_neutral_at_midpoint() {
        // Find the mono decision midpoint: the smallest gray value whose threshold-free
        // OKLab match flips from black (0) to white (1).
        let pal = ColorScheme::Mono.palette();
        let (_, pal_lab) = build_palette_lab(pal);
        let midpoint = (0u16..=255)
            .find(|&v| {
                let lin = srgb_channel_to_linear(v as u8);
                let lab = rgb_to_oklab(lin, lin, lin);
                match_pixel_oklab(lab, &pal_lab, WAB) == 1
            })
            .expect("mono palette must have a black→white transition") as u8;

        // Dither a solid midpoint-gray field. With a zero-mean threshold the white
        // fraction should sit near 0.5; the old -1/32 bias produced ≈ 7/16.
        const N: usize = 32;
        let pixels = solid_image(midpoint, midpoint, midpoint, N, N);
        let out = ordered_dither(&pixels, N, pal);
        let white = out.iter().filter(|&&v| v == 1).count() as f64 / out.len() as f64;
        assert!(
            (white - 0.5).abs() <= 1.0 / 16.0,
            "ordered dither at the mono midpoint should be ~50% white, got {white}"
        );
    }

    #[test]
    fn ordered_dither_grayscale16_tracks_ramp_and_uses_many_levels() {
        // On a horizontal sRGB ramp the full ±0.5 threshold swamps the 17/255 GS16 step
        // (14× too large). After scaling by ordered_spread, block-averaged output should
        // track the input ramp and exercise most of the 16 levels.
        const W: usize = 256;
        const H: usize = 4;
        let pal = ColorScheme::Grayscale16.palette();

        let mut pixels = Vec::with_capacity(W * H * 3);
        for _ in 0..H {
            for x in 0..W {
                let v = x as u8;
                pixels.extend_from_slice(&[v, v, v]);
            }
        }
        let out = ordered_dither(&pixels, W, pal);

        // (a) 4-wide block average of palette level tracks the input level.
        let levels: Vec<f64> = pal.colors.iter().map(|&[r, _, _]| r as f64).collect();
        let mut max_err = 0.0_f64;
        for bx in 0..W / 4 {
            let mut sum = 0.0;
            for y in 0..H {
                for dx in 0..4 {
                    let x = bx * 4 + dx;
                    sum += levels[out[y * W + x] as usize];
                }
            }
            let avg = sum / (H * 4) as f64;
            let input = (bx * 4 + 1) as f64; // center-ish input value
            max_err = max_err.max((avg - input).abs());
        }
        assert!(max_err < 24.0, "block-averaged output should track the ramp, max_err={max_err}");

        // (b) most of the 16 levels are used.
        let mut seen = [false; 16];
        for &idx in &out {
            seen[idx as usize] = true;
        }
        let used = seen.iter().filter(|&&s| s).count();
        assert!(used >= 12, "GS16 ordered dither should use ≥12 levels, used {used}");
    }

    #[test]
    fn serpentine_first_row_matches_raster() {
        // Row 0 is always scanned left-to-right regardless of serpentine flag
        let pixels: Vec<u8> = (0u8..=255)
            .flat_map(|v| [v, 128u8.wrapping_add(v), 64u8])
            .cycle()
            .take(8 * 4 * 3)
            .collect();
        let raster = floyd_steinberg(&pixels, 8, 4, ColorScheme::Mono.palette(), false);
        let serpentine = floyd_steinberg(&pixels, 8, 4, ColorScheme::Mono.palette(), true);
        assert_eq!(
            &raster[0..8],
            &serpentine[0..8],
            "first row must be identical (both scan left-to-right)"
        );
    }
}

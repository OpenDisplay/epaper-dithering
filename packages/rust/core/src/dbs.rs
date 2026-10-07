//! Colour Direct Binary Search (DBS): iterative refinement of an already-dithered image.
//!
//! Minimises perceived error `E = Σ_ch Σ_m ((p_ch ∗ e_ch)(m))²`, where `e = halftone − target`
//! in YyCxCz and `p_ch` is a per-channel eye-model blur: narrow for luminance, wider for
//! chrominance. YyCxCz is a linear transform of XYZ (linearised CIELAB), so blurring the error
//! averages light the way the eye does — unlike OKLab, which `color_space_lab` uses for
//! per-pixel matching.
//!
//! Each pass visits every pixel in raster order and tries changing it to every other ink and
//! swapping it with each differing 8-neighbour, applying the move with the largest decrease
//! in `E`. Trial cost is O(1) via the cached correlation `c_pe = c_pp ⋆ e`, where
//! `c_pp = p ⋆ p` is the blur's autocorrelation; an accepted move updates `c_pe` over the
//! footprint of `c_pp` only.
//!
//! Eye-model blurs are stored as full 2-D tables and `c_pp` is derived from them generically,
//! so a non-separable model (e.g. the exponential CSF's spatial kernel) can be swapped in.
//!
//! References: Analoui & Allebach (1992); Lieberman & Allebach (1997); Agar & Allebach (2005);
//! Flohr, Kolpatzik et al. (1993, YyCxCz); Kolpatzik & Bouman (1992, CSF constants);
//! Mullen (1985). Ported from OpenDithering's `dbs.ts` (MIT, © Guy Sie).

use rayon::prelude::*;

use crate::color_space::srgb_channel_to_linear;
use crate::palettes::Palette;

// Kolpatzik & Bouman (1992) exponential CSF decay constants, W(f) ∝ exp(−α·f), f in cycles/degree.
// Luminance: Näsänen's α(L) = 1 / (0.525·ln L + 3.91) at L = 11 cd/m² ≈ 0.193.
// Chrominance: fitted to Mullen (1985), α = 0.419 for both opponent channels.
const ALPHA_LUM: f64 = 0.193;
const ALPHA_CHROMA: f64 = 0.419;

/// Linear sRGB → CIE XYZ (D65), IEC 61966-2-1.
const SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.412_456_4, 0.357_576_1, 0.180_437_5],
    [0.212_672_9, 0.715_152_2, 0.072_175_0],
    [0.019_333_9, 0.119_192_0, 0.950_304_1],
];

/// sRGB bytes → YyCxCz: `Yy = 116·Y/Yn`, `Cx = 500·(X/Xn − Y/Yn)`, `Cz = 200·(Y/Yn − Z/Zn)`.
/// The white point is sRGB white, so (255, 255, 255) maps to zero chroma. Measured palettes
/// are normalised to paper white, which therefore lands at the same point.
fn srgb_to_yycxcz(rgb: [u8; 3]) -> [f64; 3] {
    let lin = rgb.map(srgb_channel_to_linear);
    let [x, y, z] = SRGB_TO_XYZ.map(|row| row[0] * lin[0] + row[1] * lin[1] + row[2] * lin[2]);
    let [xn, yn, zn] = SRGB_TO_XYZ.map(|row| row[0] + row[1] + row[2]);
    [116.0 * y / yn, 500.0 * (x / xn - y / yn), 200.0 * (y / yn - z / zn)]
}

/// Square, centred 2-D table: `data[(dy + r)·size + (dx + r)]` for `|dx|, |dy| ≤ r`.
#[derive(Clone)]
struct Kernel2D {
    radius: usize,
    size: usize,
    data: Vec<f64>,
}

impl Kernel2D {
    #[inline]
    fn at(&self, dx: isize, dy: isize) -> f64 {
        let r = self.radius as isize;
        self.data[((dy + r) * self.size as isize + dx + r) as usize]
    }
}

/// Normalised (unit-sum) sampled Gaussian, truncated at `ceil(3σ)`.
fn gaussian_blur(sigma: f64) -> Kernel2D {
    let r = (3.0 * sigma).ceil().max(1.0) as usize;
    let size = 2 * r + 1;
    let g: Vec<f64> = (0..size)
        .map(|i| {
            let d = i as f64 - r as f64;
            (-(d * d) / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let sum: f64 = g.iter().sum();
    let data = (0..size * size).map(|i| g[i / size] * g[i % size] / (sum * sum)).collect();
    Kernel2D { radius: r, size, data }
}

/// `c(d) = Σ_u p(u)·p(u + d)`, with twice the radius of `p`.
fn autocorrelate(p: &Kernel2D) -> Kernel2D {
    let pr = p.radius as isize;
    let radius = 2 * p.radius;
    let size = 2 * radius + 1;
    let mut data = vec![0.0; size * size];
    for dy in -(radius as isize)..=radius as isize {
        for dx in -(radius as isize)..=radius as isize {
            let mut s = 0.0;
            for uy in (-pr).max(-pr - dy)..=pr.min(pr - dy) {
                for ux in (-pr).max(-pr - dx)..=pr.min(pr - dx) {
                    s += p.at(ux, uy) * p.at(ux + dx, uy + dy);
                }
            }
            data[(dy + radius as isize) as usize * size + (dx + radius as isize) as usize] = s;
        }
    }
    Kernel2D { radius, size, data }
}

/// Eye-model blur width in pixels for a CSF decay constant at the given viewing geometry.
fn eye_sigma_px(alpha: f64, params: &DbsParams) -> f64 {
    let pixel_deg = (2.54 / params.ppi / params.viewing_distance_cm).to_degrees();
    alpha / (2.0 * std::f64::consts::PI) / pixel_deg
}

/// Per-channel eye-model blur: index 0 = luminance (Yy), 1–2 = chrominance (Cx, Cz).
fn eye_blurs(params: &DbsParams) -> [Kernel2D; 3] {
    assert!(
        params.ppi.is_finite() && params.ppi > 0.0,
        "DBS ppi must be positive, got {}",
        params.ppi
    );
    assert!(
        params.viewing_distance_cm.is_finite() && params.viewing_distance_cm > 0.0,
        "DBS viewing distance must be positive, got {}",
        params.viewing_distance_cm
    );
    let lum = gaussian_blur(eye_sigma_px(ALPHA_LUM, params));
    let chroma = gaussian_blur(eye_sigma_px(ALPHA_CHROMA, params));
    [lum, chroma.clone(), chroma]
}

/// Palette inks and target pixels in YyCxCz.
fn to_yycxcz(pixels: &[u8], width: usize, height: usize, palette: &Palette) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    assert_eq!(pixels.len(), width * height * 3, "pixel buffer size mismatch");
    let inks = palette.colors.iter().map(|&c| srgb_to_yycxcz(c)).collect();
    let target = pixels.par_chunks_exact(3).map(|c| srgb_to_yycxcz([c[0], c[1], c[2]])).collect();
    (inks, target)
}

/// `out(m) = Σ_d c(d)·e(m + d)` over the image (e is zero outside it).
fn correlate(e: &[f64], width: usize, height: usize, c: &Kernel2D) -> Vec<f64> {
    let r = c.radius as isize;
    let (w, h) = (width as isize, height as isize);
    let mut out = vec![0.0; e.len()];
    out.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
        let y = y as isize;
        for (x, o) in row.iter_mut().enumerate() {
            let x = x as isize;
            let mut s = 0.0;
            for dy in (-r).max(-y)..=r.min(h - 1 - y) {
                let base = ((y + dy) * w + x) as usize;
                for dx in (-r).max(-x)..=r.min(w - 1 - x) {
                    s += c.at(dx, dy) * e[(base as isize + dx) as usize];
                }
            }
            *o = s;
        }
    });
    out
}

/// 8-neighbourhood offsets `(dx, dy)` tried for swaps.
const NEIGHBOURS: [(isize, isize); 8] =
    [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// `(x + dx, y + dy)` if it lies inside a `width × height` image.
fn neighbour(x: usize, y: usize, dx: isize, dy: isize, width: usize, height: usize) -> Option<(usize, usize)> {
    let nx = x.checked_add_signed(dx).filter(|&v| v < width)?;
    let ny = y.checked_add_signed(dy).filter(|&v| v < height)?;
    Some((nx, ny))
}

/// Parameters for DBS refinement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DbsParams {
    /// Distance between viewer and panel, in centimetres.
    pub viewing_distance_cm: f64,
    /// Panel pixel density, in pixels per inch.
    pub ppi: f64,
    /// Upper bound on full-image passes; refinement stops earlier once a pass changes nothing.
    pub max_passes: u32,
}

impl Default for DbsParams {
    /// 40 cm from a ~127 ppi panel (7.3" 800×480), at most 10 passes.
    fn default() -> Self {
        Self { viewing_distance_cm: 40.0, ppi: 127.0, max_passes: 10 }
    }
}

/// Diagnostics from a `dbs_refine` run.
#[derive(Debug, Clone, PartialEq)]
pub struct DbsStats {
    /// Passes actually run (≤ `max_passes`).
    pub passes: u32,
    /// Number of accepted changes in each pass.
    pub accepted_per_pass: Vec<usize>,
    /// Perceived error of the starting halftone.
    pub initial_error: f64,
    /// Perceived error of the refined halftone.
    pub final_error: f64,
}

/// Refine palette `indices` in place to minimise perceived error against `pixels` (sRGB,
/// `width × height × 3`), as seen through `palette` (pass the measured palette).
///
/// `pinned` pixels are never changed or swapped, and their target is taken to be their own
/// ink, so they neither carry error nor pull neighbours to compensate for it.
///
/// # Panics
/// Panics if buffer lengths disagree with `width × height`, or on a non-positive `ppi` or
/// viewing distance.
pub fn dbs_refine(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    indices: &mut [u8],
    pinned: Option<&[bool]>,
    params: &DbsParams,
) -> DbsStats {
    let n_px = width * height;
    assert_eq!(indices.len(), n_px, "index buffer size mismatch");
    if let Some(p) = pinned {
        assert_eq!(p.len(), n_px, "pinned mask size mismatch");
    }
    let is_pinned = |i: usize| pinned.is_some_and(|p| p[i]);

    let (inks, mut target) = to_yycxcz(pixels, width, height, palette);
    for (i, t) in target.iter_mut().enumerate() {
        if is_pinned(i) {
            *t = inks[indices[i] as usize];
        }
    }

    let cpp = eye_blurs(params).map(|p| autocorrelate(&p));
    let cpp0 = [cpp[0].at(0, 0), cpp[1].at(0, 0), cpp[2].at(0, 0)];

    let mut cpe: [Vec<f64>; 3] = std::array::from_fn(|ch| {
        let e: Vec<f64> = (0..n_px).map(|i| inks[indices[i] as usize][ch] - target[i][ch]).collect();
        correlate(&e, width, height, &cpp[ch])
    });
    let mut err: f64 = (0..3)
        .map(|ch| {
            (0..n_px)
                .map(|i| (inks[indices[i] as usize][ch] - target[i][ch]) * cpe[ch][i])
                .sum::<f64>()
        })
        .sum();
    let initial_error = err;

    // c_pe += a·c_pp(· − (mx, my)), clipped to the image.
    let add_kernel = |c: &mut [f64], k: &Kernel2D, mx: usize, my: usize, a: f64| {
        let r = k.radius;
        for y in my.saturating_sub(r)..=(my + r).min(height - 1) {
            let row = y * width;
            let dy = y as isize - my as isize;
            for x in mx.saturating_sub(r)..=(mx + r).min(width - 1) {
                c[row + x] += a * k.at(x as isize - mx as isize, dy);
            }
        }
    };

    const EPS: f64 = 1e-9;
    let n_inks = inks.len();
    let mut accepted_per_pass = Vec::new();

    for _ in 0..params.max_passes {
        let mut accepted = 0;
        for y in 0..height {
            for x in 0..width {
                let m = y * width + x;
                if is_pinned(m) {
                    continue;
                }
                let k = indices[m] as usize;
                let c = [cpe[0][m], cpe[1][m], cpe[2][m]];

                enum Move { Toggle(usize), Swap { n: usize, nx: usize, ny: usize } }
                let mut best_de = -EPS;
                let mut best = None;

                for j in (0..n_inks).filter(|&j| j != k) {
                    let de: f64 = (0..3)
                        .map(|ch| {
                            let a = inks[j][ch] - inks[k][ch];
                            cpp0[ch] * a * a + 2.0 * a * c[ch]
                        })
                        .sum();
                    if de < best_de {
                        best_de = de;
                        best = Some(Move::Toggle(j));
                    }
                }

                for (dx, dy) in NEIGHBOURS {
                    let Some((nx, ny)) = neighbour(x, y, dx, dy, width, height) else { continue };
                    let n = ny * width + nx;
                    let kn = indices[n] as usize;
                    if kn == k || is_pinned(n) {
                        continue;
                    }
                    // m takes kn (Δ = a), n takes k (Δ = −a).
                    let de: f64 = (0..3)
                        .map(|ch| {
                            let a = inks[kn][ch] - inks[k][ch];
                            2.0 * a * a * (cpp0[ch] - cpp[ch].at(dx, dy)) + 2.0 * a * (c[ch] - cpe[ch][n])
                        })
                        .sum();
                    if de < best_de {
                        best_de = de;
                        best = Some(Move::Swap { n, nx, ny });
                    }
                }

                let Some(mv) = best else { continue };
                accepted += 1;
                err += best_de;
                match mv {
                    Move::Toggle(j) => {
                        for ch in 0..3 {
                            add_kernel(&mut cpe[ch], &cpp[ch], x, y, inks[j][ch] - inks[k][ch]);
                        }
                        indices[m] = j as u8;
                    }
                    Move::Swap { n, nx, ny } => {
                        let kn = indices[n] as usize;
                        for ch in 0..3 {
                            let a = inks[kn][ch] - inks[k][ch];
                            add_kernel(&mut cpe[ch], &cpp[ch], x, y, a);
                            add_kernel(&mut cpe[ch], &cpp[ch], nx, ny, -a);
                        }
                        indices.swap(m, n);
                    }
                }
            }
        }
        accepted_per_pass.push(accepted);
        if accepted == 0 {
            break;
        }
    }

    DbsStats {
        passes: accepted_per_pass.len() as u32,
        accepted_per_pass,
        initial_error,
        final_error: err,
    }
}

/// Perceived error `Σ_ch Σ_m ((p_ch ∗ e_ch)(m))²` of `indices` against `pixels`, computed
/// directly by blurring the error over the whole plane (no correlation shortcut). This is the
/// quantity `dbs_refine` minimises; useful for comparing halftones under the eye model.
pub fn perceived_error(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    indices: &[u8],
    params: &DbsParams,
) -> f64 {
    assert_eq!(indices.len(), width * height, "index buffer size mismatch");
    let (inks, target) = to_yycxcz(pixels, width, height, palette);
    let blurs = eye_blurs(params);
    let (w, h) = (width as isize, height as isize);

    (0..3)
        .map(|ch| {
            let p = &blurs[ch];
            let r = p.radius as isize;
            let e: Vec<f64> = (0..indices.len()).map(|i| inks[indices[i] as usize][ch] - target[i][ch]).collect();
            // The blurred error is non-zero up to r pixels outside the image.
            (-r..h + r)
                .into_par_iter()
                .map(|oy| {
                    let mut s = 0.0;
                    for ox in -r..w + r {
                        let mut v = 0.0;
                        for y in (oy - r).max(0)..=(oy + r).min(h - 1) {
                            for x in (ox - r).max(0)..=(ox + r).min(w - 1) {
                                v += p.at(ox - x, oy - y) * e[(y * w + x) as usize];
                            }
                        }
                        s += v * v;
                    }
                    s
                })
                .sum::<f64>()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::{error_diffusion_dither, BURKES};
    use crate::measured_palettes::SPECTRA_7_3_6COLOR;
    use crate::palettes::ColorScheme;
    use approx::assert_relative_eq;

    /// Smooth colour gradient: enough structure that Burkes leaves room for improvement.
    fn gradient(w: usize, h: usize) -> Vec<u8> {
        let mut px = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                px.push((x * 255 / (w - 1)) as u8);
                px.push((y * 255 / (h - 1)) as u8);
                px.push((255 - x * 255 / (w - 1)) as u8);
            }
        }
        px
    }

    fn burkes(px: &[u8], w: usize, h: usize, palette: &Palette) -> Vec<u8> {
        error_diffusion_dither(px, w, h, palette, &BURKES, true)
    }

    #[test]
    fn perceived_error_is_zero_when_halftone_equals_target() {
        let palette = &SPECTRA_7_3_6COLOR;
        let indices: Vec<u8> = (0..24).map(|i| (i % palette.colors.len()) as u8).collect();
        let px: Vec<u8> = indices.iter().flat_map(|&i| palette.colors[i as usize]).collect();
        let e = perceived_error(&px, 6, 4, palette, &indices, &DbsParams::default());
        assert!(e.abs() < 1e-9, "expected zero error, got {e}");
    }

    #[test]
    fn perceived_error_is_positive_for_a_mismatched_halftone() {
        let palette = ColorScheme::Mono.palette();
        let px = vec![128u8; 8 * 8 * 3];
        let indices = vec![0u8; 64]; // all black for a mid-grey target
        assert!(perceived_error(&px, 8, 8, palette, &indices, &DbsParams::default()) > 0.0);
    }

    #[test]
    fn refine_reduces_perceived_error() {
        let (w, h) = (32, 24);
        let px = gradient(w, h);
        let palette = &SPECTRA_7_3_6COLOR;
        let params = DbsParams::default();
        let mut idx = burkes(&px, w, h, palette);
        let before = perceived_error(&px, w, h, palette, &idx, &params);

        dbs_refine(&px, w, h, palette, &mut idx, None, &params);

        let after = perceived_error(&px, w, h, palette, &idx, &params);
        assert!(after < before * 0.95, "DBS should cut error noticeably: {before} -> {after}");
    }

    /// The incremental bookkeeping (ΔE formulas, correlation updates) must agree with a
    /// from-scratch blur-and-square of the final halftone. Catches wrong toggle/swap deltas.
    #[test]
    fn reported_errors_match_direct_computation() {
        let (w, h) = (32, 24);
        let px = gradient(w, h);
        let palette = &SPECTRA_7_3_6COLOR;
        let params = DbsParams::default();
        let mut idx = burkes(&px, w, h, palette);
        let before = perceived_error(&px, w, h, palette, &idx, &params);

        let stats = dbs_refine(&px, w, h, palette, &mut idx, None, &params);

        let after = perceived_error(&px, w, h, palette, &idx, &params);
        assert_relative_eq!(stats.initial_error, before, max_relative = 1e-9);
        assert_relative_eq!(stats.final_error, after, max_relative = 1e-9);
    }

    /// After convergence no single toggle or 8-neighbour swap may lower the error, judged by
    /// brute-force recomputation — the defining property of a DBS local optimum.
    #[test]
    fn converged_result_is_a_local_minimum() {
        let (w, h) = (10, 8);
        let px = gradient(w, h);
        let palette = &SPECTRA_7_3_6COLOR;
        let params = DbsParams { max_passes: 200, ..Default::default() };
        let mut idx = burkes(&px, w, h, palette);

        let stats = dbs_refine(&px, w, h, palette, &mut idx, None, &params);
        assert_eq!(stats.accepted_per_pass.last(), Some(&0), "should converge: {stats:?}");

        let base = perceived_error(&px, w, h, palette, &idx, &params);
        let tol = base * 1e-9 + 1e-9;
        for m in 0..w * h {
            for k in 0..palette.colors.len() as u8 {
                let mut t = idx.clone();
                t[m] = k;
                let e = perceived_error(&px, w, h, palette, &t, &params);
                assert!(e >= base - tol, "toggle at {m} to {k} lowers error {base} -> {e}");
            }
            for (dx, dy) in NEIGHBOURS {
                let Some((nx, ny)) = neighbour(m % w, m / w, dx, dy, w, h) else { continue };
                let mut t = idx.clone();
                t.swap(m, ny * w + nx);
                let e = perceived_error(&px, w, h, palette, &t, &params);
                assert!(e >= base - tol, "swap {m} with ({nx},{ny}) lowers error {base} -> {e}");
            }
        }
    }

    #[test]
    fn pinned_pixels_never_change() {
        let (w, h) = (32, 24);
        let px = gradient(w, h);
        let palette = &SPECTRA_7_3_6COLOR;
        let mut idx = burkes(&px, w, h, palette);
        let pinned: Vec<bool> = (0..w * h).map(|i| i % 3 == 0).collect();
        let original = idx.clone();

        let stats = dbs_refine(&px, w, h, palette, &mut idx, Some(&pinned), &DbsParams::default());

        assert!(stats.accepted_per_pass.iter().sum::<usize>() > 0, "unpinned pixels should still move");
        for i in (0..w * h).filter(|&i| pinned[i]) {
            assert_eq!(idx[i], original[i], "pinned pixel {i} changed");
        }
    }

    /// Pins the colour-science decision that DBS measures error in linear light: a flat
    /// sRGB 188 (linear ≈ 0.50) on black/white must come out ≈ 50% white ink, not the
    /// ≈ 74% an sRGB-space average would give.
    #[test]
    fn flat_grey_reaches_linear_light_coverage() {
        let (w, h) = (48, 48);
        let px = vec![188u8; w * h * 3];
        let palette = ColorScheme::Mono.palette();
        let white = palette.colors.iter().position(|&c| c == [255, 255, 255]).unwrap() as u8;
        let mut idx = burkes(&px, w, h, palette);

        dbs_refine(&px, w, h, palette, &mut idx, None, &DbsParams { max_passes: 50, ..Default::default() });

        let frac = idx.iter().filter(|&&i| i == white).count() as f64 / (w * h) as f64;
        assert!((0.46..=0.54).contains(&frac), "white coverage {frac:.3}, expected ≈ 0.50");
    }

    #[test]
    fn zero_passes_leaves_indices_untouched() {
        let (w, h) = (16, 12);
        let px = gradient(w, h);
        let palette = &SPECTRA_7_3_6COLOR;
        let mut idx = burkes(&px, w, h, palette);
        let original = idx.clone();
        let stats = dbs_refine(&px, w, h, palette, &mut idx, None, &DbsParams { max_passes: 0, ..Default::default() });
        assert_eq!(idx, original);
        assert_eq!(stats.passes, 0);
    }
}

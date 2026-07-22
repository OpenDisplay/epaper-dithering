//! Dizzy dithering: error diffusion with pseudo-random traversal.
//!
//! Algorithm devised and described by **Liam Appelbe** in "Dizzy Dithering":
//! <https://liamappelbe.medium.com/dizzy-dithering-2ae76dbceba1>
//!
//! Implemented here from that written description — the article contains no code
//! listing, so this is an independent implementation, not a port. The two design
//! choices taken directly from the article are the stateless multiply-and-xor
//! permutation used for the traversal, and the 10:1 orthogonal-to-diagonal ratio
//! when spreading error over the unvisited neighbours.

use crate::algorithms::{build_palette_lab, exact_palette_index};
use crate::color_space::srgb_channel_to_linear;
use crate::color_space_lab::{match_pixel_oklab, rgb_to_oklab, WAB};
use crate::palettes::Palette;

// ── Traversal ─────────────────────────────────────────────────────────────────
//
// A stateless bijective permutation of `0..2^bits`, so the walk needs no shuffled
// index array. Multiplication by an odd number is invertible modulo 2^k (odd
// numbers are units in that ring) and XOR by a constant is self-inverse, so five
// rounds of (multiply, mask, xor) compose to a bijection. Indices >= n are skipped.
//
// FROZEN: changing either table changes every image this mode has ever produced.
const ODD: [u64; 5] = [0x2545_F491, 0x9E37_79B1, 0x85EB_CA6B, 0xC2B2_AE35, 0x27D4_EB2F];
const XOR: [u64; 5] = [0x1656_67B1, 0xD3A2_646C, 0xFD70_46C5, 0xB55A_4F09, 0x1B87_3593];

/// Smallest `bits` such that `2^bits >= n`. `bits_for(1) == 0`.
fn bits_for(n: usize) -> u32 {
    debug_assert!(n > 0, "bits_for requires a non-empty image");
    usize::BITS - (n - 1).leading_zeros()
}

fn permute(i: u64, mask: u64) -> u64 {
    let mut p = i;
    for r in 0..ODD.len() {
        p = p.wrapping_mul(ODD[r]) & mask;
        p ^= XOR[r] & mask;
    }
    p
}

/// Yields every index in `0..n` exactly once, in pseudo-random order.
pub(crate) fn dizzy_order(n: usize) -> impl Iterator<Item = usize> {
    let bits = bits_for(n);
    let mask = if bits >= u64::BITS { u64::MAX } else { (1u64 << bits) - 1 };
    (0..=mask).filter_map(move |i| {
        let p = permute(i, mask) as usize;
        (p < n).then_some(p)
    })
}

/// 8-neighbourhood with the source article's 10:1 orthogonal:diagonal weighting.
const NEIGHBORS: [(i64, i64, f64); 8] = [
    (0, -1, 1.0), (-1, 0, 1.0), (1, 0, 1.0), (0, 1, 1.0),
    (-1, -1, 0.1), (1, -1, 0.1), (-1, 1, 0.1), (1, 1, 0.1),
];

/// Dither with pseudo-random traversal, diffusing error only to unquantized neighbours.
pub fn dizzy_dither(pixels: &[u8], width: usize, height: usize, palette: &Palette) -> Vec<u8> {
    dizzy_dither_impl(pixels, width, height, palette, None)
}

/// As [`dizzy_dither`], but pixels exactly matching a `canonical` ink pass through
/// unchanged and are excluded from error distribution.
pub fn dizzy_dither_with_canonical(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    canonical: &Palette,
) -> Vec<u8> {
    dizzy_dither_impl(pixels, width, height, palette, Some(canonical))
}

fn dizzy_dither_impl(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    canonical_palette: Option<&Palette>,
) -> Vec<u8> {
    let (_palette_linear, palette_lab) = build_palette_lab(palette);
    let palette_srgb_f: Vec<[f64; 3]> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| [r as f64, g as f64, b as f64])
        .collect();

    // Working buffer in sRGB float space [0, 255]; accumulates diffused error.
    let mut buf: Vec<f64> = pixels.iter().map(|&v| v as f64).collect();
    // LUT: u8 sRGB -> linear f64 (avoids powf per pixel in the inner loop)
    let lut: Vec<f64> = (0u8..=255).map(srgb_channel_to_linear).collect();

    let n = width * height;
    let mut output = vec![0u8; n];
    let mut processed = vec![false; n];

    // Pre-pass: pin exact canonical pixels and mark them processed BEFORE the walk,
    // so no neighbour spends error on a pixel that will ignore it. This differs
    // deliberately from the raster implementation, which lets pinned pixels
    // accumulate error that is then discarded.
    if let Some(canonical) = canonical_palette {
        for i in 0..n {
            if let Some(exact) = exact_palette_index(&pixels[i * 3..i * 3 + 3], canonical) {
                output[i] = exact;
                processed[i] = true;
            }
        }
    }

    for i in dizzy_order(n) {
        if processed[i] {
            continue;
        }
        let idx = i * 3;
        let rs = buf[idx].clamp(0.0, 255.0);
        let gs = buf[idx + 1].clamp(0.0, 255.0);
        let bs = buf[idx + 2].clamp(0.0, 255.0);

        let pixel_lab = rgb_to_oklab(
            lut[rs.round() as usize],
            lut[gs.round() as usize],
            lut[bs.round() as usize],
        );
        let best = match_pixel_oklab(pixel_lab, &palette_lab, WAB);

        output[i] = best as u8;
        processed[i] = true;

        let err = [
            rs - palette_srgb_f[best][0],
            gs - palette_srgb_f[best][1],
            bs - palette_srgb_f[best][2],
        ];

        let x = (i % width) as i64;
        let y = (i / width) as i64;

        // Pass 1: total weight of the still-unquantized neighbours.
        let mut denom = 0.0;
        for (dx, dy, w) in NEIGHBORS {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && nx < width as i64 && ny >= 0 && ny < height as i64 {
                let ni = ny as usize * width + nx as usize;
                if !processed[ni] {
                    denom += w;
                }
            }
        }

        // Every neighbour is already quantized, so this pixel's error has nowhere to
        // go and is dropped. That is inherent to the algorithm: do NOT "fix" it by
        // widening the neighbourhood or diffusing back into processed pixels --
        // either changes the output and defeats the point of the traversal order.
        if denom == 0.0 {
            continue;
        }

        // Pass 2: distribute, normalized so the full error is conserved.
        for (dx, dy, w) in NEIGHBORS {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && nx < width as i64 && ny >= 0 && ny < height as i64 {
                let ni = ny as usize * width + nx as usize;
                if !processed[ni] {
                    let share = w / denom;
                    buf[ni * 3] += err[0] * share;
                    buf[ni * 3 + 1] += err[1] * share;
                    buf[ni * 3 + 2] += err[2] * share;
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::ColorScheme;

    fn gray_image(value: u8, count: usize) -> Vec<u8> {
        std::iter::repeat_n([value, value, value], count).flatten().collect()
    }

    #[test]
    fn output_is_deterministic() {
        let img = gray_image(128, 64);
        let p = ColorScheme::Bwr.palette();
        let a = dizzy_dither(&img, 8, 8, p);
        let b = dizzy_dither(&img, 8, 8, p);
        assert_eq!(a, b, "dizzy must be deterministic for identical input");
    }

    #[test]
    fn output_length_and_indices_are_in_range() {
        let img = gray_image(90, 96);
        let p = ColorScheme::Bwgbry.palette();
        let out = dizzy_dither(&img, 12, 8, p);
        assert_eq!(out.len(), 96);
        assert!(out.iter().all(|&i| (i as usize) < p.colors.len()));
    }

    #[test]
    fn flat_midgray_uses_more_than_one_ink() {
        // A flat field must dither, not collapse to a single nearest colour.
        let img = gray_image(128, 1024);
        let out = dizzy_dither(&img, 32, 32, ColorScheme::Mono.palette());
        let black = out.iter().filter(|&&i| i == 0).count();
        assert!(black > 0 && black < 1024, "expected a mix of inks, got {black}/1024 black");
    }

    #[test]
    fn flat_midgray_black_white_ratio_is_balanced() {
        // Beyond "more than one ink": for a flat mid-gray field on MONO, the
        // black/white split should be roughly balanced, not grossly skewed.
        // A skew here (e.g. 99/1) would indicate the error normalization is
        // wrong even though every other test still passes.
        let img = gray_image(128, 1024);
        let out = dizzy_dither(&img, 32, 32, ColorScheme::Mono.palette());
        let black = out.iter().filter(|&&i| i == 0).count();
        let ratio = black as f64 / 1024.0;
        assert!(
            (0.3..=0.7).contains(&ratio),
            "expected roughly balanced black/white split, got {ratio:.3} black ({black}/1024)"
        );
    }

    #[test]
    fn error_is_conserved_while_neighbours_remain() {
        // Every pixel's error must be fully handed on (shares sum to 1.0) unless
        // every neighbour is already quantized. Verified here on the weighting
        // itself: for any non-empty subset of the 8 neighbours, the normalized
        // shares must sum to 1.
        for mask in 1u32..256 {
            let mut denom = 0.0;
            for (bit, (_, _, w)) in NEIGHBORS.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    denom += w;
                }
            }
            let total: f64 = NEIGHBORS
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, (_, _, w))| w / denom)
                .sum();
            assert!(
                (total - 1.0).abs() < 1e-12,
                "mask {mask:08b}: shares summed to {total}, expected 1.0"
            );
        }
    }

    #[test]
    fn exact_canonical_pixels_are_pinned() {
        // Mirrors algorithms.rs's equivalent test for raster error diffusion.
        let mut image = gray_image(128, 8);
        image[0..3].copy_from_slice(&[0, 255, 0]);   // pure green -> index 5 in BWGBRY
        image[9..12].copy_from_slice(&[0, 255, 0]);
        let out = dizzy_dither_with_canonical(
            &image, 4, 2,
            &crate::measured_palettes::SPECTRA_7_3_6COLOR,
            ColorScheme::Bwgbry.palette(),
        );
        assert_eq!(out[0], 5, "exact green at pixel 0 should be pinned");
        assert_eq!(out[3], 5, "exact green at pixel 3 should be pinned");
    }

    #[test]
    fn walk_visits_every_index_exactly_once() {
        // Powers of two, 2^k+1 (worst-case rejection), primes, and degenerate shapes.
        for n in [1usize, 2, 3, 4, 5, 7, 8, 9, 16, 17, 31, 64, 100, 255, 256, 257, 1000, 4096] {
            let mut seen = vec![0u32; n];
            let mut count = 0usize;
            for p in dizzy_order(n) {
                assert!(p < n, "n={n}: yielded out-of-range index {p}");
                seen[p] += 1;
                count += 1;
            }
            assert_eq!(count, n, "n={n}: walk yielded {count} indices, expected {n}");
            assert!(
                seen.iter().all(|&c| c == 1),
                "n={n}: some index was visited {:?} times, expected exactly once each",
                seen.iter().max()
            );
        }
    }

    #[test]
    fn walk_is_not_the_identity() {
        // A permutation that happened to be the identity would pass the bijection
        // test while making this mode a plain raster scan.
        let order: Vec<usize> = dizzy_order(256).collect();
        let identity: Vec<usize> = (0..256).collect();
        assert_ne!(order, identity, "walk degenerated into raster order");
    }

    #[test]
    fn cross_language_reference_vector() {
        // 4x4 horizontal ramp, BWR palette, dizzy. These literals are frozen and
        // mirrored verbatim in the Python and JavaScript suites; if any surface
        // drifts, exactly one of the three tests fails.
        let mut img = Vec::new();
        for y in 0..4u8 {
            for x in 0..4u8 {
                let v = x * 60 + y * 5;
                img.extend_from_slice(&[v, v, v]);
            }
        }
        let out = dizzy_dither(&img, 4, 4, ColorScheme::Bwr.palette());
        assert_eq!(out, vec![0, 0, 1, 0, 0, 2, 2, 1, 0, 1, 1, 1, 0, 0, 2, 1]);
    }
}

//! Wab ablation: sweep the chromatic-axes weight in `match_pixel_oklab` and report
//! mean OKLab ΔE between dithered output and the original image, downsampled to 4×4 blocks.
//!
//! Methodology: a 4×4 block average rewards dithering that *spatially averages* to the
//! correct color — which is the whole point of error diffusion. Per-pixel ΔE would
//! penalize dithering for having pixel-level color differences (which it must, by design).
//!
//! Usage:
//!   cargo run --release --example wab_sweep
//!
//! Iterates over `tests/fixtures/images/`. Uses Burkes diffusion against the Spectra 6-color
//! measured palette (the project's primary regression path). No tone/gamut preprocessing —
//! this isolates the matcher decision.

use epaper_dithering_core::color_space::{linear_channel_to_srgb, srgb_channel_to_linear};
use epaper_dithering_core::color_space_lab::{match_pixel_oklab, rgb_to_oklab, OkLab, PaletteLab};
use epaper_dithering_core::palette::Palette as RuntimePalette;
use epaper_dithering_core::srgb_palette::SrgbPalette as Palette;
use image::ImageReader;
use std::path::{Path, PathBuf};

const WABS: &[f64] = &[1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 6.0];
const BLOCK: usize = 4;

// Burkes kernel as (dx, dy, weight) triples.
const BURKES: &[(i32, i32, f64)] = &[
    ( 1, 0, 8.0 / 32.0),
    ( 2, 0, 4.0 / 32.0),
    (-2, 1, 2.0 / 32.0),
    (-1, 1, 4.0 / 32.0),
    ( 0, 1, 8.0 / 32.0),
    ( 1, 1, 4.0 / 32.0),
    ( 2, 1, 2.0 / 32.0),
];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images")
}

fn discover_images() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures_dir())
        .expect("cannot read fixtures/images")
        .filter_map(|e| {
            let e = e.ok()?;
            if !e.file_type().ok()?.is_file() { return None; }
            let p = e.path();
            let ext = p.extension()?.to_str()?;
            matches!(ext, "png" | "jpg" | "jpeg").then_some(p)
        })
        .collect();
    paths.sort();
    paths
}

/// Burkes diffusion in sRGB-float space, matching the algorithm used by the core but
/// parameterized over the Wab matcher weight.
fn dither_burkes_oklab(pixels: &[u8], width: usize, height: usize, palette: &Palette, wab: f64) -> Vec<u8> {
    let palette_linear: Vec<[f64; 3]> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| [
            srgb_channel_to_linear(r),
            srgb_channel_to_linear(g),
            srgb_channel_to_linear(b),
        ])
        .collect();
    let palette_lab = PaletteLab::from_linear_rgb(&palette_linear);
    let palette_srgb_f: Vec<[f64; 3]> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| [r as f64, g as f64, b as f64])
        .collect();

    // u8 sRGB -> linear LUT
    let lut: Vec<f64> = (0u8..=255).map(srgb_channel_to_linear).collect();

    let mut buf: Vec<f64> = pixels.iter().map(|&v| v as f64).collect();
    let mut output = vec![0u8; width * height];

    for y in 0..height {
        let reverse = y % 2 == 1;
        for xi in 0..width {
            let x = if reverse { width - 1 - xi } else { xi };
            let i = (y * width + x) * 3;

            let rs = buf[i].clamp(0.0, 255.0);
            let gs = buf[i + 1].clamp(0.0, 255.0);
            let bs = buf[i + 2].clamp(0.0, 255.0);

            let r_lin = lut[rs.round() as usize];
            let g_lin = lut[gs.round() as usize];
            let b_lin = lut[bs.round() as usize];
            let pixel_lab = rgb_to_oklab(r_lin, g_lin, b_lin);
            let best_idx = match_pixel_oklab(pixel_lab, &palette_lab, wab);

            output[y * width + x] = best_idx as u8;

            let err_r = rs - palette_srgb_f[best_idx][0];
            let err_g = gs - palette_srgb_f[best_idx][1];
            let err_b = bs - palette_srgb_f[best_idx][2];

            for &(dx, dy, w) in BURKES {
                let edx = if reverse { -dx } else { dx };
                let nx = x as i64 + edx as i64;
                let ny = y as i64 + dy as i64;
                if nx >= 0 && nx < width as i64 && ny >= 0 && ny < height as i64 {
                    let ni = (ny as usize * width + nx as usize) * 3;
                    buf[ni]     += err_r * w;
                    buf[ni + 1] += err_g * w;
                    buf[ni + 2] += err_b * w;
                }
            }
        }
    }
    output
}

/// Convert a sRGB byte image to an OKLab image (one OkLab per pixel, row-major).
fn to_oklab_image(pixels: &[u8]) -> Vec<OkLab> {
    let lut: Vec<f64> = (0u8..=255).map(srgb_channel_to_linear).collect();
    pixels
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| rgb_to_oklab(lut[c[0] as usize], lut[c[1] as usize], lut[c[2] as usize]))
        .collect()
}

/// Convert palette indices back to OKLab pixels via the palette's sRGB colors.
fn indices_to_oklab(indices: &[u8], palette: &Palette) -> Vec<OkLab> {
    let lut: Vec<f64> = (0u8..=255).map(srgb_channel_to_linear).collect();
    let pal_lab: Vec<OkLab> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| rgb_to_oklab(lut[r as usize], lut[g as usize], lut[b as usize]))
        .collect();
    indices.iter().map(|&i| pal_lab[i as usize]).collect()
}

/// Average OKLab values over `BLOCK × BLOCK` non-overlapping tiles.
fn block_average(lab: &[OkLab], width: usize, height: usize) -> Vec<OkLab> {
    let bw = width / BLOCK;
    let bh = height / BLOCK;
    let mut out = Vec::with_capacity(bw * bh);
    for by in 0..bh {
        for bx in 0..bw {
            let (mut l, mut a, mut b) = (0.0_f64, 0.0_f64, 0.0_f64);
            for dy in 0..BLOCK {
                for dx in 0..BLOCK {
                    let p = lab[(by * BLOCK + dy) * width + (bx * BLOCK + dx)];
                    l += p.l;
                    a += p.a;
                    b += p.b;
                }
            }
            let n = (BLOCK * BLOCK) as f64;
            out.push(OkLab { l: l / n, a: a / n, b: b / n });
        }
    }
    out
}

/// Mean OKLab ΔE (Euclidean) between two block-average images.
fn mean_delta_e(a: &[OkLab], b: &[OkLab]) -> f64 {
    let sum: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(p, q)| {
            let dl = p.l - q.l;
            let da = p.a - q.a;
            let db = p.b - q.b;
            (dl * dl + da * da + db * db).sqrt()
        })
        .sum();
    sum / a.len() as f64
}

fn main() {
    // Suppress the unused-import warning when only some helpers are needed by the example.
    let _ = linear_channel_to_srgb;

    let palette_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/palettes/spectra_7_3_6color.json");
    let runtime = RuntimePalette::from_json(&std::fs::read_to_string(palette_file).expect("read palette")).expect("valid palette");
    let palette_owned = Palette::from(&runtime);
    let palette: &Palette = &palette_owned;
    let images = discover_images();

    println!();
    println!("Wab ablation — Burkes + Spectra 6-color, 4×4-block mean OKLab ΔE (lower is better)");
    println!();
    print!("| image | ");
    for w in WABS { print!("Wab={w} | "); }
    println!();
    print!("|---|");
    for _ in WABS { print!("---|"); }
    println!();

    // Per-Wab running mean across images.
    let mut totals = vec![0.0_f64; WABS.len()];

    for path in &images {
        let img = ImageReader::open(path)
            .expect("open image")
            .decode()
            .expect("decode image")
            .into_rgb8();
        let (w, h) = img.dimensions();
        let pixels = img.into_raw();
        let width = w as usize;
        let height = h as usize;

        let original_lab = to_oklab_image(&pixels);
        let original_blocks = block_average(&original_lab, width, height);

        let stem = path.file_stem().unwrap().to_string_lossy();
        print!("| {stem} | ");

        for (i, &wab) in WABS.iter().enumerate() {
            let indices = dither_burkes_oklab(&pixels, width, height, palette, wab);
            let dith_lab = indices_to_oklab(&indices, palette);
            let dith_blocks = block_average(&dith_lab, width, height);
            let de = mean_delta_e(&original_blocks, &dith_blocks);
            totals[i] += de;
            print!("{de:.4} | ");
        }
        println!();
    }

    let n = images.len() as f64;
    print!("| **mean** | ");
    for total in &totals {
        print!("**{:.4}** | ", total / n);
    }
    println!();

    // Pick the winner (lowest mean ΔE).
    let (best_i, best_de) = totals
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    println!();
    println!("→ Optimal Wab = {} (mean ΔE = {:.4})", WABS[best_i], best_de / n);
}

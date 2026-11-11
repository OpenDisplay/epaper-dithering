//! Render side-by-side dithered outputs at multiple Wab values for visual A/B.
//!
//! Output: /tmp/wab_compare/<stem>__wab_compare.png — one composite per image with
//! the variants stacked horizontally and labeled across the top.
//!
//! Usage:
//!   cargo run --release --example wab_compare

use epaper_dithering_core::color_space::srgb_channel_to_linear;
use epaper_dithering_core::color_space_lab::{match_pixel_oklab, rgb_to_oklab, PaletteLab};
use epaper_dithering_core::palette::Palette as RuntimePalette;
use epaper_dithering_core::srgb_palette::SrgbPalette as Palette;
use image::{ImageReader, RgbImage};
use std::path::{Path, PathBuf};

const BURKES: &[(i32, i32, f64)] = &[
    ( 1, 0, 8.0 / 32.0),
    ( 2, 0, 4.0 / 32.0),
    (-2, 1, 2.0 / 32.0),
    (-1, 1, 4.0 / 32.0),
    ( 0, 1, 8.0 / 32.0),
    ( 1, 1, 4.0 / 32.0),
    ( 2, 1, 2.0 / 32.0),
];

/// (image_stem, candidate Wabs) — chosen to span the regimes we care about.
const TARGETS: &[(&str, &[f64])] = &[
    ("olympiapark",      &[1.5, 2.0, 2.5, 3.0]), // natural photo — fixture optimum
    ("cat_orange",       &[1.0, 2.0, 2.5]),       // desaturated outlier — low Wab favored
    ("unicorn",          &[1.5, 2.5, 4.0]),       // vivid outlier — high Wab favored
    ("frankfurt_nacht",  &[1.5, 2.0, 2.5, 3.0]), // night scene with mixed saturations
];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images")
}

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

fn indices_to_rgb(indices: &[u8], palette: &Palette) -> Vec<u8> {
    let mut out = Vec::with_capacity(indices.len() * 3);
    for &i in indices {
        out.extend_from_slice(&palette.colors[i as usize]);
    }
    out
}

fn find_image(stem: &str) -> Option<PathBuf> {
    let dir = fixtures_dir();
    for ext in ["png", "jpg", "jpeg"] {
        let p = dir.join(format!("{stem}.{ext}"));
        if p.exists() { return Some(p); }
    }
    None
}

fn main() {
    let palette_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/palettes/spectra_7_3_6color.json");
    let runtime = RuntimePalette::from_json(&std::fs::read_to_string(palette_file).expect("read palette")).expect("valid palette");
    let palette_owned = Palette::from(&runtime);
    let palette: &Palette = &palette_owned;
    let out_dir = Path::new("/tmp/wab_compare");
    std::fs::create_dir_all(out_dir).expect("create /tmp/wab_compare");

    const GAP: u32 = 8;

    for &(stem, wabs) in TARGETS {
        let path = match find_image(stem) {
            Some(p) => p,
            None => { eprintln!("skip: no image found for {stem}"); continue; }
        };
        let img = ImageReader::open(&path).expect("open").decode().expect("decode").into_rgb8();
        let (w, h) = img.dimensions();
        let pixels = img.into_raw();
        let width = w as usize;
        let height = h as usize;

        let n = wabs.len() as u32;
        let composite_w = w * n + GAP * (n - 1);
        let mut composite = vec![255u8; (composite_w * h * 3) as usize];

        for (i, &wab) in wabs.iter().enumerate() {
            let indices = dither_burkes_oklab(&pixels, width, height, palette, wab);
            let rgb = indices_to_rgb(&indices, palette);

            let x_offset = (i as u32) * (w + GAP);
            for y in 0..h {
                let src_start = (y * w * 3) as usize;
                let src_end = src_start + (w * 3) as usize;
                let dst_start = ((y * composite_w + x_offset) * 3) as usize;
                let dst_end = dst_start + (w * 3) as usize;
                composite[dst_start..dst_end].copy_from_slice(&rgb[src_start..src_end]);
            }
        }

        let out_path = out_dir.join(format!("{stem}__wab_compare.png"));
        RgbImage::from_raw(composite_w, h, composite)
            .expect("buffer mismatch")
            .save(&out_path)
            .expect("save");

        let labels: Vec<String> = wabs.iter().map(|w| format!("Wab={w}")).collect();
        println!("{stem:20}  →  {}  [{}]", out_path.display(), labels.join(" | "));
    }

    println!();
    println!("Open the PNGs in an image viewer to compare. Variants are left-to-right in the order printed.");
}

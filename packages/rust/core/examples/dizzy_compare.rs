//! Compare dizzy dithering against Burkes and Floyd-Steinberg.
//!
//! Metric: mean OKLab dE between the source and the dithered result, both
//! averaged over 4x4 blocks. Block-averaging measures whether local average
//! colour is preserved; per-pixel dE would only measure dither noise.
//!
//!   cargo run --release --example dizzy_compare

use epaper_dithering_core::{
    color_space::srgb_channel_to_linear,
    color_space_lab::rgb_to_oklab,
    dither, dither_with_canonical,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::SPECTRA_7_3_6COLOR,
    palettes::{ColorScheme, Palette},
    types::ImageBuffer,
    DitherConfig,
};

const BLOCK: usize = 4;

/// Mean OKLab dE between two flat RGB buffers, averaged over BLOCK x BLOCK tiles.
fn block_delta_e(a: &[u8], b: &[u8], width: usize, height: usize) -> f64 {
    let mut total = 0.0;
    let mut blocks = 0usize;
    for by in (0..height).step_by(BLOCK) {
        for bx in (0..width).step_by(BLOCK) {
            let (mut sa, mut sb, mut n) = ([0.0; 3], [0.0; 3], 0.0);
            for y in by..(by + BLOCK).min(height) {
                for x in bx..(bx + BLOCK).min(width) {
                    let i = (y * width + x) * 3;
                    for c in 0..3 {
                        sa[c] += srgb_channel_to_linear(a[i + c]);
                        sb[c] += srgb_channel_to_linear(b[i + c]);
                    }
                    n += 1.0;
                }
            }
            let la = rgb_to_oklab(sa[0] / n, sa[1] / n, sa[2] / n);
            let lb = rgb_to_oklab(sb[0] / n, sb[1] / n, sb[2] / n);
            let d = ((la.l - lb.l).powi(2) + (la.a - lb.a).powi(2) + (la.b - lb.b).powi(2)).sqrt();
            total += d;
            blocks += 1;
        }
    }
    total / blocks as f64
}

/// Expand palette indices back into a flat RGB buffer.
fn to_rgb(indices: &[u8], palette: &Palette) -> Vec<u8> {
    indices
        .iter()
        .flat_map(|&i| palette.colors[i as usize])
        .collect()
}

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images");
    let mut images: Vec<_> = std::fs::read_dir(&dir)
        .expect("fixtures/images")
        .filter_map(|e| {
            let e = e.ok()?;
            e.file_type().ok()?.is_file().then(|| e.file_name().into_string().ok())?
        })
        .filter(|n| n.ends_with(".png") || n.ends_with(".jpg") || n.ends_with(".jpeg"))
        .collect();
    images.sort();

    let modes = [
        ("dizzy", DitherMode::Dizzy),
        ("burkes", DitherMode::Burkes),
        ("floyd_steinberg", DitherMode::FloydSteinberg),
    ];

    println!("{:<22} {:<10} {:<16} {:>10}", "image", "palette", "mode", "mean dE");
    let mut totals = [(0.0, 0usize); 3];

    for name in &images {
        let img = image::open(dir.join(name)).expect("load").to_rgb8();
        let (w, h) = img.dimensions();
        let (w, h) = (w as usize, h as usize);
        let src = img.into_raw();
        let buf = ImageBuffer::new(&src, w);

        for (pal_name, is_measured) in [("spectra6", true), ("mono", false)] {
            for (mi, (mode_name, mode)) in modes.iter().enumerate() {
                let (indices, out_palette): (Vec<u8>, &Palette) = if is_measured {
                    let cfg = DitherConfig {
                        mode: *mode,
                        tone: ToneCompression::Auto,
                        gamut: GamutCompression::Auto,
                        ..Default::default()
                    };
                    (
                        dither_with_canonical(
                            &buf,
                            &SPECTRA_7_3_6COLOR,
                            ColorScheme::Bwgbry.palette(),
                            cfg,
                        ),
                        &SPECTRA_7_3_6COLOR,
                    )
                } else {
                    let cfg = DitherConfig { mode: *mode, ..Default::default() };
                    (dither(&buf, ColorScheme::Mono.palette(), cfg), ColorScheme::Mono.palette())
                };

                let rgb = to_rgb(&indices, out_palette);
                let de = block_delta_e(&src, &rgb, w, h);
                totals[mi].0 += de;
                totals[mi].1 += 1;
                println!("{name:<22} {pal_name:<10} {mode_name:<16} {de:>10.4}");
            }
        }
    }

    println!("\n{:<33} {:<16} {:>10}", "SUMMARY", "mode", "mean dE");
    for (mi, (mode_name, _)) in modes.iter().enumerate() {
        let (sum, n) = totals[mi];
        println!("{:<33} {mode_name:<16} {:>10.4}", "", sum / n as f64);
    }
}

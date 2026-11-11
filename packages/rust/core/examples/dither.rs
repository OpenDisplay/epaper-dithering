// Quick visual test: dither a real image and save the result.
//
// Usage:
//   cargo run --example dither <input> [output] [palette] [mode] [gamut]
//
// Examples:
//   cargo run --example dither photo.jpg
//   cargo run --example dither photo.jpg out.png spectra_7_3_6color stucki 1.0
//   cargo run --example dither photo.jpg out.png my_panel.json stucki 0.5
//
// Palette: a path to an epaper-palette JSON file, or the name of a file in
//          examples/palettes/ (without .json). Default: ideal_bwr.
// Modes:   none, ordered, floyd_steinberg, burkes, atkinson, stucki, sierra, sierra_lite,
//          jarvis_judice_ninke, dizzy
// Gamut:   0.0-1.0  (default: 0)

use epaper_dithering_core::enums::DitherMode;
use epaper_dithering_core::palette::Palette;
use epaper_dithering_core::types::ImageBuffer;
use epaper_dithering_core::{dither, DitherOptions};
use image::{ImageReader, RgbImage};
use std::env;
use std::path::Path;

fn load_palette(arg: &str) -> Palette {
    let path = if arg.ends_with(".json") {
        Path::new(arg).to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/palettes").join(format!("{arg}.json"))
    };
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("cannot read palette {}: {e}", path.display());
        std::process::exit(1);
    });
    Palette::from_json(&text).unwrap_or_else(|e| {
        eprintln!("{}: {e}", path.display());
        std::process::exit(1);
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: dither <input> [output] [palette] [mode] [gamut]");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = args.get(2).map(String::as_str).unwrap_or("dithered.png");
    let palette_name = args.get(3).map(String::as_str).unwrap_or("ideal_bwr");
    let mode_name   = args.get(4).map(String::as_str).unwrap_or("burkes");
    let gamut_name  = args.get(5).map(String::as_str).unwrap_or("0");

    let palette = load_palette(palette_name);
    let mode: DitherMode = mode_name.parse().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    // Load image
    let img = ImageReader::open(input_path)
        .expect("failed to open image")
        .decode()
        .expect("failed to decode image")
        .into_rgb8();

    let (width, height) = img.dimensions();
    let buf = ImageBuffer::new(img.as_raw(), width as usize).expect("decoded image is a whole RGB buffer");

    let gamut: f64 = gamut_name.parse().unwrap_or_else(|_| {
        eprintln!("Gamut must be a number from 0.0 to 1.0, got {gamut_name:?}");
        std::process::exit(1);
    });

    println!("Input:  {input_path} ({width}x{height})");
    println!("Palette: {palette_name}  Mode: {mode_name}  Gamut: {gamut_name}");

    let t0 = std::time::Instant::now();
    let indices = dither(&buf, &palette, &DitherOptions {
        mode, gamut, ..Default::default()
    });
    let elapsed = t0.elapsed();

    println!("Dither: {:.1}ms  ({} mpx/s)",
        elapsed.as_secs_f64() * 1000.0,
        (width as f64 * height as f64 / 1_000_000.0 / elapsed.as_secs_f64()) as u64,
    );

    // Map indices back to RGB for a viewable output
    let mut out_pixels: Vec<u8> = Vec::with_capacity(width as usize * height as usize * 3);
    for idx in &indices {
        out_pixels.extend_from_slice(&palette.preview_srgb()[*idx as usize]);
    }

    RgbImage::from_raw(width, height, out_pixels)
        .expect("buffer size mismatch")
        .save(output_path)
        .expect("failed to save output");

    println!("Output: {output_path}");
}

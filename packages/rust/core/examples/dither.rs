// Quick visual test: dither a real image and save the result.
//
// Usage:
//   cargo run --example dither <input> [output] [scheme] [mode] [tone] [gamut]
//
// Examples:
//   cargo run --example dither photo.jpg
//   cargo run --example dither photo.jpg out.png spectra stucki auto none
//   cargo run --example dither photo.jpg out.png spectra stucki 0.8 0.5
//
// Schemes: mono, bwr, bwy, bwry, bwgbry, grayscale4, grayscale16, seven_color, bwgbry_split
//          spectra, spectra_v2, mono_4_26, bwry_4_2, bwry_3_97, solum_bwr, hanshow_bwr, hanshow_bwy
// Modes:   none, ordered, floyd_steinberg, burkes, atkinson, stucki, sierra, sierra_lite, jjn, dizzy
// Tone:    auto, none, 0.0-1.0  (default: none)
// Gamut:   none, auto, 0.0-1.0  (default: none)

use epaper_dithering_core::enums::{DitherMode, GamutCompression, ToneCompression};
use epaper_dithering_core::measured_palettes::{
    BWRY_3_97, BWRY_4_2, HANSHOW_BWR, HANSHOW_BWY, MONO_4_26, SOLUM_BWR, SPECTRA_7_3_6COLOR,
    SPECTRA_7_3_6COLOR_V2,
};
use epaper_dithering_core::palettes::{ColorScheme, Palette};
use epaper_dithering_core::types::ImageBuffer;
use epaper_dithering_core::{dither, DitherConfig};
use image::{ImageReader, RgbImage};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: dither <input> [output] [scheme] [mode]");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = args.get(2).map(String::as_str).unwrap_or("dithered.png");
    let scheme_name = args.get(3).map(String::as_str).unwrap_or("bwr");
    let mode_name   = args.get(4).map(String::as_str).unwrap_or("burkes");
    let tone_name   = args.get(5).map(String::as_str).unwrap_or("none");
    let gamut_name  = args.get(6).map(String::as_str).unwrap_or("none");

    let palette: &Palette = match scheme_name {
        "mono"         => ColorScheme::Mono.palette(),
        "bwr"          => ColorScheme::Bwr.palette(),
        "bwy"          => ColorScheme::Bwy.palette(),
        "bwry"         => ColorScheme::Bwry.palette(),
        "bwgbry"       => ColorScheme::Bwgbry.palette(),
        "grayscale4"   => ColorScheme::Grayscale4.palette(),
        "grayscale16"  => ColorScheme::Grayscale16.palette(),
        "seven_color"  => ColorScheme::SevenColor.palette(),
        "bwgbry_split" => ColorScheme::BwgbrySplit.palette(),
        "spectra"      => &SPECTRA_7_3_6COLOR,
        "spectra_v2"   => &SPECTRA_7_3_6COLOR_V2,
        "mono_4_26"    => &MONO_4_26,
        "bwry_4_2"     => &BWRY_4_2,
        "bwry_3_97"    => &BWRY_3_97,
        "solum_bwr"    => &SOLUM_BWR,
        "hanshow_bwr"  => &HANSHOW_BWR,
        "hanshow_bwy"  => &HANSHOW_BWY,
        other => { eprintln!("Unknown scheme: {other}"); std::process::exit(1); }
    };

    let mode = match mode_name {
        "none"              => DitherMode::None,
        "ordered"           => DitherMode::Ordered,
        "floyd_steinberg"   => DitherMode::FloydSteinberg,
        "burkes"            => DitherMode::Burkes,
        "atkinson"          => DitherMode::Atkinson,
        "stucki"            => DitherMode::Stucki,
        "sierra"            => DitherMode::Sierra,
        "sierra_lite"       => DitherMode::SierraLite,
        "jjn"               => DitherMode::JarvisJudiceNinke,
        "dizzy"             => DitherMode::Dizzy,
        other => { eprintln!("Unknown mode: {other}"); std::process::exit(1); }
    };

    // Load image
    let img = ImageReader::open(input_path)
        .expect("failed to open image")
        .decode()
        .expect("failed to decode image")
        .into_rgb8();

    let (width, height) = img.dimensions();
    let buf = ImageBuffer::new(img.as_raw(), width as usize);

    let tone = match tone_name {
        "auto"         => ToneCompression::Auto,
        "none" | "0"   => ToneCompression::Fixed(0.0),
        s => match s.parse::<f64>() {
            Ok(v)  => ToneCompression::Fixed(v),
            Err(_) => { eprintln!("Unknown tone: {s}"); std::process::exit(1); }
        },
    };

    let gamut = match gamut_name {
        "none" | "0"  => GamutCompression::None,
        "auto" | "1"  => GamutCompression::Auto,
        s => match s.parse::<f64>() {
            Ok(v)  => GamutCompression::Fixed(v),
            Err(_) => { eprintln!("Unknown gamut: {s}"); std::process::exit(1); }
        },
    };

    println!("Input:  {input_path} ({width}x{height})");
    println!("Scheme: {scheme_name}  Mode: {mode_name}  Tone: {tone_name}  Gamut: {gamut_name}");

    let t0 = std::time::Instant::now();
    let indices = dither(&buf, palette, DitherConfig {
        mode, tone, gamut, ..Default::default()
    });
    let elapsed = t0.elapsed();

    println!("Dither: {:.1}ms  ({} mpx/s)",
        elapsed.as_secs_f64() * 1000.0,
        (width as f64 * height as f64 / 1_000_000.0 / elapsed.as_secs_f64()) as u64,
    );

    // Map indices back to RGB for a viewable output
    let mut out_pixels: Vec<u8> = Vec::with_capacity(width as usize * height as usize * 3);
    for idx in &indices {
        out_pixels.extend_from_slice(&palette.colors[*idx as usize]);
    }

    RgbImage::from_raw(width, height, out_pixels)
        .expect("buffer size mismatch")
        .save(output_path)
        .expect("failed to save output");

    println!("Output: {output_path}");
}

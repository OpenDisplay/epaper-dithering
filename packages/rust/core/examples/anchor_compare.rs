//! Side-by-side renders with anchoring off and on, for visual review.
//!
//!   cargo run --release --example anchor_compare [out_dir]
//!
//! For every fixture image and every palette below, writes `<out>/<palette>/<image>.png`:
//! source | anchoring off | anchoring on. The dithered panels are drawn in each palette's
//! preview colours, i.e. what the panel shows, normalised to its white.

use std::path::{Path, PathBuf};

use epaper_dithering_core::{dither, palette::Palette, types::ImageBuffer, DitherOptions};
use image::RgbImage;

const PALETTES: &[(&str, &str)] = &[
    ("spectra6", "../../../examples/palettes/spectra_7_3_6color.json"),
    ("bwry16", "tests/fixtures/palettes/bwry16_odbeeba6.json"),
];

fn load(rel: &str) -> Option<Palette> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let text = std::fs::read_to_string(&path).ok()?;
    Some(Palette::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

fn render(indices: &[u8], palette: &Palette, w: u32, h: u32) -> RgbImage {
    let rgb = indices.iter().flat_map(|&i| palette.preview_srgb()[i as usize]).collect();
    RgbImage::from_raw(w, h, rgb).expect("render size")
}

fn hstack(images: &[RgbImage]) -> RgbImage {
    const GAP: u32 = 4;
    let h = images.iter().map(RgbImage::height).max().unwrap_or(0);
    let w = images.iter().map(|i| i.width() + GAP).sum::<u32>() - GAP;
    let mut out = RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]));
    let mut x = 0;
    for img in images {
        image::imageops::replace(&mut out, img, i64::from(x), 0);
        x += img.width() + GAP;
    }
    out
}

fn main() {
    let out = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("anchor_compare"));
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images");
    let mut images: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/images")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "png" || e == "jpg"))
        .collect();
    images.sort();

    for &(name, rel) in PALETTES {
        let Some(palette) = load(rel) else {
            println!("skip {name}: {rel} not found");
            continue;
        };
        let pdir = out.join(name);
        std::fs::create_dir_all(&pdir).expect("create output dir");
        for path in &images {
            let src = image::open(path).expect("load image").to_rgb8();
            let (w, h) = src.dimensions();
            let buf = ImageBuffer::new(src.as_raw(), w as usize).expect("whole RGB buffer");
            let off = dither(&buf, &palette, &DitherOptions { anchor: false, ..Default::default() });
            let on = dither(&buf, &palette, &DitherOptions::default());
            let file = pdir.join(path.file_name().unwrap());
            hstack(&[src.clone(), render(&off, &palette, w, h), render(&on, &palette, w, h)])
                .save(&file)
                .expect("save");
            println!("wrote {}", file.display());
        }
    }
}

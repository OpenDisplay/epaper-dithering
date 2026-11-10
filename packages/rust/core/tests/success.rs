//! The release's success test: a dashboard with pure-ink text and fills next to a photo,
//! dithered against measured palettes. Every pixel that asked for an ink must get it.

use std::path::Path;

use epaper_dithering_core::{
    dither,
    enums::ToneCompression,
    palette::Palette,
    types::ImageBuffer,
    DitherOptions,
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn palette(rel: &str) -> Palette {
    Palette::from_json(&std::fs::read_to_string(root().join(rel)).unwrap()).unwrap()
}

fn bwry16() -> Palette {
    palette("tests/fixtures/palettes/bwry16_measured.json")
}

fn spectra6() -> Palette {
    palette("../../../examples/palettes/spectra_7_3_6color.json")
}

fn fixture() -> (Vec<u8>, usize) {
    let img = image::open(root().join("tests/fixtures/images/success/dashboard_photo.png")).unwrap().to_rgb8();
    let w = img.width() as usize;
    (img.into_raw(), w)
}

/// Every input pixel equal to an ink's nominal comes out as that ink, and each ink the
/// image uses is actually exercised.
fn assert_inks_solid(palette: &Palette, options: &DitherOptions) {
    let (pixels, w) = fixture();
    let out = dither(&ImageBuffer::new(&pixels, w).unwrap(), palette, options);
    for (index, ink) in palette.entries().iter().enumerate().filter_map(|(i, e)| e.ink.as_ref().map(|k| (i, k))) {
        let mut seen = 0;
        for (p, rgb) in pixels.as_chunks::<3>().0.iter().enumerate() {
            if *rgb == ink.nominal {
                seen += 1;
                assert_eq!(out[p] as usize, index, "{} pixel at ({}, {}) was not solid", ink.name, p % w, p / w);
            }
        }
        if ["black", "white", "red", "yellow"].contains(&ink.name.as_str()) {
            assert!(seen > 0, "fixture has no pure {} pixels", ink.name);
        }
    }
}

#[test]
fn pure_inks_are_solid_bwry16() {
    assert_inks_solid(&bwry16(), &DitherOptions::default());
}

#[test]
fn pure_inks_are_solid_spectra6() {
    assert_inks_solid(&spectra6(), &DitherOptions::default());
}

#[test]
fn pure_inks_survive_preprocessing() {
    let o = DitherOptions { exposure: 1.3, tone: ToneCompression::Auto, ..Default::default() };
    assert_inks_solid(&bwry16(), &o);
    assert_inks_solid(&spectra6(), &o);
}

#[test]
fn solid_rectangles_are_one_index() {
    let (pixels, w) = fixture();
    let p = bwry16();
    let out = dither(&ImageBuffer::new(&pixels, w).unwrap(), &p, &DitherOptions::default());
    for (x0, y0, x1, y1) in [(20, 300, 120, 380), (140, 300, 240, 380), (260, 300, 360, 380)] {
        let first = out[y0 * w + x0];
        for y in y0..y1 {
            for x in x0..x1 {
                assert_eq!(out[y * w + x], first, "rectangle at ({x0}, {y0}) is not one index");
            }
        }
    }
}

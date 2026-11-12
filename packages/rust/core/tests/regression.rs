//! Visual regression tests using real photographs.
//!
//! References are stored as raw palette-index `.bin` files in `tests/fixtures/references/`.
//! To regenerate all references (e.g. after an intentional algorithm change):
//!
//!   UPDATE_FIXTURES=1 cargo test --test regression

use std::path::{Path, PathBuf};

use epaper_dithering_core::{
    dither, DitherOptions,
    enums::DitherMode,
    palette::Palette,
    types::ImageBuffer,
};

// ── Paths ─────────────────────────────────────────────────────────────────────

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn image_path(filename: &str) -> PathBuf {
    fixtures_dir().join("images").join(filename)
}

fn reference_path(image_stem: &str, tag: &str) -> PathBuf {
    fixtures_dir()
        .join("references")
        .join(format!("{image_stem}__{tag}.bin"))
}

fn example_palette(name: &str) -> Palette {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/palettes").join(name);
    Palette::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn spectra6() -> Palette {
    example_palette("spectra_7_3_6color.json")
}

fn mono() -> Palette {
    example_palette("ideal_mono.json")
}

// ── Image loading ─────────────────────────────────────────────────────────────

fn load_rgb(filename: &str) -> (Vec<u8>, usize, usize) {
    let img = image::open(image_path(filename))
        .unwrap_or_else(|e| panic!("failed to load {filename}: {e}"))
        .to_rgb8();
    let (w, h) = img.dimensions();
    (img.into_raw(), w as usize, h as usize)
}

// ── Regression driver ─────────────────────────────────────────────────────────

/// Run `dither()` on `filename` and compare against (or write) the stored reference.
///
/// Anchoring and snapping are off: this suite pins the algorithms themselves. Ink
/// behaviour is covered by `tests/success.rs`.
///
/// `tag` identifies the combination, e.g. `"burkes_spectra6_auto"`.
fn assert_regression(
    filename: &str,
    tag: &str,
    mode: DitherMode,
    palette: Palette,
    gamut: f64,
) {
    let (pixels, w, _h) = load_rgb(filename);
    let img = ImageBuffer::new(&pixels, w).unwrap();
    let output = dither(&img, &palette, &DitherOptions {
        mode, gamut, anchor: false, snap: false, ..Default::default()
    });

    let stem = Path::new(filename).file_stem().unwrap().to_str().unwrap();
    let ref_path = reference_path(stem, tag);

    if std::env::var("UPDATE_FIXTURES").is_ok() {
        std::fs::create_dir_all(ref_path.parent().unwrap()).unwrap();
        std::fs::write(&ref_path, &output)
            .unwrap_or_else(|e| panic!("failed to write reference {ref_path:?}: {e}"));
        return;
    }

    let reference = std::fs::read(&ref_path).unwrap_or_else(|_| {
        panic!(
            "Reference not found: {ref_path:?}\nRun with UPDATE_FIXTURES=1 to generate it."
        )
    });

    // Not assert_eq!: on failure it would print both full index buffers.
    let differing = output.iter().zip(&reference).filter(|(a, b)| a != b).count();
    assert!(
        output.len() == reference.len() && differing == 0,
        "Regression failure: {filename} × {tag}\n\
         {differing} of {} pixels differ from the reference (reference has {}). If this change \
         is intentional, regenerate with UPDATE_FIXTURES=1.",
        output.len(),
        reference.len(),
    );
}

// ── Test image discovery ──────────────────────────────────────────────────────

/// Returns all image filenames in `tests/fixtures/images/` (non-recursive).
/// Images in `benchmark_only/` are excluded — those are too large for regression tests.
/// To add a new image, just drop it into `tests/fixtures/images/` and run
/// `UPDATE_FIXTURES=1 cargo test --test regression`.
fn discover_images() -> Vec<String> {
    let dir = fixtures_dir().join("images");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read fixtures/images: {e}"))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_file() {
                return None; // skip subdirectories (e.g. benchmark_only/)
            }
            let name = entry.file_name().into_string().ok()?;
            let ext = std::path::Path::new(&name).extension()?.to_str()?;
            matches!(ext, "png" | "jpg" | "jpeg").then_some(name)
        })
        .collect();
    names.sort(); // deterministic order
    names
}

// ── Regression suites ─────────────────────────────────────────────────────────

/// Primary path: Burkes + 6-color measured palette + full gamut compression.
/// (The `_auto` in the tags is historical: it now means gamut 1.0.)
#[test]
fn burkes_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "burkes_spectra6_auto",
            DitherMode::Burkes,
            spectra6(),
            1.0,
        );
    }
}

/// Secondary path: Floyd-Steinberg + monochrome + no preprocessing.
/// Fast, no measured palette — exercises pure error-diffusion on a 2-color palette.
#[test]
fn floyd_steinberg_mono_raw() {
    for img in discover_images() {
        assert_regression(
            &img,
            "floyd_steinberg_mono_raw",
            DitherMode::FloydSteinberg,
            mono(),
            0.0,
        );
    }
}

/// Ordered dithering + 6-color measured palette + full gamut compression.
/// Different algorithm family — verifies the Bayer path independently.
#[test]
fn ordered_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "ordered_spectra6_auto",
            DitherMode::Ordered,
            spectra6(),
            1.0,
        );
    }
}

/// Dizzy + 6-color measured palette + full gamut compression.
/// Random-traversal family — pins the permutation walk against accidental change.
#[test]
fn dizzy_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "dizzy_spectra6_auto",
            DitherMode::Dizzy,
            spectra6(),
            1.0,
        );
    }
}

/// Dizzy + monochrome + no preprocessing.
/// Two-color palette makes any traversal-order change highly visible in the diff.
#[test]
fn dizzy_mono_raw() {
    for img in discover_images() {
        assert_regression(
            &img,
            "dizzy_mono_raw",
            DitherMode::Dizzy,
            mono(),
            0.0,
        );
    }
}
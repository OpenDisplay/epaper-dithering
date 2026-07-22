//! Visual regression tests using real photographs.
//!
//! References are stored as raw palette-index `.bin` files in `tests/fixtures/references/`.
//! To regenerate all references (e.g. after an intentional algorithm change):
//!
//!   UPDATE_FIXTURES=1 cargo test --test regression

use std::path::{Path, PathBuf};

use epaper_dithering_core::{
    dither, DitherConfig,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::SPECTRA_7_3_6COLOR,
    palettes::ColorScheme,
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
/// `tag` identifies the combination, e.g. `"burkes_spectra6_auto"`.
fn assert_regression(
    filename: &str,
    tag: &str,
    mode: DitherMode,
    palette: impl AsRef<epaper_dithering_core::palettes::Palette>,
    tone: ToneCompression,
    gamut: GamutCompression,
) {
    let (pixels, w, _h) = load_rgb(filename);
    let img = ImageBuffer::new(&pixels, w);
    let output = dither(&img, palette, DitherConfig {
        mode, tone, gamut, ..Default::default()
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

    assert_eq!(
        output, reference,
        "Regression failure: {filename} × {tag}\n\
         Output differs from reference. If this change is intentional, \
         regenerate with UPDATE_FIXTURES=1."
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

/// Primary path: Burkes + 6-color measured palette + auto tone & gamut.
/// This is the most common real-world usage.
#[test]
fn burkes_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "burkes_spectra6_auto",
            DitherMode::Burkes,
            &SPECTRA_7_3_6COLOR,
            ToneCompression::Auto,
            GamutCompression::Auto,
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
            ColorScheme::Mono,
            ToneCompression::Fixed(0.0),
            GamutCompression::None,
        );
    }
}

/// Ordered dithering + 6-color measured palette + auto preprocessing.
/// Different algorithm family — verifies the Bayer path independently.
#[test]
fn ordered_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "ordered_spectra6_auto",
            DitherMode::Ordered,
            &SPECTRA_7_3_6COLOR,
            ToneCompression::Auto,
            GamutCompression::Auto,
        );
    }
}

/// Dizzy + 6-color measured palette + auto preprocessing.
/// Random-traversal family — pins the permutation walk against accidental change.
#[test]
fn dizzy_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "dizzy_spectra6_auto",
            DitherMode::Dizzy,
            &SPECTRA_7_3_6COLOR,
            ToneCompression::Auto,
            GamutCompression::Auto,
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
            ColorScheme::Mono,
            ToneCompression::Fixed(0.0),
            GamutCompression::None,
        );
    }
}
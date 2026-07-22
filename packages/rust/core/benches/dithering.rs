use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use epaper_dithering_core::{
    algorithms::{
        BURKES, FLOYD_STEINBERG, JARVIS_JUDICE_NINKE, direct_map, error_diffusion_dither,
        ordered_dither,
    },
    color_space::srgb_channel_to_linear,
    color_space_lab::{PaletteLab, WAB, match_pixel_oklab, rgb_to_oklab},
    dither, DitherConfig,
    dizzy::dizzy_dither,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::SPECTRA_7_3_6COLOR,
    palettes::ColorScheme,
    tone_map::{auto_compress_dynamic_range, compress_dynamic_range, gamut_compress},
    types::ImageBuffer,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images")
}

/// Load a fixture image as flat RGB bytes, or `None` if it is missing.
///
/// Criterion builds every benchmark group before running any of them, so a
/// panicking loader takes down the whole `cargo bench` run — including groups
/// that need no fixtures at all. Skipping with a warning keeps the rest usable
/// when the fixture set drifts.
fn try_load_fixture(filename: &str) -> Option<(Vec<u8>, usize, usize)> {
    match image::open(fixtures_dir().join(filename)) {
        Ok(img) => {
            let img = img.to_rgb8();
            let (w, h) = img.dimensions();
            Some((img.into_raw(), w as usize, h as usize))
        }
        Err(e) => {
            eprintln!("warning: skipping benchmark fixture {filename}: {e}");
            None
        }
    }
}

/// A non-trivial RGB gradient image (no external files needed).
fn synthetic_image(w: usize, h: usize) -> Vec<u8> {
    let n = w * h;
    (0..n)
        .flat_map(|i| {
            let x = i % w;
            let y = i / w;
            [
                ((x * 255) / w.max(1)) as u8,
                ((y * 255) / h.max(1)) as u8,
                ((i * 255) / n.max(1)) as u8,
            ]
        })
        .collect()
}

/// Synthetic image converted to linear [f64; 3] pixels (for preprocessing benchmarks).
fn synthetic_linear(w: usize, h: usize) -> Vec<[f64; 3]> {
    synthetic_image(w, h)
        .chunks_exact(3)
        .map(|c| {
            [
                srgb_channel_to_linear(c[0]),
                srgb_channel_to_linear(c[1]),
                srgb_channel_to_linear(c[2]),
            ]
        })
        .collect()
}

const SIZES: &[(usize, usize)] = &[(100, 100), (400, 300)];

// ── Error diffusion ───────────────────────────────────────────────────────────

fn bench_error_diffusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_diffusion");
    let palette = ColorScheme::Bwgbry.palette();

    for &(w, h) in SIZES {
        let pixels = synthetic_image(w, h);
        group.throughput(Throughput::Elements((w * h) as u64));

        for (name, kernel) in [
            ("floyd_steinberg", &FLOYD_STEINBERG),
            ("burkes", &BURKES),
            ("jjn", &JARVIS_JUDICE_NINKE),
        ] {
            group.bench_with_input(
                BenchmarkId::new(name, format!("{w}x{h}")),
                &(),
                |b, _| b.iter(|| error_diffusion_dither(&pixels, w, h, palette, kernel, false)),
            );
        }

        // Dizzy has no fixed kernel (pseudo-random traversal instead of raster
        // scan), so it doesn't fit the (name, kernel) loop above and calls its
        // own entry point directly.
        group.bench_with_input(
            BenchmarkId::new("dizzy", format!("{w}x{h}")),
            &(),
            |b, _| b.iter(|| dizzy_dither(&pixels, w, h, palette)),
        );
    }

    group.finish();
}

// ── Ordered dithering ─────────────────────────────────────────────────────────

fn bench_ordered_dither(c: &mut Criterion) {
    let mut group = c.benchmark_group("ordered_dither");
    let palette = ColorScheme::Bwgbry.palette();

    for &(w, h) in SIZES {
        let pixels = synthetic_image(w, h);
        group.throughput(Throughput::Elements((w * h) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(format!("{w}x{h}")), &(), |b, _| {
            b.iter(|| ordered_dither(&pixels, w, palette))
        });
    }

    group.finish();
}

// ── Direct map ────────────────────────────────────────────────────────────────

fn bench_direct_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("direct_map");
    let palette = ColorScheme::Bwgbry.palette();

    for &(w, h) in SIZES {
        let pixels = synthetic_image(w, h);
        group.throughput(Throughput::Elements((w * h) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(format!("{w}x{h}")), &(), |b, _| {
            b.iter(|| direct_map(&pixels, palette, palette))
        });
    }

    group.finish();
}

// ── Color matching (inner loop isolation) ────────────────────────────────────

fn bench_color_matching(c: &mut Criterion) {
    const N: usize = 10_000;
    let palette_linear: Vec<[f64; 3]> = ColorScheme::Bwgbry
        .palette()
        .colors
        .iter()
        .map(|&[r, g, b]| {
            [
                srgb_channel_to_linear(r),
                srgb_channel_to_linear(g),
                srgb_channel_to_linear(b),
            ]
        })
        .collect();
    let palette_lab = PaletteLab::from_linear_rgb(&palette_linear);

    // Pre-generate pixels so we don't measure generation time
    let pixels: Vec<_> = (0..N)
        .map(|i| {
            let v = i as f64 / N as f64;
            rgb_to_oklab(v, 1.0 - v, v * 0.5)
        })
        .collect();

    let mut group = c.benchmark_group("color_matching");
    group.throughput(Throughput::Elements(N as u64));
    group.bench_function("match_pixel_oklab_6color", |b| {
        b.iter(|| {
            pixels.iter().map(|&px| match_pixel_oklab(px, &palette_lab, WAB)).sum::<usize>()
        })
    });
    group.finish();
}

// ── Preprocessing ─────────────────────────────────────────────────────────────

fn bench_preprocessing(c: &mut Criterion) {
    let (w, h) = (400, 300);
    let palette = &SPECTRA_7_3_6COLOR;

    let mut group = c.benchmark_group("preprocessing");
    group.throughput(Throughput::Elements((w * h) as u64));

    group.bench_function("auto_compress_dynamic_range", |b| {
        b.iter_batched(
            || synthetic_linear(w, h),
            |mut pixels| auto_compress_dynamic_range(&mut pixels, palette),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("compress_dynamic_range_full", |b| {
        b.iter_batched(
            || synthetic_linear(w, h),
            |mut pixels| compress_dynamic_range(&mut pixels, palette, 1.0),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("gamut_compress_full", |b| {
        b.iter_batched(
            || synthetic_linear(w, h),
            |mut pixels| gamut_compress(&mut pixels, palette, 1.0),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Full pipeline ──────────────────────────────────────────────────────────────

fn bench_full_pipeline(c: &mut Criterion) {
    let (w, h) = (400, 300);
    let pixels = synthetic_image(w, h);
    let img = ImageBuffer::new(&pixels, w);

    let mut group = c.benchmark_group("full_pipeline");
    group.throughput(Throughput::Elements((w * h) as u64));

    group.bench_function("burkes_auto_tone_auto_gamut_6color", |b| {
        b.iter(|| {
            dither(&img, &SPECTRA_7_3_6COLOR, DitherConfig {
                mode: DitherMode::Burkes,
                tone: ToneCompression::Auto,
                gamut: GamutCompression::Auto,
                ..Default::default()
            })
        })
    });

    group.bench_function("burkes_no_preprocessing_mono", |b| {
        b.iter(|| {
            dither(&img, ColorScheme::Mono, DitherConfig {
                mode: DitherMode::Burkes,
                tone: ToneCompression::Fixed(0.0),
                gamut: GamutCompression::None,
                ..Default::default()
            })
        })
    });

    group.finish();
}

// ── Real image benchmarks ─────────────────────────────────────────────────────

/// Full pipeline on real 800×480 photographs — reflects actual usage.
fn bench_real_images(c: &mut Criterion) {
    // Images that cover different content characteristics
    const FIXTURES: &[(&str, &str)] = &[
        ("frankfurt_nacht.png", "night"),     // dark, low contrast
        ("unicorn.png",         "vivid"),     // saturated colors
        ("cat.png",             "detail"),    // fine detail, varied tones
        ("olympiapark.png",     "daylight"),  // normal outdoor scene
    ];

    let mut group = c.benchmark_group("real_images");
    // Every fixture in `tests/fixtures/images/` is 800×480; keep it that way or
    // this hardcoded throughput becomes a lie.
    group.throughput(Throughput::Elements((800 * 480) as u64));

    for (filename, label) in FIXTURES {
        let Some((pixels, w, _h)) = try_load_fixture(filename) else {
            continue;
        };
        let img = ImageBuffer::new(&pixels, w);

        group.bench_with_input(
            BenchmarkId::new("burkes_spectra6_auto", label),
            &(),
            |b, _| {
                b.iter(|| {
                    dither(
                        &img,
                        &SPECTRA_7_3_6COLOR,
                        DitherConfig {
                            mode: DitherMode::Burkes,
                            tone: ToneCompression::Auto,
                            gamut: GamutCompression::Auto,
                            ..Default::default()
                        },
                    )
                })
            },
        );
    }

    group.finish();
}

/// Full pipeline on a single larger image (1280×960) from `benchmark_only/`.
/// Shows realistic throughput for inputs bigger than a panel-sized frame.
fn bench_full_res(c: &mut Criterion) {
    let Some((pixels, w, h)) = try_load_fixture("benchmark_only/marienplatz.png") else {
        return;
    };
    let img = ImageBuffer::new(&pixels, w);

    let mut group = c.benchmark_group("full_res");
    group.throughput(Throughput::Elements((w * h) as u64));
    group.sample_size(20); // fewer samples — each iteration is slow

    group.bench_function("burkes_spectra6_auto", |b| {
        b.iter(|| {
            dither(&img, &SPECTRA_7_3_6COLOR, DitherConfig {
                mode: DitherMode::Burkes,
                tone: ToneCompression::Auto,
                gamut: GamutCompression::Auto,
                ..Default::default()
            })
        })
    });

    group.finish();
}

// ── Registration ──────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_error_diffusion,
    bench_ordered_dither,
    bench_direct_map,
    bench_color_matching,
    bench_preprocessing,
    bench_full_pipeline,
    bench_real_images,
    bench_full_res,
);
criterion_main!(benches);

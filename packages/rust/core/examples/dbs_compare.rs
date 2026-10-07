//! Visual comparison of plain error diffusion against the same output refined by DBS.
//!
//! For each fixture image (resized to fill the panel), renders every variant in measured ink
//! colours and writes PNGs plus an `index.html` for side-by-side viewing:
//!
//!   <out>/<image>/source.png, <variant>.png  full-size renders
//!   <out>/<image>/strip.png                  source | variants, side by side
//!   <out>/<image>/zoom.png                   centre crop at ZOOM× nearest-neighbour, same order
//!
//! Metrics: mean OKLab dE over 4×4 block averages (neutral — independent of the DBS eye model)
//! and DBS perceived error (the quantity DBS minimises, so DBS wins it by construction).
//!
//!   cargo run --release --example dbs_compare -- [options] [image files…]
//!
//! Options (defaults in brackets):
//!   --palette spectra6|mono   [spectra6]   --passes N        [10]
//!   --size WxH                [800x480]    --distance CM     [40]
//!   --ppi P                   [127]        --out DIR         [target/dbs_compare]
//!   --variants a,b,…          [all]        --baseline DIR    compare renders against an
//!                                                             earlier --out (% pixels differing)
//!
//! Without image arguments, every image in tests/fixtures/images is used.

use std::path::{Path, PathBuf};
use std::time::Instant;

use epaper_dithering_core::{
    color_space::srgb_channel_to_linear,
    color_space_lab::rgb_to_oklab,
    dbs::{perceived_error, DbsParams},
    dither, dither_with_canonical,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::SPECTRA_7_3_6COLOR,
    palettes::{ColorScheme, Palette},
    types::ImageBuffer,
    DitherConfig,
};
use image::{imageops, RgbImage};

const BLOCK: usize = 4;
const ZOOM: u32 = 4;
const CROP: (u32, u32) = (160, 120);

struct Args {
    measured: bool,
    width: u32,
    height: u32,
    params: DbsParams,
    out: PathBuf,
    variants: Vec<String>,
    baseline: Option<PathBuf>,
    images: Vec<PathBuf>,
}

fn parse_args() -> Args {
    let mut a = Args {
        measured: true,
        width: 800,
        height: 480,
        params: DbsParams::default(),
        out: Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/dbs_compare"),
        variants: Vec::new(),
        baseline: None,
        images: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--palette" => a.measured = match val().as_str() {
                "spectra6" => true,
                "mono" => false,
                p => panic!("unknown palette {p} (spectra6|mono)"),
            },
            "--size" => {
                let v = val();
                let (w, h) = v.split_once('x').expect("--size WxH");
                (a.width, a.height) = (w.parse().expect("width"), h.parse().expect("height"));
            }
            "--passes" => a.params.max_passes = val().parse().expect("--passes N"),
            "--distance" => a.params.viewing_distance_cm = val().parse().expect("--distance CM"),
            "--ppi" => a.params.ppi = val().parse().expect("--ppi P"),
            "--out" => a.out = val().into(),
            "--variants" => a.variants = val().split(',').map(str::to_owned).collect(),
            "--baseline" => a.baseline = Some(val().into()),
            _ if arg.starts_with("--") => panic!("unknown option {arg}"),
            _ => a.images.push(arg.into()),
        }
    }
    if a.images.is_empty() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images");
        a.images = std::fs::read_dir(&dir)
            .expect("fixtures/images")
            .filter_map(|e| Some(e.ok()?.path()))
            .filter(|p| p.is_file() && matches!(p.extension().and_then(|e| e.to_str()), Some("png" | "jpg" | "jpeg")))
            .collect();
        a.images.sort();
    }
    a
}

/// Mean OKLab dE between two flat RGB buffers, averaged over BLOCK × BLOCK tiles.
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
            total += ((la.l - lb.l).powi(2) + (la.a - lb.a).powi(2) + (la.b - lb.b).powi(2)).sqrt();
            blocks += 1;
        }
    }
    total / blocks as f64
}

fn render(indices: &[u8], palette: &Palette, w: u32, h: u32) -> RgbImage {
    let rgb = indices.iter().flat_map(|&i| palette.colors[i as usize]).collect();
    RgbImage::from_raw(w, h, rgb).expect("render buffer size")
}

/// Images side by side with a 4 px white gutter.
fn hstack(images: &[RgbImage]) -> RgbImage {
    const GAP: u32 = 4;
    let h = images.iter().map(RgbImage::height).max().unwrap_or(0);
    let w = images.iter().map(|i| i.width() + GAP).sum::<u32>() - GAP;
    let mut out = RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]));
    let mut x = 0;
    for img in images {
        imageops::replace(&mut out, img, x.into(), 0);
        x += img.width() + GAP;
    }
    out
}

fn zoom_crop(img: &RgbImage) -> RgbImage {
    let (cw, ch) = (CROP.0.min(img.width()), CROP.1.min(img.height()));
    let crop = imageops::crop_imm(img, (img.width() - cw) / 2, (img.height() - ch) / 2, cw, ch).to_image();
    imageops::resize(&crop, cw * ZOOM, ch * ZOOM, imageops::FilterType::Nearest)
}

struct Variant {
    name: &'static str,
    mode: DitherMode,
    dbs: bool,
}

const VARIANTS: [Variant; 4] = [
    Variant { name: "burkes", mode: DitherMode::Burkes, dbs: false },
    Variant { name: "burkes_dbs", mode: DitherMode::Burkes, dbs: true },
    Variant { name: "dizzy", mode: DitherMode::Dizzy, dbs: false },
    Variant { name: "dizzy_dbs", mode: DitherMode::Dizzy, dbs: true },
];

fn main() {
    let args = parse_args();
    let palette: &Palette = if args.measured { &SPECTRA_7_3_6COLOR } else { ColorScheme::Mono.palette() };
    let (w, h) = (args.width as usize, args.height as usize);
    std::fs::create_dir_all(&args.out).expect("create output dir");

    println!(
        "palette {}, {}x{}, {} cm, {} ppi, ≤{} passes → {}\n",
        if args.measured { "spectra6 (measured)" } else { "mono" },
        w, h, args.params.viewing_distance_cm, args.params.ppi, args.params.max_passes,
        args.out.display()
    );
    let variants: Vec<&Variant> = VARIANTS
        .iter()
        .filter(|v| args.variants.is_empty() || args.variants.iter().any(|n| n == v.name))
        .collect();
    assert!(!variants.is_empty(), "no variant matches {:?}", args.variants);
    // Per variant: (sum block dE, sum perceived E, total seconds, differing px, compared px).
    let mut totals = vec![(0.0, 0.0, 0.0, 0usize, 0usize); variants.len()];

    println!(
        "{:<22} {:<12} {:>10} {:>14} {:>10} {:>9}",
        "image", "variant", "block dE", "perceived E", "time", "Δpx %"
    );

    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>DBS comparison</title>\
         <style>body{font:14px system-ui;margin:16px;background:#888}img{max-width:100%;image-rendering:pixelated}\
         h2{margin:24px 0 4px}p{margin:0 0 8px}</style>",
    );
    html += &format!(
        "<h1>DBS comparison</h1><p>{}, {w}×{h}, {} cm, {} ppi, ≤{} passes. Order: source, {}.</p>",
        if args.measured { "Spectra 6 measured" } else { "Mono" },
        args.params.viewing_distance_cm, args.params.ppi, args.params.max_passes,
        variants.iter().map(|v| v.name).collect::<Vec<_>>().join(", ")
    );

    for path in &args.images {
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("image name");
        let src = image::open(path)
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()))
            .resize_to_fill(args.width, args.height, imageops::FilterType::Lanczos3)
            .to_rgb8();
        let dir = args.out.join(stem);
        std::fs::create_dir_all(&dir).expect("create image dir");
        src.save(dir.join("source.png")).expect("save source");

        let buf = ImageBuffer::new(src.as_raw(), w);
        let mut renders = vec![src.clone()];
        let mut rows = String::new();

        for (vi, v) in variants.iter().enumerate() {
            let cfg = DitherConfig {
                mode: v.mode,
                dbs: v.dbs.then_some(args.params),
                ..Default::default()
            };
            let t = Instant::now();
            let indices = if args.measured {
                let cfg = DitherConfig { tone: ToneCompression::Auto, gamut: GamutCompression::Auto, ..cfg };
                dither_with_canonical(&buf, palette, ColorScheme::Bwgbry.palette(), cfg)
            } else {
                dither(&buf, palette, cfg)
            };
            let elapsed = t.elapsed();

            let img = render(&indices, palette, args.width, args.height);
            let de = block_delta_e(src.as_raw(), img.as_raw(), w, h);
            let pe = perceived_error(src.as_raw(), w, h, palette, &indices, &args.params);
            let diff = args.baseline.as_ref().and_then(|b| {
                let base = image::open(b.join(stem).join(format!("{}.png", v.name))).ok()?.to_rgb8();
                (base.dimensions() == img.dimensions())
                    .then(|| base.pixels().zip(img.pixels()).filter(|(a, b)| a != b).count())
            });
            let t = &mut totals[vi];
            (t.0, t.1, t.2) = (t.0 + de, t.1 + pe, t.2 + elapsed.as_secs_f64());
            if let Some(d) = diff {
                (t.3, t.4) = (t.3 + d, t.4 + w * h);
            }
            let diff_col = diff.map_or("-".into(), |d| format!("{:.3}", 100.0 * d as f64 / (w * h) as f64));
            println!(
                "{stem:<22} {:<12} {de:>10.4} {pe:>14.4e} {:>9.2}s {diff_col:>9}",
                v.name,
                elapsed.as_secs_f64()
            );
            rows += &format!("<tr><td>{}<td>{de:.4}<td>{pe:.4e}<td>{:.2}s", v.name, elapsed.as_secs_f64());

            img.save(dir.join(format!("{}.png", v.name))).expect("save variant");
            renders.push(img);
        }

        hstack(&renders).save(dir.join("strip.png")).expect("save strip");
        hstack(&renders.iter().map(zoom_crop).collect::<Vec<_>>())
            .save(dir.join("zoom.png"))
            .expect("save zoom");

        html += &format!(
            "<h2>{stem}</h2><table><tr><th>variant<th>block dE<th>perceived E<th>time{rows}</table>\
             <p><img src=\"{stem}/strip.png\"></p><p><img src=\"{stem}/zoom.png\"></p>"
        );
    }

    println!("\n{:<22} {:<12} {:>10} {:>14} {:>10} {:>9}", "TOTAL", "variant", "mean dE", "mean E", "time", "Δpx %");
    let n = args.images.len() as f64;
    for (v, t) in variants.iter().zip(&totals) {
        let diff_col = if t.4 > 0 { format!("{:.3}", 100.0 * t.3 as f64 / t.4 as f64) } else { "-".into() };
        println!("{:<22} {:<12} {:>10.4} {:>14.4e} {:>9.2}s {diff_col:>9}", "", v.name, t.0 / n, t.1 / n, t.2);
    }

    std::fs::write(args.out.join("index.html"), html).expect("write index.html");
    println!("\nopen {}", args.out.join("index.html").display());
}

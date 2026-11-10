#!/usr/bin/env python3
"""Pipeline visualization: saves one image per dithering step for inspection.

Usage:
    uv run --project packages/python python tools/pipeline.py [options] [image1.jpg ...]

Options:
    --gc auto|0|0.5|1.0     Gamut compression (default: 0)
    --palette spectra_7_3_6color_v2|spectra_7_3_6color  (default: spectra_7_3_6color_v2)
    --mode BURKES|FLOYD_STEINBERG|ATKINSON|...      (default: BURKES)

Output: pipeline_out/pipeline_<name>_gc<gc>.png — a vertical strip:
    1. Input (sRGB)
    2. After sRGB→linear   + luminance histogram
    3. After gamut compression | OKLab movement heatmap
    4. Direct palette map (no error diffusion; includes anchoring)
    5. Final dithered output (includes anchoring)

Defaults to marienplatz.jpg if no image paths given.
"""

from __future__ import annotations

import argparse
from pathlib import Path


import epaper_dithering._rs as _rs
import example_palettes
import numpy as np
from epaper_dithering import DitherMode, DitherOptions, Palette, dither_image
from PIL import Image, ImageDraw, ImageFont

# ── Color-space helpers (self-contained; the Rust core owns the canonical math) ──


def srgb_to_linear(a: np.ndarray) -> np.ndarray:
    """sRGB [0, 255] → linear [0, 1] (IEC 61966-2-1)."""
    x = a.astype(np.float64) / 255.0
    return np.where(x <= 0.04045, x / 12.92, ((x + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(a: np.ndarray) -> np.ndarray:
    """Linear [0, 1] → sRGB uint8 [0, 255]."""
    x = np.clip(a, 0.0, 1.0)
    s = np.where(x <= 0.0031308, x * 12.92, 1.055 * x ** (1.0 / 2.4) - 0.055)
    return np.clip(s * 255.0, 0, 255).astype(np.uint8)


def get_palette_colors(palette: Palette) -> list[tuple[int, int, int]]:
    """Preview sRGB colours of a palette, in entry order."""
    return palette.preview_rgb()


def rgb_to_lab(px: np.ndarray) -> np.ndarray:
    """Linear-RGB (H, W, 3) → OKLab (H, W, 3) via the Rust core."""
    flat = np.ascontiguousarray(px, dtype=np.float64).reshape(-1).tolist()
    out = _rs.rgb_to_oklab_buffer(flat)
    return np.array(out, dtype=np.float64).reshape(px.shape)


# ── Layout constants ──────────────────────────────────────────────────────────
WIDTH, HEIGHT = 800, 480
LABEL_H = 20
HIST_H = 80

# ── Luminance weights (ITU-R BT.709) ─────────────────────────────────────────
_WR, _WG, _WB = 0.2126729, 0.7151522, 0.0721750

# ── Available palettes ────────────────────────────────────────────────────────
PALETTES: dict[str, Palette] = {name.lower(): p for name, p in example_palettes.measured()}

# ── Available dither modes ────────────────────────────────────────────────────
MODES: dict[str, DitherMode] = {m.name: m for m in DitherMode}


# ── Helpers ───────────────────────────────────────────────────────────────────


def parse_compression(val: str) -> float | str:
    """Parse 'auto', '0', '0.5', '1.0' → "auto" or float."""
    if val.lower() == "auto":
        return "auto"
    return float(val)


def fmt(val: float | str) -> str:
    return val if isinstance(val, str) else f"{val:.2g}"


def load_font(size: int = 13) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    for name in ("DejaVuSans.ttf", "Arial.ttf", "FreeSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def lum(px: np.ndarray) -> np.ndarray:
    return _WR * px[:, :, 0] + _WG * px[:, :, 1] + _WB * px[:, :, 2]


def to_pil(pixels_linear: np.ndarray) -> Image.Image:
    srgb = linear_to_srgb(np.clip(pixels_linear, 0.0, 1.0).astype(np.float32))
    return Image.fromarray(srgb, "RGB")


def add_label(img: Image.Image, text: str, font: ImageFont.ImageFont) -> Image.Image:
    out = Image.new("RGB", (img.width, img.height + LABEL_H), (20, 20, 20))
    out.paste(img, (0, LABEL_H))
    ImageDraw.Draw(out).text((6, 4), text, fill=(220, 220, 220), font=font)
    return out


def with_histogram(
    img: Image.Image,
    Y: np.ndarray,
    black_Y: float,
    white_Y: float,
    font: ImageFont.ImageFont,
) -> Image.Image:
    w = img.width
    strip = Image.new("RGB", (w, HIST_H), (30, 30, 30))
    draw = ImageDraw.Draw(strip)

    hist, _ = np.histogram(Y.ravel(), bins=256, range=(0.0, 1.0))
    max_count = max(int(hist.max()), 1)
    bar_w = max(1, w // 256)
    for i, count in enumerate(hist):
        x = int(i * w / 256)
        bh = int((count / max_count) * (HIST_H - 4))
        draw.rectangle([x, HIST_H - bh - 2, x + bar_w - 1, HIST_H - 2], fill=(130, 130, 130))

    def vline(val: float, color: tuple[int, int, int], label: str) -> None:
        x = int(val * w)
        draw.line([(x, 0), (x, HIST_H)], fill=color, width=2)
        draw.text((x + 3, 2), label, fill=color, font=font)

    p2 = float(np.percentile(Y, 2))
    p98 = float(np.percentile(Y, 98))
    vline(black_Y, (80, 80, 255), f"disp_black {black_Y:.3f}")
    vline(white_Y, (255, 80, 80), f"disp_white {white_Y:.3f}")
    vline(p2, (0, 220, 220), f"p2={p2:.3f}")
    vline(p98, (255, 160, 0), f"p98={p98:.3f}")

    combined = Image.new("RGB", (w, img.height + HIST_H))
    combined.paste(img, (0, 0))
    combined.paste(strip, (0, img.height))
    return combined


def gamut_heatmap(before: np.ndarray, after: np.ndarray) -> Image.Image:
    dist = np.sqrt(np.sum((rgb_to_lab(after) - rgb_to_lab(before)) ** 2, axis=-1))
    scaled = np.clip(dist / 0.3 * 255, 0, 255).astype(np.uint8)
    rgb = np.zeros((*scaled.shape, 3), dtype=np.uint8)
    rgb[:, :, 0] = scaled
    rgb[:, :, 1] = (scaled * 0.25).astype(np.uint8)
    return Image.fromarray(rgb, "RGB")


def _run_step(fn, pixels_linear: np.ndarray, palette: Palette, strength: float | None) -> np.ndarray:
    flat = np.ascontiguousarray(pixels_linear, dtype=np.float64).reshape(-1).tolist()
    out = fn(flat, palette._inner, strength)  # pylint: disable=protected-access
    return np.array(out, dtype=np.float64).reshape(pixels_linear.shape)


def apply_gc(pixels_linear: np.ndarray, palette: Palette, gc: float | str) -> tuple[np.ndarray, str]:
    if gc == 0.0:
        return pixels_linear, "disabled"
    # gamut_compress: None → full strength (auto), else fixed strength.
    strength = None if gc == "auto" else float(gc)
    result = _run_step(_rs.gamut_compress, pixels_linear, palette, strength)
    return result, "auto" if gc == "auto" else f"strength={gc:.2g}"


# ── Main per-image function ───────────────────────────────────────────────────


def run(
    image_path: Path,
    palette: Palette,
    palette_name: str,
    gc: float | str,
    mode: DitherMode,
    out_path: Path,
) -> None:
    src = Image.open(image_path).convert("RGB").resize((WIDTH, HEIGHT), Image.LANCZOS)
    font = load_font(13)

    palette_srgb = get_palette_colors(palette)
    palette_linear = srgb_to_linear(np.array(palette_srgb, dtype=np.float64))
    black_Y = float(_WR * palette_linear[0, 0] + _WG * palette_linear[0, 1] + _WB * palette_linear[0, 2])
    white_Y = float(_WR * palette_linear[1, 0] + _WG * palette_linear[1, 1] + _WB * palette_linear[1, 2])

    # ── Step 1: Input ──────────────────────────────────────────────────────────
    panel1 = add_label(src.copy(), f"1. Input (sRGB)  palette={palette_name}", font)

    # ── Step 2: sRGB→linear ───────────────────────────────────────────────────
    pixels_srgb = np.array(src, dtype=np.uint8)
    pixels_linear = srgb_to_linear(pixels_srgb.astype(np.float32))
    Y2 = lum(pixels_linear)
    p2_2, p98_2 = float(np.percentile(Y2, 2)), float(np.percentile(Y2, 98))
    panel2 = with_histogram(to_pil(pixels_linear), Y2, black_Y, white_Y, font)
    panel2 = add_label(
        panel2,
        f"2. After sRGB→linear  (p2={p2_2:.3f}  p98={p98_2:.3f}  |  display: [{black_Y:.3f}, {white_Y:.3f}])",
        font,
    )

    # ── Step 3: Gamut compression ─────────────────────────────────────────────
    pixels_gc, gc_note = apply_gc(pixels_linear, palette, gc)
    n_moved = int(np.sum(np.any(np.abs(pixels_gc - pixels_linear) > 1e-5, axis=-1)))
    pct_moved = 100.0 * n_moved / (WIDTH * HEIGHT)

    heatmap = gamut_heatmap(pixels_linear, pixels_gc)
    half_w = WIDTH // 2
    side = Image.new("RGB", (WIDTH, HEIGHT))
    side.paste(to_pil(pixels_gc).resize((half_w, HEIGHT), Image.LANCZOS), (0, 0))
    side.paste(heatmap.resize((half_w, HEIGHT), Image.LANCZOS), (half_w, 0))
    d = ImageDraw.Draw(side)
    d.text((4, 4), "result", fill=(200, 200, 200), font=font)
    d.text((half_w + 4, 4), f"heatmap: {pct_moved:.1f}% pixels moved", fill=(255, 130, 80), font=font)
    panel4 = add_label(
        side,
        f"3. Gamut compression [gc={fmt(gc)} — {gc_note}]  {n_moved:,} px / {pct_moved:.1f}% affected  |  heatmap",
        font,
    )

    # ── Step 5: Direct palette map ────────────────────────────────────────────
    direct = dither_image(src, palette, DitherOptions(mode=DitherMode.NONE, gamut=gc))
    panel5 = add_label(
        direct.convert("RGB"), f"4. Direct palette map (no error diffusion)  gc={fmt(gc)}", font
    )

    # ── Step 6: Final output ──────────────────────────────────────────────────
    final = dither_image(src, palette, DitherOptions(mode=mode, gamut=gc))
    panel6 = add_label(final.convert("RGB"), f"5. Final: {mode.name}  gc={fmt(gc)}", font)

    # ── Assemble vertical strip ───────────────────────────────────────────────
    panels = [panel1, panel2, panel4, panel5, panel6]
    total_h = sum(p.height for p in panels)
    sheet = Image.new("RGB", (WIDTH, total_h), (10, 10, 10))
    y = 0
    for p in panels:
        sheet.paste(p, (0, y))
        y += p.height

    sheet.save(out_path)
    print(f"  → {out_path}  ({sheet.width}×{sheet.height})")


def main() -> None:
    parser = argparse.ArgumentParser(description="Pipeline visualization for epaper dithering.")
    parser.add_argument("images", nargs="*", help="Image paths (default: marienplatz.jpg)")
    parser.add_argument("--gc", default="0", help="Gamut compression: auto|0|0.5|1.0 (default: 0)")
    parser.add_argument(
        "--palette", default="spectra_7_3_6color_v2", choices=list(PALETTES), help="Example palette name"
    )
    parser.add_argument("--mode", default="BURKES", choices=list(MODES), help="Dither mode (default: BURKES)")
    args = parser.parse_args()

    gc = parse_compression(args.gc)
    palette = PALETTES[args.palette]
    mode = MODES[args.mode]

    here = example_palettes.ROOT / "packages/python"
    image_paths = [here / p for p in args.images] if args.images else [here / "marienplatz.jpg"]
    out_dir = here / "pipeline_out"
    out_dir.mkdir(parents=True, exist_ok=True)

    suffix = f"_gc{fmt(gc)}_{args.palette}"
    for img_path in image_paths:
        if not img_path.exists():
            print(f"  skip (not found): {img_path}")
            continue
        out_path = out_dir / f"pipeline_{img_path.stem}{suffix}.png"
        print(f"Processing {img_path.name}  (gc={fmt(gc)}  palette={args.palette}  mode={args.mode})...")
        run(img_path, palette, args.palette, gc, mode, out_path)


if __name__ == "__main__":
    main()

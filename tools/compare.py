#!/usr/bin/env python3
"""Visual dithering comparison — produces three focused contact sheets.

    schemes.png              All idealised palettes, Burkes
    algorithms_{NAME}.png    All dither modes for every idealised and measured palette
    gamut_compression.png    Measured palettes: gc=0 | gc=0.5 | gc=1.0

Usage (from the repo root):
    uv run --project packages/python python tools/compare.py [image] [--width W] [--height H] [--out DIR]

Without an image, the `ubahn_station.png` test fixture is used. The README images are made by
`generate_docs_examples.py`, not by this script.
"""

from __future__ import annotations

import argparse
import time
from pathlib import Path

import example_palettes
from epaper_dithering import DitherMode, DitherOptions, Palette, dither_image
from PIL import Image, ImageDraw, ImageFont

DISPLAY_WIDTH = 800
DISPLAY_HEIGHT = 480
LABEL_H = 32
FONT_SIZE = 18
DEFAULT_IMAGE = example_palettes.ROOT / "packages/rust/core/tests/fixtures/images/ubahn_station.png"

# Discovered automatically from the library — no manual updates needed
ALL_ALGORITHMS: list[DitherMode] = list(DitherMode)

COLOR_SCHEMES: list[tuple[str, Palette]] = example_palettes.ideal()

MEASURED_PALETTES: list[tuple[str, Palette]] = example_palettes.measured()

ALL_PALETTES_FOR_ALGO = COLOR_SCHEMES + MEASURED_PALETTES  # type: ignore[assignment]


def load_font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    for name in ("DejaVuSans.ttf", "Arial.ttf", "FreeSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def gc_str(gc: float) -> str:
    if gc == 1.0 or gc == 1:
        return "gc=100%"
    if gc == 0 or gc == 0.0:
        return "gc=0"
    return f"gc={gc}"


def render(
    src: Image.Image, scheme: Palette, mode: DitherMode, gc: float = 0.0
) -> tuple[Image.Image, float]:
    t0 = time.perf_counter()
    dithered = dither_image(src, scheme, DitherOptions(mode=mode, gamut=gc))
    return dithered.convert("RGB"), time.perf_counter() - t0


def make_sheet(cells: list[tuple[str, Image.Image]], cols: int, iw: int, ih: int) -> Image.Image:
    """Arrange (label, image) cells into a labeled grid."""
    font = load_font(FONT_SIZE)
    rows = (len(cells) + cols - 1) // cols
    sheet = Image.new("RGB", (iw * cols, (ih + LABEL_H) * rows), (40, 40, 40))
    draw = ImageDraw.Draw(sheet)
    for i, (label, img) in enumerate(cells):
        col, row = i % cols, i // cols
        x, y = col * iw, row * (ih + LABEL_H)
        sheet.paste(img, (x, y))
        draw.text((x + 4, y + ih + 4), label, fill=(220, 220, 220), font=font)
    return sheet


def save(sheet: Image.Image, path: Path) -> None:
    sheet.save(path)
    print(f"  → {path}")


def run(image_path: Path, out_dir: Path, width: int, height: int, gamut_compression: float = 0.0) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    src = Image.open(image_path).convert("RGB").resize((width, height), Image.LANCZOS)
    iw, ih = width, height

    gc_label = f" · gc={gamut_compression}" if gamut_compression != 0.0 else ""
    print(f"Input:  {image_path}  ({width}×{height}){gc_label}\n")

    # ------------------------------------------------------------------
    # 1. schemes.png — all idealised palettes, Burkes
    # ------------------------------------------------------------------
    print("── schemes ──────────────────────────────────")
    scheme_cells: list[tuple[str, Image.Image]] = []
    for label, scheme in COLOR_SCHEMES:
        img, t = render(src, scheme, DitherMode.BURKES, gamut_compression)
        scheme_cells.append((f"{label} · Burkes{gc_label}", img))
        print(f"  {label:<16} {t * 1000:>6.0f}ms")
    save(make_sheet(scheme_cells, 4, iw, ih), out_dir / "schemes.png")
    print()

    # ------------------------------------------------------------------
    # 2. algorithms_{NAME}.png — all dither modes for every palette
    # ------------------------------------------------------------------
    print("── algorithms ───────────────────────────────")
    for palette_label, palette in ALL_PALETTES_FOR_ALGO:
        cells: list[tuple[str, Image.Image]] = []
        for mode in ALL_ALGORITHMS:
            img, t = render(src, palette, mode, gamut_compression)
            cells.append((f"{mode.name} · {palette_label}{gc_label}", img))
            print(f"  {palette_label:<16} {mode.name:<22} {t * 1000:>6.0f}ms")
        save(make_sheet(cells, 3, iw, ih), out_dir / f"algorithms_{palette_label}.png")
    print()

    # ------------------------------------------------------------------
    # 3. gamut_compression.png — measured palettes, gc=0 | gc=0.5 | gc=1.0
    # ------------------------------------------------------------------
    print("── gamut_compression ────────────────────────")
    gc_cells: list[tuple[str, Image.Image]] = []
    for pal_label, palette in MEASURED_PALETTES:
        for gc_val in [0.0, 0.5, 1.0]:
            img, t = render(src, palette, DitherMode.BURKES, gc_val)
            label = f"{pal_label} · Burkes · {gc_str(gc_val)}"
            gc_cells.append((label, img))
            print(f"  {pal_label:<14} {gc_str(gc_val):<10} {t * 1000:>6.0f}ms")
    save(make_sheet(gc_cells, 3, iw, ih), out_dir / "gamut_compression.png")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("image", nargs="?", default=str(DEFAULT_IMAGE))
    parser.add_argument("--width", type=int, default=DISPLAY_WIDTH)
    parser.add_argument("--height", type=int, default=DISPLAY_HEIGHT)
    parser.add_argument("--out", default=str(example_palettes.ROOT / "packages/python/compare_out"))
    parser.add_argument(
        "--gamut-compression",
        type=float,
        default=0.0,
        metavar="GC",
        help="Gamut compression strength 0.0–1.0 for the schemes and algorithms sheets (default: 0).",
    )
    args = parser.parse_args()
    run(Path(args.image), Path(args.out), args.width, args.height, args.gamut_compression)


if __name__ == "__main__":
    main()

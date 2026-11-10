#!/usr/bin/env python3
"""Visual dithering comparison — produces three focused contact sheets.

    schemes.png              All color schemes, Burkes, tc=0
    algorithms_{NAME}.png    All dither modes for every color scheme and measured palette
    tone_compression.png     Measured palettes: tc=0 | tc=auto | tc=1.0
    gamut_compression.png    Measured palettes: gc=0 | gc=auto | gc=1.0

Add --docs to also generate images for the README in docs/images/:
    Full resolution in docs/images/
    50% thumbnails in docs/images/thumbs/

Usage:
    uv run scripts/compare.py [image] [--width W] [--height H] [--out DIR] [--docs]

Examples:
    uv run scripts/compare.py
    uv run scripts/compare.py photo.jpg --width 400 --height 300
    uv run scripts/compare.py photo.jpg --docs
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
DOCS_SCALE = 0.5

# Discovered automatically from the library — no manual updates needed
ALL_ALGORITHMS: list[DitherMode] = list(DitherMode)

COLOR_SCHEMES: list[tuple[str, Palette]] = example_palettes.ideal()

MEASURED_PALETTES: list[tuple[str, Palette]] = example_palettes.measured()

ALL_PALETTES_FOR_ALGO = COLOR_SCHEMES + MEASURED_PALETTES  # type: ignore[assignment]
COLOR_SCHEME_NAMES = {s for s, _ in COLOR_SCHEMES}


def load_font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    for name in ("DejaVuSans.ttf", "Arial.ttf", "FreeSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def tc_str(tc: float | str) -> str:
    if tc == "auto":
        return "tc=auto"
    if tc == 1.0 or tc == 1:
        return "tc=100%"
    return f"tc={tc}"


def gc_str(gc: float | str) -> str:
    if gc == "auto":
        return "gc=auto"
    if gc == 1.0 or gc == 1:
        return "gc=100%"
    if gc == 0 or gc == 0.0:
        return "gc=0"
    return f"gc={gc}"


def render(
    src: Image.Image, scheme: Palette, mode: DitherMode, tc: float | str, gc: float | str = 0.0
) -> tuple[Image.Image, float]:
    t0 = time.perf_counter()
    dithered = dither_image(src, scheme, DitherOptions(mode=mode, tone=tc, gamut=gc))
    return dithered.convert("RGB"), time.perf_counter() - t0


def make_sheet(
    cells: list[tuple[str, Image.Image]],
    cols: int,
    iw: int,
    ih: int,
    scale: float = 1.0,
) -> Image.Image:
    """Arrange (label, image) cells into a labeled grid, optionally scaled."""
    tw = int(iw * scale)
    th = int(ih * scale)
    label_h = max(1, int(LABEL_H * scale))
    font = load_font(max(8, int(FONT_SIZE * scale)))

    rows = (len(cells) + cols - 1) // cols
    sheet = Image.new("RGB", (tw * cols, (th + label_h) * rows), (40, 40, 40))
    draw = ImageDraw.Draw(sheet)
    for i, (label, img) in enumerate(cells):
        col, row = i % cols, i // cols
        x, y = col * tw, row * (th + label_h)
        sheet.paste(img.resize((tw, th), Image.LANCZOS) if scale != 1.0 else img, (x, y))
        draw.text((x + 4, y + th + 4), label, fill=(220, 220, 220), font=font)
    return sheet


def save(sheet: Image.Image, path: Path, also_thumb: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(path)
    print(f"  → {path}")
    if also_thumb:
        thumb_path = path.parent / "thumbs" / path.name
        thumb_path.parent.mkdir(parents=True, exist_ok=True)
        w, h = sheet.size
        sheet.resize((int(w * DOCS_SCALE), int(h * DOCS_SCALE)), Image.LANCZOS).save(thumb_path)
        print(f"  → {thumb_path}")


def run(
    image_path: Path,
    out_dir: Path,
    width: int,
    height: int,
    docs: bool,
    docs_algo: str,
    docs_tc: str,
    docs_gc: str,
    gamut_compression: float | str = 0.0,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    src = Image.open(image_path).convert("RGB").resize((width, height), Image.LANCZOS)
    iw, ih = width, height

    gc_label = f" · gc={gamut_compression}" if gamut_compression != 0.0 else ""
    print(f"Input:  {image_path}  ({width}×{height}){gc_label}\n")

    # ------------------------------------------------------------------
    # 1. schemes.png — all color schemes, Burkes, tc=0
    # ------------------------------------------------------------------
    print("── schemes ──────────────────────────────────")
    scheme_cells: list[tuple[str, Image.Image]] = []
    for label, scheme in COLOR_SCHEMES:
        img, t = render(src, scheme, DitherMode.BURKES, 0.0, gamut_compression)
        scheme_cells.append((f"{label} · Burkes · tc=0{gc_label}", img))
        print(f"  {label:<16} {t * 1000:>6.0f}ms")
    save(make_sheet(scheme_cells, 4, iw, ih), out_dir / "schemes.png")
    print()

    # ------------------------------------------------------------------
    # 2. algorithms_{NAME}.png — all dither modes for every palette
    # ------------------------------------------------------------------
    print("── algorithms ───────────────────────────────")
    algo_cells_by_name: dict[str, list[tuple[str, Image.Image]]] = {}
    for palette_label, palette in ALL_PALETTES_FOR_ALGO:
        tc: float | str = 0.0 if palette_label in COLOR_SCHEME_NAMES else "auto"
        cells: list[tuple[str, Image.Image]] = []
        for mode in ALL_ALGORITHMS:
            img, t = render(src, palette, mode, tc, gamut_compression)
            cells.append((f"{mode.name} · {palette_label} · {tc_str(tc)}{gc_label}", img))
            print(f"  {palette_label:<16} {mode.name:<22} {t * 1000:>6.0f}ms")
        algo_cells_by_name[palette_label] = cells
        save(make_sheet(cells, 3, iw, ih), out_dir / f"algorithms_{palette_label}.png")
    print()

    # ------------------------------------------------------------------
    # 3. tone_compression.png — measured palettes, tc=0 | tc=auto | tc=1.0
    # ------------------------------------------------------------------
    print("── tone_compression ─────────────────────────")
    tc_cells: list[tuple[str, Image.Image]] = []
    tc_cells_solum: list[tuple[str, Image.Image]] = []
    for pal_label, palette in MEASURED_PALETTES:
        for tc_val in [0.0, "auto", 1.0]:
            img, t = render(src, palette, DitherMode.BURKES, tc_val, gamut_compression)
            label = f"{pal_label} · Burkes · {tc_str(tc_val)}{gc_label}"
            tc_cells.append((label, img))
            if pal_label == "SPECTRA_7_3_6COLOR":
                tc_cells_solum.append((label, img))
            print(f"  {pal_label:<14} {tc_str(tc_val):<10} {t * 1000:>6.0f}ms")
    save(make_sheet(tc_cells, 3, iw, ih), out_dir / "tone_compression.png")
    print()

    # ------------------------------------------------------------------
    # 4. gamut_compression.png — measured palettes, gc=0 | gc=auto | gc=1.0
    # ------------------------------------------------------------------
    print("── gamut_compression ────────────────────────")
    gc_cells: list[tuple[str, Image.Image]] = []
    for pal_label, palette in MEASURED_PALETTES:
        for gc_val in [0.0, "auto", 1.0]:
            img, t = render(src, palette, DitherMode.BURKES, "auto", gc_val)
            label = f"{pal_label} · Burkes · {gc_str(gc_val)}"
            gc_cells.append((label, img))
            print(f"  {pal_label:<14} {gc_str(gc_val):<10} {t * 1000:>6.0f}ms")
    save(make_sheet(gc_cells, 3, iw, ih), out_dir / "gamut_compression.png")
    print()

    # ------------------------------------------------------------------
    # --docs: full-res + 50% thumbnails for the README
    # ------------------------------------------------------------------
    if docs:
        docs_dir = example_palettes.ROOT / "packages/python/docs/images"
        print(f"── docs → {docs_dir}")

        # algorithms: use the requested palette, fall back to first available
        algo_palette_names = list(algo_cells_by_name.keys())
        algo_key = (
            docs_algo
            if docs_algo in algo_cells_by_name
            else next((n for n in ["BWR", "MONO"] if n in algo_cells_by_name), algo_palette_names[0])
        )
        if algo_key != docs_algo:
            print(f"  (note: --docs-algo-palette '{docs_algo}' not found, using '{algo_key}')")

        # tone compression: use the requested palette, fall back to first measured
        measured_palette_names = [name for name, _ in MEASURED_PALETTES]
        tc_key = docs_tc if docs_tc in measured_palette_names else measured_palette_names[0]
        if tc_key != docs_tc:
            print(f"  (note: --docs-tc-palette '{docs_tc}' not found, using '{tc_key}')")
        tc_cells_docs = [cell for cell in tc_cells if cell[0].startswith(tc_key)]

        # gamut compression: use the requested palette, fall back to first measured
        gc_key = docs_gc if docs_gc in measured_palette_names else measured_palette_names[0]
        if gc_key != docs_gc:
            print(f"  (note: --docs-gc-palette '{docs_gc}' not found, using '{gc_key}')")
        gc_cells_docs = [cell for cell in gc_cells if cell[0].startswith(gc_key)]

        save(make_sheet(scheme_cells, 4, iw, ih), docs_dir / "schemes.png", also_thumb=True)
        save(make_sheet(algo_cells_by_name[algo_key], 3, iw, ih), docs_dir / "algorithms.png", also_thumb=True)
        print(f"     (palette: {algo_key})")
        save(make_sheet(tc_cells_docs, 3, iw, ih), docs_dir / "tone_compression.png", also_thumb=True)
        print(f"     (palette: {tc_key})")
        save(make_sheet(gc_cells_docs, 3, iw, ih), docs_dir / "gamut_compression.png", also_thumb=True)
        print(f"     (palette: {gc_key})")


def main() -> None:
    here = example_palettes.ROOT / "packages/python"
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("image", nargs="?", default=str(here / "marienplatz.jpg"))
    parser.add_argument("--width", type=int, default=DISPLAY_WIDTH)
    parser.add_argument("--height", type=int, default=DISPLAY_HEIGHT)
    parser.add_argument("--out", default=str(here / "compare_out"))
    parser.add_argument("--docs", action="store_true", help="Also write images to docs/images/ for the README")
    parser.add_argument(
        "--docs-algo-palette", default="BWR", metavar="NAME", help="Palette for docs algorithms image (default: BWR)"
    )
    parser.add_argument(
        "--docs-tc-palette",
        default="SPECTRA_7_3_6COLOR",
        metavar="NAME",
        help="Palette for docs tone_compression image (default: SPECTRA_7_3_6COLOR)",
    )
    parser.add_argument(
        "--docs-gc-palette",
        default="SPECTRA_7_3_6COLOR",
        metavar="NAME",
        help="Palette for docs gamut_compression image (default: SPECTRA_7_3_6COLOR)",
    )
    parser.add_argument(
        "--gamut-compression",
        default="auto",
        metavar="GC",
        help="Gamut compression: 'auto' (default), or strength 0.0–1.0.",
    )
    args = parser.parse_args()
    gc: float | str = "auto" if args.gamut_compression == "auto" else float(args.gamut_compression)
    run(
        Path(args.image),
        Path(args.out),
        args.width,
        args.height,
        args.docs,
        args.docs_algo_palette,
        args.docs_tc_palette,
        args.docs_gc_palette,
        gc,
    )


if __name__ == "__main__":
    main()

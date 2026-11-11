#!/usr/bin/env python3
"""Generate color calibration patch images for e-paper displays.

Usage:
    uv run --project packages/python python tools/generate_patches.py --palette ideal_bwgbry --size 800x480
    uv run --project packages/python python tools/generate_patches.py --list

Patches show each entry's nominal colour (what you send to the panel); unnamed entries show
their preview colour.
"""

from __future__ import annotations

import argparse
import math
import sys

from epaper_dithering import Palette
from example_palettes import EXAMPLES, load
from PIL import Image, ImageDraw, ImageFont


def generate_patches(palette: Palette, width: int, height: int) -> Image.Image:
    colors = [
        (e.name or f"#{i}", e.nominal or palette.preview_rgb()[i]) for i, e in enumerate(palette.entries)
    ]
    n = len(colors)

    # Find grid that makes patches closest to square
    best_cols, best_rows = n, 1
    best_ratio = float("inf")
    for rows in range(1, n + 1):
        cols = math.ceil(n / rows)
        pw, ph = width / cols, height / rows
        ratio = max(pw, ph) / max(min(pw, ph), 1)
        if ratio < best_ratio:
            best_ratio = ratio
            best_cols, best_rows = cols, rows

    cell_w = width // best_cols
    cell_h = height // best_rows
    font = ImageFont.load_default(max(14, min(cell_w, cell_h) // 6))

    img = Image.new("RGB", (width, height), (0, 0, 0))
    draw = ImageDraw.Draw(img)

    for i, (name, rgb) in enumerate(colors):
        x = (i % best_cols) * cell_w
        y = (i // best_cols) * cell_h
        draw.rectangle([x, y, x + cell_w, y + cell_h], fill=rgb)

        bright = (0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]) > 128
        draw.text(
            (x + cell_w // 2, y + cell_h - font.size),
            name.upper(),
            fill=(0, 0, 0) if bright else (255, 255, 255),
            font=font,
            anchor="mm",
        )

    return img


def main() -> None:
    schemes = sorted(p.stem for p in EXAMPLES.glob("*.json"))
    parser = argparse.ArgumentParser(description="Generate calibration patches for e-paper displays.")
    parser.add_argument("--palette", choices=schemes, help="Example palette name")
    parser.add_argument("--size", help="Display size as WxH (e.g. 800x480)")
    parser.add_argument("-o", "--output", default="calibration_patches.png", help="Output filename")
    parser.add_argument("--list", action="store_true", help="List available example palettes")
    args = parser.parse_args()

    if args.list:
        for name in schemes:
            p = load(name)
            inks = ", ".join(e.name or "-" for e in p.entries)
            print(f"  {name:24s} ({len(p)} colors): {inks}")
        sys.exit(0)

    if not args.palette or not args.size:
        parser.error("--palette and --size are required (use --list to see palettes)")

    try:
        width, height = (int(x) for x in args.size.lower().split("x"))
    except ValueError:
        parser.error(f"Invalid size: {args.size!r} (expected WxH)")

    img = generate_patches(load(args.palette), width, height)
    img.save(args.output)
    print(f"Saved {args.output} ({width}x{height})")


if __name__ == "__main__":
    main()

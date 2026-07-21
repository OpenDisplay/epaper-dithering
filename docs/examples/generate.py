"""Generate example images for README documentation."""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
from epaper_dithering import dither_image, ColorScheme, DitherMode, SPECTRA_7_3_6COLOR_V2

FIXTURES = Path(__file__).parent.parent.parent / "packages/rust/core/tests/fixtures/images"
OUT = Path(__file__).parent

def load(name: str) -> Image.Image:
    return Image.open(FIXTURES / name).convert("RGB")


def dithered_rgb(img: Image.Image, palette, mode: DitherMode) -> Image.Image:
    return dither_image(img, palette, mode=mode).convert("RGB")


def label(img: Image.Image, text: str, size: int = 20) -> Image.Image:
    """Add a label bar at the bottom of an image."""
    bar_h = 32
    out = Image.new("RGB", (img.width, img.height + bar_h), (30, 30, 30))
    out.paste(img, (0, 0))
    draw = ImageDraw.Draw(out)
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", size)
    except Exception:
        font = ImageFont.load_default()
    draw.text((img.width // 2, img.height + bar_h // 2), text, fill=(220, 220, 220), font=font, anchor="mm")
    return out


def grid(cells: list[list[Image.Image]]) -> Image.Image:
    rows = len(cells)
    cols = max(len(r) for r in cells)
    w = cells[0][0].width
    h = cells[0][0].height
    out = Image.new("RGB", (cols * w, rows * h), (15, 15, 15))
    for r, row in enumerate(cells):
        for c, cell in enumerate(row):
            out.paste(cell, (c * w, r * h))
    return out


# ── 1. Frankfurt night before/after ───────────────────────────────────────────

print("Generating frankfurt night before/after...")
frankfurt_orig = Image.open(FIXTURES / "frankfurt_nacht.png").convert("RGB")
frankfurt_no_pre = dither_image(frankfurt_orig, SPECTRA_7_3_6COLOR_V2,
                                mode=DitherMode.BURKES, tone=0.0, gamut=0.0).convert("RGB")
frankfurt_auto   = dither_image(frankfurt_orig, SPECTRA_7_3_6COLOR_V2,
                                mode=DitherMode.BURKES).convert("RGB")

p1 = label(frankfurt_orig,   "Original")
p2 = label(frankfurt_no_pre, "Spectra 6-color · Burkes · no preprocessing")
p3 = label(frankfurt_auto,   "Spectra 6-color · Burkes · auto tone + gamut")

ba = Image.new("RGB", (p1.width + p2.width + p3.width + 8, p1.height), (15, 15, 15))
ba.paste(p1, (0, 0))
ba.paste(p2, (p1.width + 4, 0))
ba.paste(p3, (p1.width + p2.width + 8, 0))
ba.save(OUT / "frankfurt_before_after.png")

# ── 2. All algorithms grid ────────────────────────────────────────────────────

print("Generating algorithms grid...")
src = load("ubahn_station.png")

ALGOS = [
    (DitherMode.NONE,               "None (direct map)"),
    (DitherMode.ORDERED,            "Ordered (Bayer 4×4)"),
    (DitherMode.FLOYD_STEINBERG,    "Floyd-Steinberg"),
    (DitherMode.ATKINSON,           "Atkinson"),
    (DitherMode.BURKES,             "Burkes"),
    (DitherMode.SIERRA_LITE,        "Sierra Lite"),
    (DitherMode.SIERRA,             "Sierra"),
    (DitherMode.STUCKI,             "Stucki"),
    (DitherMode.JARVIS_JUDICE_NINKE,"Jarvis-Judice-Ninke"),
]

algo_cells = []
for i in range(0, len(ALGOS), 3):
    row = []
    for mode, name in ALGOS[i:i+3]:
        cell = dithered_rgb(src, SPECTRA_7_3_6COLOR_V2, mode)
        row.append(label(cell, name))
    algo_cells.append(row)

grid(algo_cells).save(OUT / "algorithms_grid.png")

# ── 3. All color schemes grid ─────────────────────────────────────────────────

print("Generating color schemes grid...")
src2 = load("river.png")

SCHEMES = [
    (ColorScheme.MONO,       "Mono"),
    (ColorScheme.BWR,        "BWR"),
    (ColorScheme.BWY,        "BWY"),
    (ColorScheme.BWRY,       "BWRY"),
    (ColorScheme.BWGBRY,     "BWGBRY (Spectra 6)"),
    (ColorScheme.GRAYSCALE_4, "Grayscale 4"),
    (ColorScheme.GRAYSCALE_16,"Grayscale 16"),
    (ColorScheme.SEVEN_COLOR, "7-color (Spectra/ACeP)"),
]

scheme_cells = []
for i in range(0, len(SCHEMES), 4):
    row = []
    for scheme, name in SCHEMES[i:i+4]:
        cell = dithered_rgb(src2, scheme, DitherMode.BURKES)
        row.append(label(cell, name))
    scheme_cells.append(row)

grid(scheme_cells).save(OUT / "color_schemes_grid.png")

print("Done. Output in", OUT)

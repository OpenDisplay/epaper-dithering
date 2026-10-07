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


def hstack(images: list[Image.Image], gap: int = 4) -> Image.Image:
    """Place images side by side with a dark gutter."""
    out = Image.new("RGB", (sum(i.width for i in images) + gap * (len(images) - 1), max(i.height for i in images)), (15, 15, 15))
    x = 0
    for im in images:
        out.paste(im, (x, 0))
        x += im.width + gap
    return out


def vstack(images: list[Image.Image], gap: int = 4) -> Image.Image:
    """Stack images vertically with a dark gutter."""
    out = Image.new("RGB", (max(i.width for i in images), sum(i.height for i in images) + gap * (len(images) - 1)), (15, 15, 15))
    y = 0
    for im in images:
        out.paste(im, (0, y))
        y += im.height + gap
    return out


def zoom(img: Image.Image, box: tuple[int, int, int, int], factor: int = 4) -> Image.Image:
    """Crop and upscale with nearest-neighbour so individual pixels stay visible."""
    crop = img.crop(box)
    return crop.resize((crop.width * factor, crop.height * factor), Image.Resampling.NEAREST)


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
                                mode=DitherMode.BURKES, tone="auto", gamut="auto").convert("RGB")

p1 = label(frankfurt_orig,   "Original")
p2 = label(frankfurt_no_pre, "Spectra 6-color · Burkes · no preprocessing")
p3 = label(frankfurt_auto,   "Spectra 6-color · Burkes · auto tone + gamut")

hstack([p1, p2, p3]).save(OUT / "frankfurt_before_after.png")

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
    (DitherMode.DIZZY,              "Dizzy"),
]

# Two columns keep each cell large enough to judge once GitHub scales the image down.
algo_cells = []
for i in range(0, len(ALGOS), 2):
    row = []
    for mode, name in ALGOS[i:i+2]:
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

# ── 4. DBS refinement ─────────────────────────────────────────────────────────

print("Generating DBS comparison...")
src3 = load("river.png")
dbs_kwargs = {"mode": DitherMode.BURKES, "tone": "auto", "gamut": "auto"}
burkes = dither_image(src3, SPECTRA_7_3_6COLOR_V2, **dbs_kwargs).convert("RGB")
refined = dither_image(src3, SPECTRA_7_3_6COLOR_V2, dbs=True, **dbs_kwargs).convert("RGB")

# Sky, clouds and treeline: smooth gradients plus edges.
CROP = (230, 60, 430, 180)
dbs_rows = [
    [label(src3, "Original"), label(burkes, "Burkes"), label(refined, "Burkes + DBS")],
    [label(zoom(im, CROP), f"{name} · 4× crop") for im, name in
     [(src3, "Original"), (burkes, "Burkes"), (refined, "Burkes + DBS")]],
]
vstack([hstack(row) for row in dbs_rows]).save(OUT / "dbs_comparison.png")

print("Done. Output in", OUT)

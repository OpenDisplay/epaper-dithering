"""Write the dashboard + photo success fixture (800×480).

Left half: a white dashboard with pure #000000 text at several sizes (anti-aliased),
solid #000000 / #ff0000 / #ffff00 rectangles and a 1 px black grid. Right half: the
river.png fixture, scaled to fill.

    uv run --project packages/python python tools/make_dashboard_fixture.py
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps

ROOT = Path(__file__).resolve().parents[1]
IMAGES = ROOT / "packages/rust/core/tests/fixtures/images"
OUT = IMAGES / "success/dashboard_photo.png"

W, H = 800, 480
HALF = W // 2

# Solid regions the success test checks: (x0, y0, x1, y1) inclusive-exclusive, colour.
RECTS = [
    ((20, 300, 120, 380), (0, 0, 0)),
    ((140, 300, 240, 380), (255, 0, 0)),
    ((260, 300, 360, 380), (255, 255, 0)),
]


def main() -> None:
    img = Image.new("RGB", (W, H), (255, 255, 255))
    draw = ImageDraw.Draw(img)

    y = 16
    for size, text in [(40, "21.4 °C"), (24, "Living room"), (16, "Humidity 48 %  ·  CO2 612 ppm"), (12, "Updated 14:05")]:
        draw.text((20, y), text, fill=(0, 0, 0), font=ImageFont.load_default(size=size))
        y += size + 14

    for x in range(20, 380, 40):
        draw.line([(x, 190), (x, 280)], fill=(0, 0, 0), width=1)
    for gy in range(190, 281, 30):
        draw.line([(20, gy), (380, gy)], fill=(0, 0, 0), width=1)

    for (x0, y0, x1, y1), colour in RECTS:
        draw.rectangle([x0, y0, x1 - 1, y1 - 1], fill=colour)

    photo = Image.open(IMAGES / "river.png").convert("RGB")
    img.paste(ImageOps.fit(photo, (W - HALF, H), Image.Resampling.LANCZOS), (HALF, 0))

    OUT.parent.mkdir(parents=True, exist_ok=True)
    img.save(OUT)
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()

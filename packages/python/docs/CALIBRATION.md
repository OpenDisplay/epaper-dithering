# E-Paper Display Color Calibration Guide

This guide explains how to measure the actual RGB values of your e-paper display
for accurate dithering. E-paper displays are reflective, making their colors
significantly darker than pure RGB values:

- **Pure RGB assumption**: White=(255,255,255), Red=(255,0,0)
- **Reality**: White~(180-200), Red~(115-125) -- 30-87% darker

Using measured values produces much better dithering results. The library stores them
normalised to the panel's white (see step 6).

## What You Need

- **Camera** with RAW support (smartphone RAW works fine, e.g. iPhone ProRAW)
- **Two lights** with controllable color temperature (e.g. Philips Hue Play bars)
- **White paper sheet** as a reference (plain printer paper)
- **Photo editing software** with eyedropper/color picker (Photoshop, GIMP, Digital Color Meter, etc.)

## Step 1: Generate Calibration Patches

Use the included script to generate a patch image for your display:

```bash
# From the repo root. List the example palettes the script can draw patches for
uv run --project packages/python python tools/generate_patches.py --list

# Generate for your display (replace palette and size)
uv run --project packages/python python tools/generate_patches.py --palette ideal_bwgbry --size 800x480 -o patches.png
uv run --project packages/python python tools/generate_patches.py --palette ideal_bwr --size 296x128 -o patches.png
```

Each patch shows an entry's nominal colour, which is what you send to the panel for that ink.

The script automatically picks a grid layout (e.g. 3x2 for 6 colors on a
landscape display) that keeps patches as large as possible.

Display the generated image on your e-paper and wait for a full refresh to
complete.

## Step 2: Set Up Lighting

Place two lights at **45-degree angles** on either side of the display. This
eliminates glare and provides even illumination across the screen.

```
    [Light]           [Light]
       \  45°    45°  /
        \            /
         \          /
      +-----------------+
      |   e-paper       |
      |   display       |
      +-----------------+
```

Set both lights to **6500K** (daylight white point). On Hue lights this is
154 mireds. The exact color temperature matters less than consistency -- what's
important is that both lights match and the scene is evenly lit.

Make sure there are no other light sources (overhead lights, windows) that could
add a color cast.

## Step 3: Photograph

1. Place a **white paper sheet** next to the display -- this is your reference
   for normalizing the measurements
2. Shoot in **RAW** format (not JPEG -- RAW preserves the actual sensor values
   without white balance or tone curve adjustments)
3. Frame the shot to include both the display patches and the paper reference
4. Keep the camera perpendicular to the display
5. Avoid casting shadows with your body or the camera

## Step 4: Sample Raw RGB Values

Open the RAW file in your photo editor. Sample the average RGB value from the
center of each color patch and from the white paper reference.

Record the raw values:

```
Paper reference:  (215, 217, 218)

Black:   (22, 11, 30)
White:   (156, 172, 175)
Red:     (102, 8, 0)
Yellow:  (170, 157, 0)
Blue:    (0, 59, 119)
Green:   (34, 70, 49)
```

These are raw camera values and depend on your specific lighting setup. The
paper reference is what makes them comparable across setups.

## Step 5: Normalize

The raw values are relative to your lighting conditions. Normalize them against
the paper reference so that pure white paper maps to (255, 255, 255):

```
normalized = raw x (255 / paper_channel)
```

Applied **per channel** (R, G, B independently):

```
Paper reference: (215, 217, 218)
Scale factors:   (255/215, 255/217, 255/218) = (1.186, 1.175, 1.170)

Raw Black:  (22, 11, 30)
Normalized: (22 x 1.186, 11 x 1.175, 30 x 1.170)
          = (26, 13, 35)

Raw Red:    (102, 8, 0)
Normalized: (102 x 1.186, 8 x 1.175, 0 x 1.170)
          = (121, 9, 0)
```

Round to the nearest integer. Clamp to [0, 255] if any value exceeds the range.

> [!IMPORTANT]
> **Palette values must be sRGB-encoded (gamma), not linear-light.**
> The library decodes every palette color with the sRGB→linear transfer function before
> matching, so it expects standard gamma-encoded RGB — the same space a normal JPEG/PNG
> eyedropper reports. If you sampled from a **linear** workflow (e.g. a DNG developed with
> a *linear* tone curve, as in the SPECTRA v2 measurement), your normalized numbers are
> linear-light and must be gamma-encoded before you use them, or every color will read
> too dark. Apply the sRGB OETF per channel to each normalized value `c` in `[0, 1]`:
>
> ```
> encoded = 12.92 · c                        if c ≤ 0.0031308
>           1.055 · c^(1/2.4) − 0.055         otherwise
> ```
>
> then scale by 255. Standard photo editors (Photoshop/GIMP eyedropper on an sRGB export)
> already report gamma-encoded values, so no extra step is needed there.

## Step 6: Create Your Palette

The library keeps every palette **normalised to the panel's own white**: the entry named `white`
must be L\* 100, a\* = b\* = 0. That is a different reference from the paper sheet in step 5, so
the panel's white still comes out below 255 there (e.g. `(185, 202, 205)`), and the library
normalises once more, to it.

With sRGB values from step 5, `Palette.from_srgb` does that for you when one entry is the `white`
ink:

```python
from epaper_dithering import Palette

my_display = Palette.from_srgb(
    [
        ((26, 13, 35), "black", "#000000"),
        ((185, 202, 205), "white", "#ffffff"),
        ((202, 184, 0), "yellow", "#ffff00"),
        ((121, 9, 0), "red", "#ff0000"),
        ((0, 69, 139), "blue", "#0000ff"),
        ((40, 82, 57), "green", "#00ff00"),
    ],
    measured=True,
).with_source("Spectra 7.3in, RAW photo under two 5000 K lights, 2026-01-12")
```

If your tool reports CIELAB instead (a colorimeter, or a converted photo), use
`Palette.from_absolute_lab(entries, white_index=..., measured=True)` with `PaletteEntry` values.

Name every **pure ink** and give it the nominal colour your software sends for it; leave in-between
mixes (e.g. the extra colours of a multi-pass mode) unnamed. Named inks are what keeps requested
colours solid: see "How named inks behave" in the [README](../README.md#how-named-inks-behave).

Save it as a palette file and dither with it:

```python
from pathlib import Path
from epaper_dithering import DitherOptions, dither_image

Path("my_display.json").write_text(my_display.to_json())
result = dither_image(image, my_display, DitherOptions(mode="floyd_steinberg"))
```

## Entry Order

> [!IMPORTANT]
> The output index of a colour is its position in the palette. List the entries in the order your
> display driver expects, or remap the indices yourself before sending them.

## Sharing Your Measurements

To contribute a measured palette, add its JSON file to
[`examples/palettes/`](../../../examples/palettes/), describe it in that folder's README (panel
model, how and when it was measured), and open a PR. Measurements for any display model help.

## References

- **[esp32-photoframe](https://github.com/aitjcize/esp32-photoframe)** by
  [@aitjcize](https://github.com/aitjcize) for measuring actual
  e-paper display colors, camera calibration methodology
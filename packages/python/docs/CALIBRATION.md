# E-Paper Display Color Calibration Guide

This guide explains how to measure the actual RGB values of your e-paper display
for accurate dithering. E-paper displays are reflective, making their colors
significantly darker than pure RGB values:

- **Pure RGB assumption**: White=(255,255,255), Red=(255,0,0)
- **Reality**: White~(180-200), Red~(115-125) -- 30-87% darker

Using measured values produces much better dithering results.

## What You Need

- **Camera** with RAW support (smartphone RAW works fine, e.g. iPhone ProRAW)
- **Two lights** with controllable color temperature (e.g. Philips Hue Play bars)
- **White paper sheet** as a reference (plain printer paper)
- **Photo editing software** with eyedropper/color picker (Photoshop, GIMP, Digital Color Meter, etc.)

## Step 1: Generate Calibration Patches

Use the included script to generate a patch image for your display:

```bash
# List available color schemes
python scripts/generate_patches.py --list

# Generate for your display (replace scheme and size)
python scripts/generate_patches.py --scheme BWGBRY --size 800x480 -o patches.png
python scripts/generate_patches.py --scheme BWR --size 296x128 -o patches.png
```

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

## Step 6: Create Your ColorPalette

Use the normalized values to create a `ColorPalette`:

```python
from epaper_dithering import ColorPalette

my_display = ColorPalette(
    colors={
        "black": (26, 13, 35),
        "white": (185, 202, 205),
        "yellow": (202, 184, 0),
        "red": (121, 9, 0),
        "blue": (0, 69, 139),
        "green": (40, 82, 57),
    },
    accent="red",
)
```

Then use it for dithering:

```python
from epaper_dithering import dither_image, DitherMode

result = dither_image(image, my_display, DitherMode.FLOYD_STEINBERG)
```

## Color Order Requirement

> [!IMPORTANT]
> Color names and order must match the corresponding `ColorScheme`.
> Reordering colors will break palette encoding.

Check the reference order for your scheme:

```python
from epaper_dithering import ColorScheme

scheme = ColorScheme.BWGBRY
print(list(scheme.palette.colors.keys()))
# ['black', 'white', 'yellow', 'red', 'blue', 'green']
```

Your `ColorPalette` must use the same key order.

## Adding to the Library

To contribute your measurements:

1. Add a constant to `src/epaper_dithering/palettes.py`:
   ```python
   MY_DISPLAY_BWR = ColorPalette(
       colors={
           "black": (5, 5, 5),
           "white": (185, 190, 180),
           "red": (120, 15, 5),
       },
       accent="red",
   )
   ```

2. Export in `__init__.py`

3. Submit a PR -- measurements for any display model help the community!

## References

- **[esp32-photoframe](https://github.com/aitjcize/esp32-photoframe)** by
  [@aitjcize](https://github.com/aitjcize) for measuring actual
  e-paper display colors, camera calibration methodology
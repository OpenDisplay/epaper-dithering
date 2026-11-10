# Success-test palettes

## `bwry16_odbeeba6.json`

The 16-colour mode of a BWRY panel (OpenDisplay ODBEEBA6, 2.9" GDEY029F51H), measured on
2026-11-06 from a hand-held, camera-white-balanced photo of a 4×4 swatch grid. The source data is
`source_palette16_photo_ODBEEBA6.json`, copied unchanged from the OpenDisplay Silabs firmware repo
(`panels/palette16_photo_ODBEEBA6.json`): absolute CIELAB (D65) converted from the photo's sRGB,
with paper white at L\* 68.26.

It was converted with `Palette::from_absolute_lab(entries, white_index = 1, measured = true)`, which
adapts every entry so entry 1 (paper white) becomes exactly L\* 100, a\* = b\* = 0.

**The ink assignment is a fixture assumption.** The source data has no names, only per-pass panel
codes and Lab, so nothing says which entries are the pure inks. For this test they were picked by
colour:

| Entry | Ink | Nominal | Why |
|---|---|---|---|
| 1 | white | `#ffffff` | codes `[1, 1, 1]`, paper white |
| 14 | black | `#000000` | darkest entry (L\* 3.19 absolute) |
| 2 | yellow | `#ffff00` | strongest b\* (68.64) |
| 3 | red | `#ff0000` | strongest a\* (44.96) |

The other 12 entries are unnamed mixes. A real palette for this panel should take its ink names from
the device.

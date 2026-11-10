# Example palettes

Palette files in the `epaper-palette` format (version 1). They are starting points and test
material. **They are not shipped in any package** (crate, wheel or npm) and the library has no API
that returns them: load one explicitly with `Palette::from_json` / `Palette.load(...)`.

All files were exported from the built-in palettes of epaper-dithering 6.x by
`packages/rust/core/examples/export_catalog.rs` (removed after the export). Each entry is a named ink
whose `nominal` is the ideal colour of that ink.

| File | What it is |
|---|---|
| `ideal_*.json` | The 6.x idealised schemes (`#000000`, `#ff0000`, …). Appearance equals nominal, so anchoring does nothing. Use them only when no measurement exists. |
| `spectra_7_3_6color.json`, `spectra_7_3_6color_v2.json` | Measured 7.3" Spectra 6 panels |
| `mono_4_26.json` | Measured 4.26" black/white panel |
| `bwry_4_2.json`, `bwry_3_97.json` | Measured BWRY panels |
| `solum_bwr.json`, `hanshow_bwr.json`, `hanshow_bwy.json` | Measured electronic shelf label panels |

The 6.x measured values were sRGB relative to the camera, not to the panel's white (Spectra's white
was `185, 202, 205`). On export every palette was normalised so its white entry is L\* 100,
a\* = b\* = 0, as the library now requires. That brightens the measured colours compared with 6.x.

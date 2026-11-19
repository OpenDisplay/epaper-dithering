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
| `spectra_7_3_6color.json`, `spectra_7_3_6color_v2.json` | Waveshare 7.3" Spectra 6 panel, measured 2026-02-03 (iPhone RAW) and 2026-03-15 (linear DNG) |
| `spectra_7_3_6color_cr30.json` | 7.3" Spectra 6 panel (OpenDisplay board, panel IC `0x0023`), measured 2026-11-19 with a CHNSpec CR30 spectrocolorimeter (45°/0°, 31 bands 400–700 nm, 3 reads per ink) at 18.5 °C, via [epaper-colorcal](https://github.com/OpenDisplay/epaper-colorcal) |

The 6.x constants `MONO_4_26`, `BWRY_4_2`, `SOLUM_BWR`, `HANSHOW_BWR` and `HANSHOW_BWY` were never
measured (placeholder values) and are not provided.

The 6.x measured values were sRGB relative to the camera, not to the panel's white (Spectra's white
was `185, 202, 205`). On export every palette was normalised so its white entry is L\* 100,
a\* = b\* = 0, as the library now requires. That brightens the measured colours compared with 6.x.

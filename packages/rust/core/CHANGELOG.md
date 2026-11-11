# Changelog

## [7.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-core-v6.0.0...epaper-dithering-core-v7.0.0) (2026-11-11)


### ⚠ BREAKING CHANGES

* tone and gamut take numbers from 0.0 to 1.0 only; the GamutCompression enum and the "auto"/"off" values are removed.
* the tone option (DitherOptions.tone / ToneCompression, Python tone=, JS tone) and the tone_map dynamic-range functions are removed.
* **core:** ColorScheme, measured_palettes, dither_with_canonical, DitherConfig and DitherMode integer conversion are removed. Load palettes from epaper-palette files instead.

### Features

* **core:** add epaper-palette JSON format and export the built-in catalog as example files ([80af93b](https://github.com/OpenDisplay/epaper-dithering/commit/80af93b80a6a5b6117e7370888357e7096a5cb3e))
* **core:** add validated Palette type with CIELAB appearance and ink identity ([7c78aac](https://github.com/OpenDisplay/epaper-dithering/commit/7c78aac69fe44af9d290c16e8891829d9a003e48))
* **core:** anchor ink nominals onto measured appearances before matching ([7a01c7d](https://github.com/OpenDisplay/epaper-dithering/commit/7a01c7d5f30c1296e3af4c5f90ac8c8bab37c543))
* **core:** dither against runtime Palette; remove ColorScheme, built-in catalog and canonical palettes ([9bd3d2d](https://github.com/OpenDisplay/epaper-dithering/commit/9bd3d2d12f1fb45eb046828544fbc634c8c3d02f))
* make tone and gamut opt-in strengths; anchoring no longer lifts shadows ([cc80cd8](https://github.com/OpenDisplay/epaper-dithering/commit/cc80cd8c8ad07f56b8bcd3995797416a082879db))
* replace tone compression with black-point compensation in anchoring ([4dddf94](https://github.com/OpenDisplay/epaper-dithering/commit/4dddf94acb8d45af7f2a525f022ebc02ec3919c4))


### Bug Fixes

* **core:** keep anchoring monotonic for any set of named inks; reject non-finite appearances ([e959f1a](https://github.com/OpenDisplay/epaper-dithering/commit/e959f1a9bdbf08c270a1325deee70b191d035a65))

## [6.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-core-v5.0.0...epaper-dithering-core-v6.0.0) (2026-10-07)


### ⚠ BREAKING CHANGES

* **core:** `DitherConfig` has a new public field `dbs` (DBS refinement, added in #71). Code that constructs `DitherConfig` with an exhaustive struct literal must add `dbs: None`, or switch to `..Default::default()`.

### Features

* **core:** add DbsParams::validate ([33a4b89](https://github.com/OpenDisplay/epaper-dithering/commit/33a4b89034572869cf1dfe7fe58211e4d8a3b719))
* **core:** add Direct Binary Search (DBS) refinement ([d1da1eb](https://github.com/OpenDisplay/epaper-dithering/commit/d1da1ebb6b7907364cf9800046e996acf000af5d))


### Performance Improvements

* **core:** interleave DBS correlation channels and fuse chroma update ([6d8735f](https://github.com/OpenDisplay/epaper-dithering/commit/6d8735fd23a374a08974fe397e81083d7eee89e8))
* **core:** run DBS on parallel tiles ([1af7db6](https://github.com/OpenDisplay/epaper-dithering/commit/1af7db6d9238718cd376c4a90749da861da5c094))
* **core:** sweep DBS update footprint over row slices ([beddd43](https://github.com/OpenDisplay/epaper-dithering/commit/beddd43ccfd2131d3e24063d5cf234fdb2b891f7))
* **core:** truncate DBS eye-model blur at 2.5 sigma ([3ae0706](https://github.com/OpenDisplay/epaper-dithering/commit/3ae070609e4ea1be8d108dffb8dffe494d7a88dd))


### Documentation

* **core:** note that DitherConfig gained the dbs field ([4ba1956](https://github.com/OpenDisplay/epaper-dithering/commit/4ba19561a4fd8ea60a7ce3b349ad5d60abfc8082))

## [5.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-core-v4.0.1...epaper-dithering-core-v5.0.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* `ColorScheme` values 7 and 8 were realigned with protocol v2. **Value 7 now decodes as `SEVEN_COLOR` (Spectra/ACeP 7-color), not `GRAYSCALE_8`** — code or devices that hardcoded numeric value 7 to mean 8-level gray will now get 7-color inks. **Value 8 (`BWGBRY_SPLIT`)** is newly accepted where it previously raised. `GRAYSCALE_8` itself is **not removed** — it was renumbered to library-local value 9 and is fully supported (see issue [#19](https://github.com/OpenDisplay/epaper-dithering/issues/19)); code using the name `ColorScheme.GRAYSCALE_8` / `ColorScheme::Grayscale8` keeps working, but its numeric value changed from 7 to 9. Value 9 is a library-local dithering target (e.g. Inkplate 10), not an OpenDisplay firmware wire value, and must never be sent to a device as a `color_scheme`.

### Features

* add dizzy dithering (DitherMode 9) ([#68](https://github.com/OpenDisplay/epaper-dithering/issues/68)) ([4192900](https://github.com/OpenDisplay/epaper-dithering/commit/41929006ad4d319b3744b719331f2fbafb872ab8))
* realign ColorScheme with protocol v2 (SEVEN_COLOR, BWGBRY_SPLIT) ([#60](https://github.com/OpenDisplay/epaper-dithering/issues/60)) ([14adc2a](https://github.com/OpenDisplay/epaper-dithering/commit/14adc2ad57549e112ade50c3c3997f30688f0b49))


### Bug Fixes

* align ToneCompression::default() with DitherConfig::default() and fix misplaced doc comment ([#64](https://github.com/OpenDisplay/epaper-dithering/issues/64)) ([6970ecc](https://github.com/OpenDisplay/epaper-dithering/commit/6970ecc396da0fcc098823afbb13fab50f4a5d8e))
* **bench:** repair three dead fixture references ([#69](https://github.com/OpenDisplay/epaper-dithering/issues/69)) ([082b627](https://github.com/OpenDisplay/epaper-dithering/commit/082b6278f1c74d62270067119e8b9117542db355))
* close the ungated idealized-palette hole and the remaining audit findings ([#65](https://github.com/OpenDisplay/epaper-dithering/issues/65)) ([496858d](https://github.com/OpenDisplay/epaper-dithering/commit/496858d2f495748cd45c1da46fe448660df8c2a3))
* restore GRAYSCALE_8 at library-local value 9 (issue [#19](https://github.com/OpenDisplay/epaper-dithering/issues/19)) ([#70](https://github.com/OpenDisplay/epaper-dithering/issues/70)) ([f826ce4](https://github.com/OpenDisplay/epaper-dithering/commit/f826ce4586d6c2309ae9c3c03e98c4fdbf7c0527))

## [4.0.1](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-core-v4.0.0...epaper-dithering-core-v4.0.1) (2026-07-05)


### Bug Fixes

* **dither:** correct ordered-dither brightness, palette scaling, and tone-map edge cases ([6765760](https://github.com/OpenDisplay/epaper-dithering/commit/676576051fd3581df53ddcae8ee3f892a4133ba3))


### Performance Improvements

* **dither:** eliminate per-pixel powf in ordered dither; hoist gamut edges ([bafc435](https://github.com/OpenDisplay/epaper-dithering/commit/bafc435fbb4bd3c38af39d573fdd4392c01e86ff))

## [4.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-core-v3.0.0...epaper-dithering-core-v4.0.0) (2026-05-21)


### ⚠ BREAKING CHANGES

* ColorScheme.GRAYSCALE_8 is now value 7 and ColorScheme.GRAYSCALE_16 is now value 6. Code that hardcoded these integer values needs to be updated.
* **api:** positional args replaced with kwargs/options/config struct. Callers must update. See README for migration guide.
* unify versioning at 3.0.0 and add crates.io release

### Features

* add criterion benchmarks ([dfbafb2](https://github.com/OpenDisplay/epaper-dithering/commit/dfbafb289c0bf8ffaaf32d4bf4372c971eb42e53))
* **api:** flatten v4 API across Rust/Python/JS ([f74cbff](https://github.com/OpenDisplay/epaper-dithering/commit/f74cbff94404eac994e8222a4f0683653b73e898))
* derive measured palettes from Rust FFI ([081f2c4](https://github.com/OpenDisplay/epaper-dithering/commit/081f2c41f3a205c529ad6c209b6de1aabf672c18))
* preserve exact display colors for measured palettes ([2108514](https://github.com/OpenDisplay/epaper-dithering/commit/21085148ed11851790c7349e33566a892e46a7d7))
* **python:** rewrite as maturin mixed Rust/Python package ([3f54dfe](https://github.com/OpenDisplay/epaper-dithering/commit/3f54dfebc8c6132f096f901f08ec13ea587fa601))
* **rust:** add tone mapping and gamut compression to core ([1a0b4ae](https://github.com/OpenDisplay/epaper-dithering/commit/1a0b4ae633dd6c6350108456454a6306aa5f76a3))


### Bug Fixes

* correct GRAYSCALE_16=6 and GRAYSCALE_8=7 to match firmware ([a991067](https://github.com/OpenDisplay/epaper-dithering/commit/a991067820529081afb25ff5ab786c58c1c527ee))
* fix x86_64 wheel publishing and reduce crate size ([54d625c](https://github.com/OpenDisplay/epaper-dithering/commit/54d625c6a20ef0cfa2d1fdca9e5bfc99fcb8ee2a))
* optimize tone_map and fix LUT rounding ([766f41b](https://github.com/OpenDisplay/epaper-dithering/commit/766f41b62943d0a8e12be5be0d5e7ab37eae6756))
* publish musllinux wheel for Alpine Linux compatibility ([9c14bc6](https://github.com/OpenDisplay/epaper-dithering/commit/9c14bc66c77be2b3fb7608b977bcafb1d932e372))
* **rust:** apply Bayer threshold in sRGB space ([#27](https://github.com/OpenDisplay/epaper-dithering/issues/27)) ([1844f3b](https://github.com/OpenDisplay/epaper-dithering/commit/1844f3b801690ec9eb7424f8a2e88948481ba3e3))
* **rust:** replace LCH-weighted matcher with Cartesian OKLab ([#28](https://github.com/OpenDisplay/epaper-dithering/issues/28)) ([8b6686b](https://github.com/OpenDisplay/epaper-dithering/commit/8b6686b0933f078d8f08bd263eaad5bf2db3b3d5))
* satisfy rust clippy checks ([5563269](https://github.com/OpenDisplay/epaper-dithering/commit/5563269889c52909f7d88a01337241bf4043dd92))


### Miscellaneous Chores

* unify versioning at 3.0.0 and add crates.io release ([d1e6222](https://github.com/OpenDisplay/epaper-dithering/commit/d1e62222ef1e7f2d3c19a1c412b278877771337a))

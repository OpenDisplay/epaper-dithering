# Changelog

## [5.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-core-v4.0.1...epaper-dithering-core-v5.0.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* `GRAYSCALE_8` is removed from all three language surfaces. It occupied wire value 7, which protocol v2 reassigned to `SEVEN_COLOR` — gray8 was never a real panel scheme, and the header records it as removed rather than renumbered. Code referencing `ColorScheme.GRAYSCALE_8` / `ColorScheme::Grayscale8` must be deleted; there is no replacement. Value 7 now decodes as 7-color Spectra/ACeP, and value 8 (`BWGBRY_SPLIT`) is newly accepted where it previously raised.

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

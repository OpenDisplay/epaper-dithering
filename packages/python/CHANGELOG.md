# Changelog

## [6.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.9...epaper-dithering-v6.0.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* `ColorScheme` values 7 and 8 were realigned with protocol v2. **Value 7 now decodes as `SEVEN_COLOR` (Spectra/ACeP 7-color), not `GRAYSCALE_8`** — code or devices that hardcoded numeric value 7 to mean 8-level gray will now get 7-color inks. **Value 8 (`BWGBRY_SPLIT`)** is newly accepted where it previously raised. `GRAYSCALE_8` itself is **not removed** — it was renumbered to library-local value 9 and is fully supported (see issue [#19](https://github.com/OpenDisplay/epaper-dithering/issues/19)); code using the name `ColorScheme.GRAYSCALE_8` / `ColorScheme::Grayscale8` keeps working, but its numeric value changed from 7 to 9. Value 9 is a library-local dithering target (e.g. Inkplate 10), not an OpenDisplay firmware wire value, and must never be sent to a device as a `color_scheme`.

### Features

* add dizzy dithering (DitherMode 9) ([#68](https://github.com/OpenDisplay/epaper-dithering/issues/68)) ([4192900](https://github.com/OpenDisplay/epaper-dithering/commit/41929006ad4d319b3744b719331f2fbafb872ab8))
* realign ColorScheme with protocol v2 (SEVEN_COLOR, BWGBRY_SPLIT) ([#60](https://github.com/OpenDisplay/epaper-dithering/issues/60)) ([14adc2a](https://github.com/OpenDisplay/epaper-dithering/commit/14adc2ad57549e112ade50c3c3997f30688f0b49))


### Bug Fixes

* close the ungated idealized-palette hole and the remaining audit findings ([#65](https://github.com/OpenDisplay/epaper-dithering/issues/65)) ([496858d](https://github.com/OpenDisplay/epaper-dithering/commit/496858d2f495748cd45c1da46fe448660df8c2a3))
* restore GRAYSCALE_8 at library-local value 9 (issue [#19](https://github.com/OpenDisplay/epaper-dithering/issues/19)) ([#70](https://github.com/OpenDisplay/epaper-dithering/issues/70)) ([f826ce4](https://github.com/OpenDisplay/epaper-dithering/commit/f826ce4586d6c2309ae9c3c03e98c4fdbf7c0527))


### Performance Improvements

* release the GIL during dither_image (audit H2) ([#57](https://github.com/OpenDisplay/epaper-dithering/issues/57)) ([71eb8dc](https://github.com/OpenDisplay/epaper-dithering/commit/71eb8dc6e80384e606beac25ac1bd64247fca4bb))

## [5.0.9](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.8...epaper-dithering-v5.0.9) (2026-07-06)


### Documentation

* clarify DitherMode::None is for pre-quantized graphics, not photos ([f1c3ec5](https://github.com/OpenDisplay/epaper-dithering/commit/f1c3ec56fde94a6f6f96cd5816dce8ae7188c407))

## [5.0.8](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.7...epaper-dithering-v5.0.8) (2026-07-06)


### Bug Fixes

* **python:** ship the core tone-map NaN fix in the wheel ([296b939](https://github.com/OpenDisplay/epaper-dithering/commit/296b939cf127a77c69bb1821f4377360cabc5f06))
* **python:** ship the core tone-map NaN fix in the wheel ([3310c84](https://github.com/OpenDisplay/epaper-dithering/commit/3310c84b96fe31b7ef3340a2b622725c4d5247b9))

## [5.0.7](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.6...epaper-dithering-v5.0.7) (2026-06-29)


### Bug Fixes

* re-release Python sdist with corrected workspace manifest ([#40](https://github.com/OpenDisplay/epaper-dithering/issues/40)) ([371b029](https://github.com/OpenDisplay/epaper-dithering/commit/371b029ea336e02e37ec16e083e2b4358b7eaa93))

## [5.0.6](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.5...epaper-dithering-v5.0.6) (2026-05-21)


### Bug Fixes

* include OS in artifact name to prevent x86_64 collision between Linux and Windows builds ([c3db2d2](https://github.com/OpenDisplay/epaper-dithering/commit/c3db2d2922cec5daa6308d64d7f6f99ad7b8f2ef))

## [5.0.3](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.2...epaper-dithering-v5.0.3) (2026-05-21)


### Bug Fixes

* skip already-uploaded wheels on PyPI publish retry ([b6f0bd5](https://github.com/OpenDisplay/epaper-dithering/commit/b6f0bd56aa349a9e3fee076f058b5e3704754551))

## [5.0.2](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.1...epaper-dithering-v5.0.2) (2026-05-21)


### Bug Fixes

* broaden release-please scope to repo root ([50f0cfe](https://github.com/OpenDisplay/epaper-dithering/commit/50f0cfeecf87179c6438348c769c795573bc2376))
* fix x86_64 wheel publishing and reduce crate size ([54d625c](https://github.com/OpenDisplay/epaper-dithering/commit/54d625c6a20ef0cfa2d1fdca9e5bfc99fcb8ee2a))

## [5.0.1](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v5.0.0...epaper-dithering-v5.0.1) (2026-05-21)


### Bug Fixes

* publish musllinux wheel for Alpine Linux compatibility ([9c14bc6](https://github.com/OpenDisplay/epaper-dithering/commit/9c14bc66c77be2b3fb7608b977bcafb1d932e372))

## [5.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v4.1.0...epaper-dithering-v5.0.0) (2026-05-18)


### ⚠ BREAKING CHANGES

* ColorScheme.GRAYSCALE_8 is now value 7 and ColorScheme.GRAYSCALE_16 is now value 6. Code that hardcoded these integer values needs to be updated.

### Bug Fixes

* correct GRAYSCALE_16=6 and GRAYSCALE_8=7 to match firmware ([a991067](https://github.com/OpenDisplay/epaper-dithering/commit/a991067820529081afb25ff5ab786c58c1c527ee))

## [4.1.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v4.0.0...epaper-dithering-v4.1.0) (2026-05-05)


### Features

* preserve exact display colors for measured palettes ([2108514](https://github.com/OpenDisplay/epaper-dithering/commit/21085148ed11851790c7349e33566a892e46a7d7))

## [4.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-v3.0.0...epaper-dithering-v4.0.0) (2026-04-28)


### ⚠ BREAKING CHANGES

* **api:** positional args replaced with kwargs/options/config struct. Callers must update. See README for migration guide.
* unify versioning at 3.0.0 and add crates.io release
* tone_compression default changed from 1.0 to "auto".
* replace LAB hue penalty with weighted LCH distance and fix Burkes kernel
* All dithering algorithms now use LAB color space instead of weighted RGB. Colors will look different (more accurate) than previous versions. Fixes yellow dominance in faces and skin tones. Performance optimized with palette pre-conversion for minimal overhead.
* Dithering output has changed due to color science improvements:
    - All algorithms now work in linear RGB space with IEC 61966-2-1 sRGB gamma correction
    - Color matching uses ITU-R BT.601 perceptual luma weighting instead of Euclidean distance
    - RGBA images now composite on white background (e-paper assumption) instead of black
    - Ordered dithering completely rewritten to fix broken 0-240 bias bug
* initial release with dithering algorithms for e-paper displays

### Features

* add auto mode for gamut compression ([9a9ffee](https://github.com/OpenDisplay/epaper-dithering/commit/9a9ffee1f734af51c3a96fa09178a3a1036538be))
* add auto tone compression as default ([a8fa4ca](https://github.com/OpenDisplay/epaper-dithering/commit/a8fa4cab91e9bebe7a6ff78f5faa38e83903474e))
* add BWRY_3_97 calibration ([b670645](https://github.com/OpenDisplay/epaper-dithering/commit/b670645e5815d71e290441f21bd74dc4319d1a7d))
* add comparison script ([d60c797](https://github.com/OpenDisplay/epaper-dithering/commit/d60c79773623dae363a6688f2bc36b2999f5455f))
* add dynamic range compression for measured palettes ([f3e37a5](https://github.com/OpenDisplay/epaper-dithering/commit/f3e37a51eeb1c566d629ef8dc854e53732405316))
* add gamut compression ([c5e4df9](https://github.com/OpenDisplay/epaper-dithering/commit/c5e4df90c1cf33ce1b4e1f12d184c1f382254352))
* add gamut_compression sheet to compare script ([f0c1db4](https://github.com/OpenDisplay/epaper-dithering/commit/f0c1db4e7d485fd8eb2f5313f34a832477d90421))
* add GRAYSCALE_8 and GRAYSCALE_16 ([4e9711a](https://github.com/OpenDisplay/epaper-dithering/commit/4e9711a6b4a6545fe4451bce2fef59fef3cbfe3e))
* add LAB color space for perceptual color matching ([7a5cb07](https://github.com/OpenDisplay/epaper-dithering/commit/7a5cb07baa81dbc69e641612ca4b3b622ee6694f))
* add measured display color support for accurate dithering ([b5d3df3](https://github.com/OpenDisplay/epaper-dithering/commit/b5d3df324b2eb245d47c2a904e9298530fa9bae4))
* add prek and update typing ([f323692](https://github.com/OpenDisplay/epaper-dithering/commit/f3236925ccb065ff23d161f93dbb0fe789094dd0))
* add py.typed marker and fix strict mypy errors ([22ecf06](https://github.com/OpenDisplay/epaper-dithering/commit/22ecf06199712565a182e0a1dc7ff4c95271c345))
* add SPECTRA_7_3_6COLOR_V2 palette with calibrated measurements ([fac7112](https://github.com/OpenDisplay/epaper-dithering/commit/fac71125e4efa1da0f7c7b0371f2d435d7309006))
* **api:** flatten v4 API across Rust/Python/JS ([f74cbff](https://github.com/OpenDisplay/epaper-dithering/commit/f74cbff94404eac994e8222a4f0683653b73e898))
* change lab to oklab ([7904094](https://github.com/OpenDisplay/epaper-dithering/commit/79040946367ab20f38cce9ca5746466d499fefa3))
* **compare:** add gamut_compression contact sheet; fix int literals ([7755c3d](https://github.com/OpenDisplay/epaper-dithering/commit/7755c3dde42fed2b199c7823c47fb1d114374177))
* derive measured palettes from Rust FFI ([081f2c4](https://github.com/OpenDisplay/epaper-dithering/commit/081f2c41f3a205c529ad6c209b6de1aabf672c18))
* implement reference-quality color science for dithering ([e151fbf](https://github.com/OpenDisplay/epaper-dithering/commit/e151fbfd836176e32adf9a290f55d242167def82))
* initial release with dithering algorithms for e-paper displays ([03a5b4e](https://github.com/OpenDisplay/epaper-dithering/commit/03a5b4e59f5b3531b7607478f6ab6cc097a7feab))
* **python:** rewrite as maturin mixed Rust/Python package ([3f54dfe](https://github.com/OpenDisplay/epaper-dithering/commit/3f54dfebc8c6132f096f901f08ec13ea587fa601))
* **tone-map:** continuous strength for auto gamut compression ([68d9b7c](https://github.com/OpenDisplay/epaper-dithering/commit/68d9b7cf53e27e0df7ab7d4a387957a54ca22fc2))
* **tone-map:** hue-preserving gamut compression ([bdddc97](https://github.com/OpenDisplay/epaper-dithering/commit/bdddc9756c9098ef6bd4bf616b7385cb2f317d52))
* **tone-map:** iterative optimization for auto gamut compression strength ([4263183](https://github.com/OpenDisplay/epaper-dithering/commit/4263183aabc97113908dafa22bb9c89fe8e9cfe7))
* **tone-map:** use Reinhard 2004 log-skewness for auto compression strength ([715dc0a](https://github.com/OpenDisplay/epaper-dithering/commit/715dc0af89fc1236ef7f50fcdc0b30f5520f0bd8))


### Bug Fixes

* **ci:** unbreak python build and rust 1.95 clippy ([b24a139](https://github.com/OpenDisplay/epaper-dithering/commit/b24a1393fa2e455a7f473579fb85defe63f86cfd))
* correct OKLab LCH weights and gamut compression gating ([05685a8](https://github.com/OpenDisplay/epaper-dithering/commit/05685a86529d9ef3319b8785b38a0f27b62aebb8))
* replace LAB hue penalty with weighted LCH distance and fix Burkes kernel ([71cb04c](https://github.com/OpenDisplay/epaper-dithering/commit/71cb04c575d77d9ed0e665c569738c220c8937b0))
* swap color order in ColorScheme.BWRY.palette ([812a2d8](https://github.com/OpenDisplay/epaper-dithering/commit/812a2d8d47589bd61689ed6d8b54a0bd45e0f328))
* **tests:** update LAB/auto-compress tests for OKLab and conservative auto strength ([86b1075](https://github.com/OpenDisplay/epaper-dithering/commit/86b107513085ba7b0967022f3c3b95f677c84104))
* updated hooks and ci ([c54a675](https://github.com/OpenDisplay/epaper-dithering/commit/c54a6754a409e1a16cc3afa353357cdc4ac4030d))
* use sRGB space for error diffusion, conditional auto tone compression ([fab9017](https://github.com/OpenDisplay/epaper-dithering/commit/fab9017f500e0e81945be41667d2a3015859b4bc))


### Performance Improvements

* eliminate numpy overhead in error diffusion inner loop ([49dccd7](https://github.com/OpenDisplay/epaper-dithering/commit/49dccd77067a667ace050bf33ad233380996b303))
* vectorize palette matching and pixel processing with NumPy broadcasting ([410f2c1](https://github.com/OpenDisplay/epaper-dithering/commit/410f2c1ac5e1db2407134714d1e2f091891d82a9))


### Documentation

* add color calibration guide ([723d07a](https://github.com/OpenDisplay/epaper-dithering/commit/723d07ae1a42693f2a4e30924c31a3d30bc94b77))
* regenerate examples and update READMEs for v4 API ([3927d79](https://github.com/OpenDisplay/epaper-dithering/commit/3927d79e1553d10455c381af0509fae0eb71c8ea))
* update README with badges for PyPI, npm, tests, and linting ([3c61e36](https://github.com/OpenDisplay/epaper-dithering/commit/3c61e360cd835983a4d98dfbc5ce20345ce3b7e2))
* Update READMEs ([3a7e5ad](https://github.com/OpenDisplay/epaper-dithering/commit/3a7e5ad2007b69f30e9751a7c7e79edbcf43644d))


### Miscellaneous Chores

* unify versioning at 3.0.0 and add crates.io release ([d1e6222](https://github.com/OpenDisplay/epaper-dithering/commit/d1e62222ef1e7f2d3c19a1c412b278877771337a))

## [0.6.4](https://github.com/OpenDisplay/epaper-dithering/compare/python-v0.6.3...python-v0.6.4) (2026-03-07)


### Features

* add GRAYSCALE_8 and GRAYSCALE_16 ([4e9711a](https://github.com/OpenDisplay/epaper-dithering/commit/4e9711a6b4a6545fe4451bce2fef59fef3cbfe3e))

## [0.6.3](https://github.com/OpenDisplay/epaper-dithering/compare/python-v0.6.2...python-v0.6.3) (2026-03-07)


### Features

* add prek and update typing ([f323692](https://github.com/OpenDisplay/epaper-dithering/commit/f3236925ccb065ff23d161f93dbb0fe789094dd0))


### Bug Fixes

* updated hooks and ci ([c54a675](https://github.com/OpenDisplay/epaper-dithering/commit/c54a6754a409e1a16cc3afa353357cdc4ac4030d))

## [0.6.2](https://github.com/OpenDisplay/epaper-dithering/compare/python-v0.6.1...python-v0.6.2) (2026-03-07)


### Features

* add py.typed marker and fix strict mypy errors ([22ecf06](https://github.com/OpenDisplay/epaper-dithering/commit/22ecf06199712565a182e0a1dc7ff4c95271c345))

## [0.6.1](https://github.com/OpenDisplay/epaper-dithering/compare/python-v0.6.0...python-v0.6.1) (2026-03-06)


### Features

* add BWRY_3_97 calibration ([b670645](https://github.com/OpenDisplay/epaper-dithering/commit/b670645e5815d71e290441f21bd74dc4319d1a7d))

## [0.6.0](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.5.3...python-v0.6.0) (2026-02-11)


### ⚠ BREAKING CHANGES

* tone_compression default changed from 1.0 to "auto".

### Features

* add auto tone compression as default ([a8fa4ca](https://github.com/OpenDisplay-org/epaper-dithering/commit/a8fa4cab91e9bebe7a6ff78f5faa38e83903474e))

## [0.5.3](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.5.2...python-v0.5.3) (2026-02-10)


### Features

* add dynamic range compression for measured palettes ([f3e37a5](https://github.com/OpenDisplay-org/epaper-dithering/commit/f3e37a51eeb1c566d629ef8dc854e53732405316))


### Documentation

* update README with badges for PyPI, npm, tests, and linting ([3c61e36](https://github.com/OpenDisplay-org/epaper-dithering/commit/3c61e360cd835983a4d98dfbc5ce20345ce3b7e2))

## [0.5.2](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.5.1...python-v0.5.2) (2026-02-09)


### Bug Fixes

* swap color order in ColorScheme.BWRY.palette ([812a2d8](https://github.com/OpenDisplay-org/epaper-dithering/commit/812a2d8d47589bd61689ed6d8b54a0bd45e0f328))


### Documentation

* add color calibration guide ([723d07a](https://github.com/OpenDisplay-org/epaper-dithering/commit/723d07ae1a42693f2a4e30924c31a3d30bc94b77))

## [0.5.1](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.5.0...python-v0.5.1) (2026-02-09)


### Performance Improvements

* eliminate numpy overhead in error diffusion inner loop ([49dccd7](https://github.com/OpenDisplay-org/epaper-dithering/commit/49dccd77067a667ace050bf33ad233380996b303))

## [0.5.0](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.4.0...python-v0.5.0) (2026-02-09)


### ⚠ BREAKING CHANGES

* replace LAB hue penalty with weighted LCH distance and fix Burkes kernel

### Bug Fixes

* replace LAB hue penalty with weighted LCH distance and fix Burkes kernel ([71cb04c](https://github.com/OpenDisplay-org/epaper-dithering/commit/71cb04c575d77d9ed0e665c569738c220c8937b0))

## [0.4.0](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.3.2...python-v0.4.0) (2026-02-03)


### ⚠ BREAKING CHANGES

* All dithering algorithms now use LAB color space instead of weighted RGB. Colors will look different (more accurate) than previous versions. Fixes yellow dominance in faces and skin tones. Performance optimized with palette pre-conversion for minimal overhead.

### Features

* add LAB color space for perceptual color matching ([7a5cb07](https://github.com/OpenDisplay-org/epaper-dithering/commit/7a5cb07baa81dbc69e641612ca4b3b622ee6694f))

## [0.3.2](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.3.1...python-v0.3.2) (2026-02-03)


### Performance Improvements

* vectorize palette matching and pixel processing with NumPy broadcasting ([410f2c1](https://github.com/OpenDisplay-org/epaper-dithering/commit/410f2c1ac5e1db2407134714d1e2f091891d82a9))

## [0.3.1](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.3.0...python-v0.3.1) (2026-02-02)


### Features

* add measured display color support for accurate dithering ([b5d3df3](https://github.com/OpenDisplay-org/epaper-dithering/commit/b5d3df324b2eb245d47c2a904e9298530fa9bae4))

## [0.3.0](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.2.0...python-v0.3.0) (2026-02-02)


### ⚠ BREAKING CHANGES

* Dithering output has changed due to color science improvements:
    - All algorithms now work in linear RGB space with IEC 61966-2-1 sRGB gamma correction
    - Color matching uses ITU-R BT.601 perceptual luma weighting instead of Euclidean distance
    - RGBA images now composite on white background (e-paper assumption) instead of black
    - Ordered dithering completely rewritten to fix broken 0-240 bias bug

### Features

* implement reference-quality color science for dithering ([e151fbf](https://github.com/OpenDisplay-org/epaper-dithering/commit/e151fbfd836176e32adf9a290f55d242167def82))

## [0.2.0](https://github.com/OpenDisplay-org/epaper-dithering/compare/python-v0.1.0...python-v0.2.0) (2026-01-16)


### ⚠ BREAKING CHANGES

* initial release with dithering algorithms for e-paper displays

### Features

* initial release with dithering algorithms for e-paper displays ([03a5b4e](https://github.com/OpenDisplay-org/epaper-dithering/commit/03a5b4e59f5b3531b7607478f6ab6cc097a7feab))

## 0.1.0 (2026-01-11)


### ⚠ BREAKING CHANGES

* initial release with dithering algorithms for e-paper displays

### Features

* initial release with dithering algorithms for e-paper displays ([03a5b4e](https://github.com/OpenDisplay-org/epaper-dithering/commit/03a5b4e59f5b3531b7607478f6ab6cc097a7feab))

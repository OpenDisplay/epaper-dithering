# Changelog

## [0.6.5](https://github.com/OpenDisplay/epaper-dithering/compare/python-v0.6.4...python-v0.6.5) (2026-03-16)


### Features

* add auto mode for gamut compression ([9a9ffee](https://github.com/OpenDisplay/epaper-dithering/commit/9a9ffee1f734af51c3a96fa09178a3a1036538be))
* add comparison script ([d60c797](https://github.com/OpenDisplay/epaper-dithering/commit/d60c79773623dae363a6688f2bc36b2999f5455f))
* add gamut compression ([c5e4df9](https://github.com/OpenDisplay/epaper-dithering/commit/c5e4df90c1cf33ce1b4e1f12d184c1f382254352))
* add gamut_compression sheet to compare script ([f0c1db4](https://github.com/OpenDisplay/epaper-dithering/commit/f0c1db4e7d485fd8eb2f5313f34a832477d90421))
* add SPECTRA_7_3_6COLOR_V2 palette with calibrated measurements ([fac7112](https://github.com/OpenDisplay/epaper-dithering/commit/fac71125e4efa1da0f7c7b0371f2d435d7309006))
* change lab to oklab ([7904094](https://github.com/OpenDisplay/epaper-dithering/commit/79040946367ab20f38cce9ca5746466d499fefa3))
* **compare:** add gamut_compression contact sheet; fix int literals ([7755c3d](https://github.com/OpenDisplay/epaper-dithering/commit/7755c3dde42fed2b199c7823c47fb1d114374177))
* **tone-map:** continuous strength for auto gamut compression ([68d9b7c](https://github.com/OpenDisplay/epaper-dithering/commit/68d9b7cf53e27e0df7ab7d4a387957a54ca22fc2))
* **tone-map:** hue-preserving gamut compression ([bdddc97](https://github.com/OpenDisplay/epaper-dithering/commit/bdddc9756c9098ef6bd4bf616b7385cb2f317d52))
* **tone-map:** iterative optimization for auto gamut compression strength ([4263183](https://github.com/OpenDisplay/epaper-dithering/commit/4263183aabc97113908dafa22bb9c89fe8e9cfe7))
* **tone-map:** use Reinhard 2004 log-skewness for auto compression strength ([715dc0a](https://github.com/OpenDisplay/epaper-dithering/commit/715dc0af89fc1236ef7f50fcdc0b30f5520f0bd8))


### Bug Fixes

* correct OKLab LCH weights and gamut compression gating ([05685a8](https://github.com/OpenDisplay/epaper-dithering/commit/05685a86529d9ef3319b8785b38a0f27b62aebb8))
* **tests:** update LAB/auto-compress tests for OKLab and conservative auto strength ([86b1075](https://github.com/OpenDisplay/epaper-dithering/commit/86b107513085ba7b0967022f3c3b95f677c84104))
* use sRGB space for error diffusion, conditional auto tone compression ([fab9017](https://github.com/OpenDisplay/epaper-dithering/commit/fab9017f500e0e81945be41667d2a3015859b4bc))

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

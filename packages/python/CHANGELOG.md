# Changelog

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

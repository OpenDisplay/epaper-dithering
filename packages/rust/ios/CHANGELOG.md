# Changelog

## [1.0.1](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-ios-v1.0.0...epaper-dithering-ios-v1.0.1) (2026-11-09)


### Bug Fixes

* **ios:** remove embedded bitcode so the release XCFramework is ~5x smaller ([6801486](https://github.com/OpenDisplay/epaper-dithering/commit/68014861acb15027174993e1304c7fd7e0958151))

## [1.0.0](https://github.com/OpenDisplay/epaper-dithering/compare/epaper-dithering-ios-v0.1.0...epaper-dithering-ios-v1.0.0) (2026-11-09)


### ⚠ BREAKING CHANGES

* **ios:** ed_dither's signature changed and ed_abi_version() is now 2. The only consumer, the OpenDisplay iOS app, vendors the XCFramework and updates in lockstep.

### Features

* Add C-ABI iOS/Swift FFI wrapper crate (dithering only) ([64edb20](https://github.com/OpenDisplay/epaper-dithering/commit/64edb20d6363fe11c1f45360f3fd4f383b057adc))
* **ios:** take an options struct in ed_dither and expose DBS ([63d8289](https://github.com/OpenDisplay/epaper-dithering/commit/63d8289ebff95b69bb6d334123bf69744a4d5945))

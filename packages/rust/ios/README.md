# epaper-dithering-ios

C-ABI FFI wrapper around [`epaper-dithering-core`](../core) for the OpenDisplay **iOS/Swift**
app. Sibling of the [`wasm`](../wasm) crate — same "thin wrapper over the core" role, different
ABI (plain C instead of `wasm-bindgen`).

## Scope (deliberately minimal)

Exposes **only dithering** — OKLab palette matching + error diffusion, plus the optional Direct
Binary Search (DBS) refinement that runs after it. It does **not** surface
the core's tone / gamut / exposure / saturation pre-processing: the iOS app keeps its own
tone-compression pass in Swift for now and hands us already-pre-processed pixels, so we run
pure matching + diffusion at `DitherConfig` defaults (all pre-processing off).

Why a wrapper at all: the app currently reimplements dithering in Swift with **sRGB-Euclidean**
nearest-color matching, which diverges from the core / website / Python (**OKLab**). This makes
the iOS output match the reference implementation exactly.

## API

See [`include/epaper_dithering.h`](include/epaper_dithering.h):

- `ed_dither(..., const ed_dither_options *opts, ...) -> int32_t` — dither a flat sRGB image
  to palette indices.
- `ed_dither_options` — mode, serpentine, and DBS settings (`dbs_max_passes`, 0 = off;
  `dbs_viewing_distance_cm`; `dbs_ppi`). Mirrors the core's `DitherConfig`.
- `ed_dither_options_default()` / `ed_dither_options_default_dbs_passes()` — the core's
  defaults, so callers never hard-code them.
- `ed_abi_version() -> uint32_t` — ABI version (currently `2`; `1` took `mode_id, serpentine`
  as arguments and had no DBS).

Contract highlights:

- **Output is caller-allocated.** Output length is deterministic — exactly `width * height`
  indices — so Swift allocates the buffer; nothing is allocated across the boundary and there
  is no free function.
- **Indices are in the palette you pass.** Feed the app's palette in the app's index order and
  the returned indices line up with the app's wire-format packing with no remap.
- **Panics never cross the boundary** (`catch_unwind` → `ED_ERR_PANIC`); errors are negative
  status codes, never exceptions.

## Build & test

Host-side unit/integration tests (no Xcode, no iOS targets needed):

```sh
cargo test --manifest-path packages/rust/ios/Cargo.toml
```

Produce the XCFramework (macOS + Xcode + Rust; the **only** Xcode-gated step in the repo):

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
packages/rust/ios/build-xcframework.sh
# → packages/rust/ios/EpaperDithering.xcframework
```

The resulting `EpaperDithering.xcframework` (device arm64 + simulator arm64/x86_64) is vendored
into the app repo, not committed here.

## Releases

The crate is versioned by release-please as `epaper-dithering-ios` (tags
`epaper-dithering-ios-vX.Y.Z`, changelog in `CHANGELOG.md`). It is not published to crates.io;
instead `release.yml` builds the XCFramework on macOS and attaches it to the GitHub release as
`EpaperDithering.xcframework.zip`, with its SwiftPM checksum in
`EpaperDithering.xcframework.zip.checksum`. Use conventional commits scoped `ios`
(`feat(ios): …`, `fix(ios)!: …`); a change to `ed_dither`'s signature or `ed_dither_options` is
breaking and must bump `ed_abi_version()`.

## Swift usage sketch

```swift
import EpaperDithering

// pixels: [UInt8] flat RGB (w*h*3); palette: app palette bytes in app index order
var opts = ed_dither_options_default()
opts.mode_id = modeId
opts.dbs_max_passes = ed_dither_options_default_dbs_passes()   // opt in to DBS (slower)
opts.dbs_ppi = panelPPI                                         // e.g. from the panel's mm size
var indices = [UInt8](repeating: 0, count: w * h)
let status = pixels.withUnsafeBufferPointer { px in
    palette.withUnsafeBufferPointer { pal in
        indices.withUnsafeMutableBufferPointer { out in
            ed_dither(px.baseAddress, px.count, w,
                      pal.baseAddress, pal.count, accentIdx,
                      nil, 0, 0,           // no canonical palette
                      &opts,
                      out.baseAddress, out.count)
        }
    }
}
precondition(status == ED_OK)
// `indices` are palette indices in app order → hand straight to the existing packer.
```

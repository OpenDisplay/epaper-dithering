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

- `ed_palette_from_json(json, len, &palette) -> int32_t` — parse an `epaper-palette` JSON
  document into an opaque, immutable handle; `ed_palette_free(palette)` releases it (once).
- `ed_palette_len(palette)` / `ed_palette_preview_rgb(palette, out, out_len)` — entry count and
  each entry's sRGB preview colour.
- `ed_dither(pixels, pixels_len, width, palette, &opts, out, out_len) -> int32_t` — dither a
  flat sRGB image to palette indices.
- `ed_dither_options` — `mode` (a NUL-terminated name such as `"burkes"`), `serpentine`,
  `anchor`, `snap`, and DBS settings (`dbs_max_passes`, 0 = off; `dbs_viewing_distance_cm`;
  `dbs_ppi`). Mirrors the core's `DitherOptions`.
- `ed_dither_options_default()` / `ed_dither_options_default_dbs_passes()` — the core's
  defaults, so callers never hard-code them.
- `ed_abi_version() -> uint32_t` — ABI version (currently `3`: palette handles, named modes,
  anchor/snap; `2` took raw palette bytes, an accent index and an optional canonical palette).

Contract highlights:

- **Output is caller-allocated.** Output length is deterministic — exactly `width * height`
  indices — so Swift allocates the buffer. The only allocation crossing the boundary is the
  palette handle, which `ed_palette_free` releases.
- **Indices are in the palette's entry order.** Build the palette file with entries in the
  app's index order and the returned indices line up with the app's wire-format packing.
- **Panics never cross the boundary** (`catch_unwind` → `ED_ERR_PANIC`); errors are negative
  status codes, never exceptions.

## Swift usage sketch

```swift
import EpaperDithering

// paletteJSON: Data holding an epaper-palette document, entries in app index order
var palette: OpaquePointer?
let parsed = paletteJSON.withUnsafeBytes { json in
    ed_palette_from_json(json.baseAddress?.assumingMemoryBound(to: CChar.self), json.count, &palette)
}
precondition(parsed == ED_OK)
defer { ed_palette_free(palette) }

var opts = ed_dither_options_default()
opts.dbs_max_passes = ed_dither_options_default_dbs_passes()   // opt in to DBS (slower)
opts.dbs_ppi = panelPPI                                         // e.g. from the panel's mm size
var indices = [UInt8](repeating: 0, count: w * h)
let status = "floyd_steinberg".withCString { mode in
    opts.mode = mode
    return pixels.withUnsafeBufferPointer { px in
        indices.withUnsafeMutableBufferPointer { out in
            ed_dither(px.baseAddress, px.count, w, palette, &opts, out.baseAddress, out.count)
        }
    }
}
precondition(status == ED_OK)
// `indices` are palette indices in app order → hand straight to the existing packer.
```

## Build & test

Host-side unit/integration tests (no Xcode, no iOS targets needed):

```sh
cargo test --manifest-path packages/rust/ios/Cargo.toml
```

Produce the XCFramework (macOS + Xcode + Rust; the **only** Xcode-gated step in the repo):

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
rustup component add llvm-tools
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
(`feat(ios): …`, `fix(ios)!: …`); a change to an exported signature or to `ed_dither_options` is
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

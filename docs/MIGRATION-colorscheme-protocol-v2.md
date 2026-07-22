# Migration: ColorScheme realigned to protocol v2

**What changed:** `epaper-dithering` PR #60 (`14adc2a`) realigned the `ColorScheme` enum with the
canonical wire contract in `opendisplay-protocol/src/opendisplay_structs.h`. This is a **breaking
change** across all three language surfaces.

| value | before | after |
|---|---|---|
| 0-6 | MONO, BWR, BWY, BWRY, BWGBRY, GRAYSCALE_4, GRAYSCALE_16 | unchanged |
| **7** | `GRAYSCALE_8` | **`SEVEN_COLOR`** (Spectra/ACeP 7) |
| **8** | *(rejected)* | **`BWGBRY_SPLIT`** (dual-CS panels) |

`GRAYSCALE_8` was **deleted, not renumbered** — the canonical header states it "was a mistake
(gray8 is not a real scheme) and is removed". Values 100-102 (RGB565/888/16BPC) remain out of
scope for this library.

**Canonical ink order** for the new schemes, verified against the vendored bb_epaper driver
(`bb_epaper.h:300-306`, `u8Colors_7clr` at `bb_ep.inl:76`):

```
SEVEN_COLOR   = black, white, yellow, red, blue, green, orange   (orange = [255,128,0], nominal)
BWGBRY_SPLIT  = black, white, yellow, red, blue, green           (same inks as BWGBRY)
```

`BWGBRY_SPLIT` differs from `BWGBRY` only in **packing** (left-half plane then right-half plane),
which is the consumer's concern, not this library's.

---

## Severity by project

| project | severity | why |
|---|---|---|
| **py-opendisplay** | **HIGH** | Owns the encoders. Uploads to 7-colour and split panels are impossible until updated. |
| **OD-App** (iOS) | **HIGH** | Silently mis-renders scheme 7 as monochrome and computes the wrong packed byte count. |
| **Web / opendisplay.org** | **MEDIUM** | Ships a stale vendored bundle with `GRAYSCALE_8 = 7` baked in. |
| **opendisplay-js** | **LOW** | Dependency range is years stale; won't pick this up at all. |
| **Home_Assistant_Integration** | **LOW** | Transitive only — a pin bump once py-opendisplay ships. |
| **opendisplay-protocol** | **DOC** | One audit finding needs correcting. |

Good news that limits the blast radius: **no downstream source references `GRAYSCALE_8`.** The
removal itself breaks nothing. Nearly all the work below is *adding* the two new schemes.

---

## 1. py-opendisplay — HIGH

Imports the enum directly (`from epaper_dithering import ColorScheme`), so the new members appear
automatically on a version bump. There is no separate mirror to update — but every place that
*dispatches* on a scheme now has two unhandled cases.

**Bump the pin** in `pyproject.toml` (currently `epaper-dithering==5.0.9`) to the new major.

Then add `SEVEN_COLOR` and `BWGBRY_SPLIT` handling at four sites:

**a. `src/opendisplay/encoding/images.py:88-109` — `encode_image()`**
Currently ends in `raise ValueError(f"Unsupported color scheme: {color_scheme}")`, so both new
schemes raise today.

- `SEVEN_COLOR` → 4bpp with the **identity** index map: `encode_4bpp(image)`, *not*
  `bwgbry_mapping=True`. Verified against `u8Colors_7clr` (`bb_ep.inl:76`), which is the identity
  over logical indices 0-6. Contrast with `u8Colors_spectra` (`bb_ep.inl:81`), which maps 4→0x05
  and 5→0x06 — that is exactly why BWGBRY needs `BWGBRY_MAP = [0,1,2,3,5,6]` and 7-colour does not.
- `BWGBRY_SPLIT` → same 4bpp nibbles and the same `BWGBRY_MAP`, but emitted as **left-half plane
  followed by right-half plane** rather than row-major across the full width. Confirm the exact
  split geometry against the firmware before shipping; the panel has no device framebuffer.

**b. `src/opendisplay/device.py:216` — `_DIRECT_WRITE_PIXELS_PER_BYTE`**
Add `ColorScheme.SEVEN_COLOR: 2` and `ColorScheme.BWGBRY_SPLIT: 2` (both 4bpp). Omitting them
will `KeyError` or silently mis-size a direct write.

**c. `src/opendisplay/device.py:330-360`** — the scheme dispatch chain and the panel-capability
guard. Decide whether 7-colour needs a `PANELS_*` allow-list like `GRAYSCALE_4` has.

**d. `src/opendisplay/display_palettes.py:74` — `DISPLAY_PALETTE_MAP`**
Keyed by `(panel_ic, ColorScheme)`. Add entries for any 7-colour panel IC. Note there is **no
measured palette for SEVEN_COLOR** — the idealized orange is nominal, so output will be
uncalibrated until a panel is photographed.

**Unrelated but worth doing while you are here** (from the audit): `display_palettes.py:74-87`
routes panel 35 to `SPECTRA_7_3_6COLOR` (v1) and never uses `SPECTRA_7_3_6COLOR_V2`. Confirm
against the calibration notes whether v1-over-v2 is deliberate.

---

## 2. OD-App (iOS) — HIGH

`Models/ImageProcessor.swift` carries **hand-written copies of both palette tables**, keyed by the
`ColorScheme` integer (`palettes` at line 62, `measuredPalettes` at line 78). Both stop at key 6.

**The concrete failure:** the lookup at lines 107-108 and 139-140 is

```swift
let palette = (useMeasuredPalette ? measuredPalettes[colorScheme] : nil)
            ?? palettes[colorScheme] ?? palettes[0]!
```

A device reporting `color_scheme = 7` now falls through to `palettes[0]` — **a 7-colour panel is
silently dithered to black and white**, with no error. Separately,
`expectedPackedByteCount(width:height:colorScheme:)` at line 88 has no `case 7`/`case 8`, so its
`default:` returns the 1bpp size, producing a wrong-length upload.

Fixes:
- Add keys `7` and `8` to `palettes`. **Use the app's own index order**, as the existing entries do.
- Add `case 7, 8:` to `expectedPackedByteCount` returning `(pixels + 1) / 2` (4bpp).
- Consider making the `?? palettes[0]!` fallback an explicit error instead of a silent
  monochrome downgrade — that fallback is what turned this into a silent failure.

**Note the app's index order is deliberately NOT the library's.** Scheme 4 in the app is
`black, white, green, blue, red, yellow` (comment: "app order k,w,g,b,r,y"), while the library's
BWGBRY is `black, white, yellow, red, blue, green`. The app's packer maps its own order to
firmware nibbles. Keep that convention — just be aware the two orders differ, so you cannot copy
library palette data without reordering it. The comment at line 71 already flags this.

**Structural risk:** these two tables are a *fifth* hand-written copy of palette data (after Rust,
Python, TypeScript and the generated TypeScript). `epaper-dithering` PR #65 eliminated the
TypeScript duplication by generating it from the Rust `CATALOG`. The same treatment is available
for Swift — see the iOS section below.

---

## 3. Web / opendisplay.org — MEDIUM

`Web/httpdocs/js/vendor/epaper-dithering.min.js` is a **vendored, minified bundle** of the JS
package with the old enum compiled in:

```js
A[A.GRAYSCALE_16=6]="GRAYSCALE_16", A[A.GRAYSCALE_8=7]="GRAYSCALE_8"
```

Any page offering scheme 7 will dither to an 8-step gray ramp for what is now a 7-colour panel.

- Re-vendor from the new release, or better, replace the checked-in bundle with a pinned
  dependency so it cannot silently rot again. A vendored minified bundle is invisible to every
  drift gate in the ecosystem.
- While updating: the audit separately found the site documents NFC on the retired `0x0082`
  opcode and omits the PIPE_WRITE protocol entirely. Different change, same repo.

---

## 4. opendisplay-js — LOW

`package.json:38` pins `"@opendisplay/epaper-dithering": "^2.1.3"`. The package is at **5.0.9**, so
the caret range has not resolved a current version in a long time. Update deliberately and read
the 3.x/4.x/5.x notes — you are crossing several majors, not just this one.

---

## 5. Home_Assistant_Integration — LOW

No direct dependency on `epaper-dithering`; it consumes it through `py-opendisplay`. Bump the
`py-opendisplay` pin in `manifest.json` **after** py-opendisplay ships its update. Nothing else to
do. HA gains 7-colour support for free once the encoder lands.

---

## 6. opendisplay-protocol — documentation

**Audit finding M4 is false and should be struck.** It claimed Python's PIL-based RGBA compositing
and the WASM float-round path produce different palette indices. They are provably identical:

- PIL computes `floor((x + 127) / 255)`; the WASM path computes `floor((x + 127.5) / 255)`.
- These differ only if an integer falls inside a half-open interval of width 0.5 anchored at a
  half-integer — impossible.
- Verified exhaustively: **65,536 cases (256 channel × 256 alpha), 0 differing bytes.**

The related **L2 was real** and is fixed — `composite_rgba` silently dropped a partial trailing
pixel via `rgba.len() / 4`.

Also worth recording: the header's `@external` annotation names
`epaper-dithering packages/rust/core/src/palettes.rs` as its mirror. That mirror is now gated in
CI on every push and daily, via a checkout of the public `davelee98/opendisplay-protocol`. **If the
`ColorScheme` enum in the header changes, epaper-dithering's CI goes red** — that is intended
behaviour, not a flake.

---

## Recommended order

1. **py-opendisplay** — encoders first; nothing downstream can use the new schemes until this lands.
2. **OD-App** — independent of py-opendisplay; fixes a silent mis-render, so treat as urgent.
3. **Web** — re-vendor the bundle.
4. **Home_Assistant_Integration** — pin bump after step 1.
5. **opendisplay-js** — schedule the multi-major upgrade separately.
6. **opendisplay-protocol** — strike M4 whenever convenient.

## Verifying a consumer is correct

The library ships gates you can borrow:

- `scripts/check_enum_parity.py` — asserts `ColorScheme` and `DitherMode` agree across Rust,
  Python and TypeScript. Extend it with a parser for any new mirror you create.
- `scripts/check_enum_parity.py --header <path>` — asserts they match the canonical C header.
- `cargo run --example gen_ts_palettes -- --check` — asserts the TypeScript palettes still match
  the Rust source of truth.

For any consumer holding its own palette table, the test that actually catches an ink swap asserts
**key order and literal RGB values**, not counts or accents. Counting colours passes happily while
red and yellow are transposed.

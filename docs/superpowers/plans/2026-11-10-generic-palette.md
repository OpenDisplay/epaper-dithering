# Generic Palettes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn epaper-dithering into a general "image + palette in, indices out" library: runtime palettes with measured appearance and optional ink identity, anchored mapping and ink snapping, no built-in palettes, no OpenDisplay scheme numbers.

**Architecture:** A new validated `Palette` type in the Rust core (CIELAB appearance normalised to the panel's white, optional `Ink { name, nominal }` per entry, JSON file format) replaces `ColorScheme`, the measured catalog and the canonical-palette plumbing. Internally each `Palette` caches an sRGB rendering of its appearances, so the existing algorithms keep running unchanged on `[u8; 3]` colours; anchoring and snapping are added around them. Python, wasm/JS and iOS bindings each wrap that one type and one `DitherOptions` type.

**Tech Stack:** Rust 2024 (core, rayon, new: serde + serde_json), PyO3/maturin (Python), wasm-bindgen + TypeScript/vitest (JS), C ABI XCFramework (iOS), release-please.

**Spec:** `docs/superpowers/specs/2026-11-10-generic-palette-design.md`

## Global Constraints

- Hard cut in a major release: no compatibility shim for `ColorScheme` or the old call signatures.
- No built-in palettes in any shipped package. Example palette files live in `examples/palettes/` and test fixtures only, are excluded from the crate, wheel and npm package, and are never exposed through an API.
- Inside the library, appearance is CIELAB (D65), normalised so the entry named `white` is L\* 100, a\* = b\* = 0. A `white` entry more than ΔE76 0.5 from `[100, 0, 0]` is rejected with an error naming `from_absolute_lab`.
- 2 to 256 entries. Ink names unique; ink nominals unique. `measured` is optional metadata (default `false`) and no pipeline step reads it.
- Anchoring and snapping are options, both on by default. The snap mask is computed from the original input pixels, before any pre-processing.
- Algorithms (kernels, ordered, dizzy, DBS, OKLab matching with `WAB = 1.5`) keep their behaviour and pinned tests.
- Performance-sensitive code stays in Rust; bindings stay thin.
- Never edit `version` fields by hand; release-please manages versions. Commits use Conventional Commits; a `feat!:` / `BREAKING CHANGE:` footer marks the major bump.
- Nothing outside this repo changes (py-opendisplay stays pinned to the current major).

## Review Focus

1. **Palettes in any order.** `tone_map::compress_dynamic_range` assumes `colors[0]` is black and `colors[1]` is white. A user-supplied palette in another order must still compress correctly: black/white are the darkest/lightest appearances. Pinned in Task 3.
2. **A palette with no named inks.** Anchoring and snapping must be exact no-ops, output identical to `anchor: false, snap: false`. Pinned in Task 4.
3. **Pure ink pixels under pre-processing.** With `exposure: 1.3` and `tone: Auto`, every input pixel equal to an ink's nominal still gets that ink. Today's code loses this. Pinned in Task 5.
4. **Malformed or foreign palette files.** Wrong `format`, unknown `version`, a nominal that isn't `#rrggbb`, a name without a nominal, duplicate names: each gives a specific error, never a panic. Pinned in Task 2.
5. **Bad buffers across bindings.** A pixel buffer whose length isn't `width × height × 3` returns an error from every binding instead of panicking in `ImageBuffer::new`. Pinned in Task 3 (core) and each binding task.

---

### Task 1: Core `Palette` type with validation and conversion

**Files:**
- Create: `packages/rust/core/src/palette.rs`
- Create: `packages/rust/core/src/lab.rs` (CIELAB ↔ XYZ ↔ sRGB, ΔE76, white-point adaptation)
- Modify: `packages/rust/core/src/lib.rs` (add `pub mod palette; pub mod lab;`)

**Interfaces:**
- Produces:
  - `pub struct Lab { pub l: f64, pub a: f64, pub b: f64 }` (`Copy, Debug, PartialEq`)
  - `pub struct Ink { pub name: String, pub nominal: [u8; 3] }`
  - `pub struct PaletteEntry { pub appearance: Lab, pub ink: Option<Ink> }`
  - `pub struct Palette` (private fields `entries`, `measured: bool`, `source: Option<String>`, cached `srgb: Vec<[u8; 3]>`)
  - `Palette::new(entries: Vec<PaletteEntry>, measured: bool) -> Result<Palette, PaletteError>`
  - `Palette::from_absolute_lab(entries: Vec<PaletteEntry>, white_index: usize, measured: bool) -> Result<Palette, PaletteError>`: adapts every appearance from the white entry's white point to D65 (Bradford, via XYZ) so that entry becomes exactly `[100, 0, 0]`.
  - `Palette::from_srgb(colors: Vec<([u8; 3], Option<Ink>)>, measured: bool) -> Result<Palette, PaletteError>`: converts each sRGB colour to Lab; if an ink named `white` exists it normalises through `from_absolute_lab`.
  - `Palette::with_source(self, source: impl Into<String>) -> Palette`
  - Accessors: `entries(&self) -> &[PaletteEntry]`, `len(&self) -> usize`, `measured(&self) -> bool`, `source(&self) -> Option<&str>`, `index_of(&self, name: &str) -> Option<u8>`, `preview_srgb(&self) -> &[[u8; 3]]` (the cached sRGB of each appearance, clamped; also what the algorithms match against), `pub(crate) fn inks(&self) -> impl Iterator<Item = (u8, &Ink)>` (index and ink of every named entry).
  - `pub enum PaletteError { TooFewEntries(usize), TooManyEntries(usize), DuplicateName(String), DuplicateNominal([u8; 3]), WhiteNotNormalised { delta_e: f64 }, WhiteIndexOutOfRange(usize), InvalidJson(String), UnsupportedFormat(String), UnsupportedVersion(u64), InvalidNominal(String) }` with `Display` + `std::error::Error`. `WhiteNotNormalised`'s message contains `from_absolute_lab`.

- [ ] **Step 1: Write failing tests in `palette.rs`'s test module**

```rust
#[test] fn rejects_one_entry() { assert_eq!(Palette::new(vec![entry(50.0, None)], false).unwrap_err(), PaletteError::TooFewEntries(1)); }
#[test] fn rejects_257_entries() { /* 257 unnamed entries */ assert_eq!(err, PaletteError::TooManyEntries(257)); }
#[test] fn rejects_duplicate_name() { /* two inks named "black" */ assert_eq!(err, PaletteError::DuplicateName("black".into())); }
#[test] fn rejects_duplicate_nominal() { /* "black" and "ink" both #000000 */ assert_eq!(err, PaletteError::DuplicateNominal([0, 0, 0])); }
#[test] fn rejects_unnormalised_white() {
    let err = Palette::new(vec![ink(68.26, 8.33, 5.98, "white", [255,255,255]), ink(30.47, 8.7, 4.58, "black", [0,0,0])], true).unwrap_err();
    assert!(matches!(err, PaletteError::WhiteNotNormalised { delta_e } if delta_e > 30.0));
    assert!(err.to_string().contains("from_absolute_lab"));
}
#[test] fn accepts_white_within_half_delta_e() { /* white [99.7, 0.2, -0.2] */ assert!(ok); }
#[test] fn accepts_palette_without_white() { /* two unnamed entries */ assert!(ok); }
#[test] fn from_absolute_lab_puts_white_at_exactly_100() {
    // ODBEEBA6 entries 0 and 1 (absolute, paper white at L* 68.26)
    let p = Palette::from_absolute_lab(vec![entry(30.47, 8.7, 4.58, None), ink(68.26, 8.33, 5.98, "white", [255,255,255])], 1, true).unwrap();
    assert_relative_eq!(p.entries()[1].appearance.l, 100.0, epsilon = 1e-9);
    assert_relative_eq!(p.entries()[1].appearance.a, 0.0, epsilon = 1e-9);
    assert!(p.entries()[0].appearance.l > 30.47); // darker entry scales up with white
}
#[test] fn from_absolute_lab_rejects_bad_white_index() { assert_eq!(err, PaletteError::WhiteIndexOutOfRange(5)); }
#[test] fn from_srgb_roundtrips_preview() {
    let p = Palette::from_srgb(vec![([0,0,0], Some(black())), ([255,255,255], Some(white())), ([200,30,30], None)], false).unwrap();
    assert_eq!(p.preview_srgb(), &[[0,0,0], [255,255,255], [200,30,30]]);
}
#[test] fn index_of_finds_named_ink() { assert_eq!(p.index_of("white"), Some(1)); assert_eq!(p.index_of("red"), None); }
#[test] fn measured_defaults_false_and_source_is_optional() { /* new(.., false) → measured() == false, source() == None */ }
```

`lab.rs` tests: D65 white `[255,255,255]` ↔ `[100,0,0]` within 1e-6; black ↔ `[0,0,0]`; sRGB→Lab→sRGB round-trips every value of a 4096-colour grid exactly after rounding; `delta_e76` of identical colours is 0.

- [ ] **Step 2: Run** `cd packages/rust && cargo test -p epaper-dithering-core palette lab` — Expected: compile failure (types missing).
- [ ] **Step 3: Implement `lab.rs` and `palette.rs`** with the signatures above. Validation order in `new`: entry count, duplicate names, duplicate nominals, white check. Compute and cache `srgb` once in the constructor.
- [ ] **Step 4: Run** the same command — Expected: all PASS. Also `cargo clippy --workspace -- -D warnings` clean.
- [ ] **Step 5: Commit** `feat(core): add validated Palette type with CIELAB appearance and ink identity`

---

### Task 2: Palette file format, and export the current catalog

**Files:**
- Modify: `packages/rust/core/Cargo.toml` (deps `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`; add `examples/palettes` is outside the crate, nothing to exclude)
- Modify: `packages/rust/core/src/palette.rs` (`from_json`, `to_json`)
- Create: `packages/rust/core/examples/export_catalog.rs` (one-off: writes today's `ColorScheme` and measured palettes as files; deleted in Task 3 together with the catalog)
- Create: `examples/palettes/*.json` (repo root; generated output, committed) and `examples/palettes/README.md` (what each file is, where it came from, "not shipped in any package")
- Test: `packages/rust/core/tests/palette_json.rs`

**Interfaces:**
- Consumes: Task 1 `Palette`, `PaletteEntry`, `Ink`, `PaletteError`.
- Produces: `Palette::from_json(s: &str) -> Result<Palette, PaletteError>`, `Palette::to_json(&self) -> String` (pretty-printed, nominals as `#rrggbb` lowercase). File shape exactly as the spec's "Palette file format": top-level `format: "epaper-palette"`, `version: 1`, optional `measured`, optional `source`, `entries: [{ appearance: [L, a, b], name?, nominal? }]`. Unknown top-level keys ignored.

- [ ] **Step 1: Write failing tests in `tests/palette_json.rs`**

```rust
#[test] fn parses_spec_example() { /* the JSON block from the spec */ assert_eq!(p.len(), 3); assert_eq!(p.index_of("black"), Some(1)); assert!(p.measured()); }
#[test] fn round_trips() { assert_eq!(Palette::from_json(&p.to_json()).unwrap(), p); }
#[test] fn ignores_unknown_top_level_keys() { /* add "device": "ODBEEBA6" */ assert!(ok); }
#[test] fn measured_defaults_false() { /* omit "measured" */ assert!(!p.measured()); }
#[test] fn rejects_wrong_format() { assert_eq!(err, PaletteError::UnsupportedFormat("something-else".into())); }
#[test] fn rejects_unknown_version() { assert_eq!(err, PaletteError::UnsupportedVersion(2)); }
#[test] fn rejects_bad_nominal() { /* "nominal": "red" */ assert!(matches!(err, PaletteError::InvalidNominal(s) if s == "red")); }
#[test] fn rejects_name_without_nominal() { assert!(matches!(err, PaletteError::InvalidJson(_))); }
#[test] fn rejects_nominal_without_name() { assert!(matches!(err, PaletteError::InvalidJson(_))); }
#[test] fn malformed_json_is_an_error_not_a_panic() { assert!(matches!(Palette::from_json("{"), Err(PaletteError::InvalidJson(_)))); }
#[test] fn every_example_palette_loads() { /* read every ../../../examples/palettes/*.json */ }
```

- [ ] **Step 2: Run** `cargo test -p epaper-dithering-core --test palette_json` — Expected: FAIL.
- [ ] **Step 3: Implement `from_json`/`to_json`** with private serde structs (don't derive serde on the public types).
- [ ] **Step 4: Write `examples/export_catalog.rs`** that, for every `ColorScheme` and every `measured_palettes::CATALOG` entry, builds a `Palette` via `Palette::from_srgb` naming each colour by the scheme's `color_names()` with nominal = the ideal scheme colour of that name, sets `measured` (true for catalog entries) and `source` ("epaper-dithering 6.x built-in <name>"), and writes `examples/palettes/<snake_name>.json`. File names: ideal schemes `ideal_<scheme>.json`, measured ones by their catalog id (e.g. `spectra_7_3_6color.json`, `solum_bwr.json`).
- [ ] **Step 5: Run** `cargo run -p epaper-dithering-core --example export_catalog`, then `cargo test -p epaper-dithering-core --test palette_json` — Expected: files written, all PASS.
- [ ] **Step 6: Commit** `feat(core): add epaper-palette JSON format and export the built-in catalog as example files`

---

### Task 3: Switch the core onto `Palette`; remove schemes, catalog and canonical plumbing

**Files:**
- Modify: `packages/rust/core/src/lib.rs` (`DitherOptions`, `dither`, pipeline, `needs_preprocess`)
- Modify: `algorithms.rs`, `dizzy.rs`, `dbs.rs`, `tone_map.rs`, `kernels.rs` as needed (take the internal sRGB palette; canonical → snap mask)
- Modify: `packages/rust/core/src/enums.rs` (`DitherMode: FromStr + Display` with snake_case names; remove `TryFrom<u8>`)
- Modify: `packages/rust/core/src/error.rs` (`DitherError`: `InvalidImage { len: usize, width: usize }`, `UnknownDitherMode(String)`; drop `UnknownColorScheme`)
- Modify: `packages/rust/core/src/types.rs` (`ImageBuffer::new -> Result<ImageBuffer, DitherError>`)
- Delete: `palettes.rs`, `measured_palettes.rs`, `examples/export_catalog.rs`, `examples/gen_ts_palettes.rs`
- Modify: `tests/regression.rs`, `benches/dithering.rs`, other `examples/*.rs` (load palettes from `examples/palettes/`)

**Interfaces:**
- Consumes: Task 1/2 `Palette`, `Palette::preview_srgb()`, `Palette::inks()`.
- Produces:
  - `pub struct DitherOptions { pub mode: DitherMode, pub serpentine: bool, pub exposure: f64, pub saturation: f64, pub shadows: f64, pub highlights: f64, pub tone: ToneCompression, pub gamut: GamutCompression, pub dbs: Option<DbsParams>, pub anchor: bool, pub snap: bool }`, `Default` with `anchor: true, snap: true`, all else as today's `DitherConfig::default()`.
  - `pub fn dither(img: &ImageBuffer, palette: &Palette, options: &DitherOptions) -> Vec<u8>`
  - `DitherMode` names: `none, burkes, ordered, floyd_steinberg, atkinson, stucki, sierra, sierra_lite, jarvis_judice_ninke, dizzy`.
  - `pub(crate) fn snap_mask(pixels: &[u8], palette: &Palette) -> Vec<Option<u8>>` (per pixel: index of the ink whose nominal equals the pixel, else `None`).

- [ ] **Step 1: Port the existing canonical tests** in `lib.rs` (`none_uses_canonical_exact_colors_with_measured_palette`, `exact_canonical_colors_bypass_error_diffusion`, `exact_canonical_pixels_are_pinned_inside_mixed_error_diffusion_image`, `…_mixed_ordered_image`, `exact_canonical_pixels_stay_pinned_under_dbs`) to the new API, renamed `snap_*`, using a measured palette whose inks carry the ideal nominals. Replace `exact_bypass_is_skipped_when_preprocessing_is_enabled` with `snap_survives_preprocessing` (exposure 1.3, tone Auto: every pure-ink pixel keeps its ink). Add `snap_false_disables_passthrough` and:

```rust
#[test] fn tone_compression_finds_black_and_white_by_lightness() {
    // same three colours in two orders: [white, red, black] and [black, white, red]
    // tone Fixed(1.0) on a grey ramp → identical outputs after mapping indices by colour
}
#[test] fn dither_mode_names_round_trip() { for m in ALL { assert_eq!(m.to_string().parse::<DitherMode>().unwrap(), m); } }
#[test] fn unknown_dither_mode_name_errors() { assert_eq!("bayer".parse::<DitherMode>().unwrap_err(), DitherError::UnknownDitherMode("bayer".into())); }
#[test] fn image_buffer_rejects_bad_length() { assert_eq!(ImageBuffer::new(&[0; 10], 3).unwrap_err(), DitherError::InvalidImage { len: 10, width: 3 }); }
```

- [ ] **Step 2: Run** `cargo test --workspace` — Expected: compile failures.
- [ ] **Step 3: Implement.** Rename the old internal `Palette` struct to `pub(crate) struct SrgbPalette { colors: Vec<[u8; 3]> }` built from `Palette::preview_srgb()`, so algorithm bodies only change type names. Replace every `canonical_palette: &Palette` / `pin_exact_pixels` parameter with `snap: Option<&[Option<u8>]>`. In `dither`, compute the snap mask from `img.data` before pre-processing (when `options.snap`), keep the all-pixels-snapped fast path, and pass the mask through dispatch and DBS pinning. In `tone_map`, take black = lowest-L\* and white = highest-L\* entry instead of indices 0/1.
- [ ] **Step 4: Update `tests/regression.rs`** to load `examples/palettes/spectra_7_3_6color.json` and `ideal_mono.json`. Run `cargo test --test regression`. If references differ, check whether the cause is palette conversion (sRGB → Lab → sRGB) rather than algorithm change: diff `preview_srgb()` against the old constants. Only then regenerate with `UPDATE_FIXTURES=1 cargo test --test regression` and say so in the commit message.
- [ ] **Step 5: Run** `cargo test --workspace && cargo clippy --workspace -- -D warnings` — Expected: PASS, clean. `grep -rn "ColorScheme\|measured_palettes\|canonical" packages/rust/core/src` — Expected: no hits.
- [ ] **Step 6: Commit** `feat(core)!: dither against runtime Palette; remove ColorScheme, built-in catalog and canonical palettes` with a `BREAKING CHANGE:` footer.

---

### Task 4: Anchored mapping

**Files:**
- Create: `packages/rust/core/src/anchor.rs`
- Modify: `packages/rust/core/src/lib.rs` (pipeline step, `needs_preprocess`, pipeline doc comment)
- Create: `packages/rust/core/examples/anchor_compare.rs` (writes a side-by-side PNG: anchoring off / on, for each fixture photo and both test palettes)

**Interfaces:**
- Consumes: Task 3 `DitherOptions.anchor`, `Palette::inks()`.
- Produces: `pub(crate) fn anchor_map(pixels: &mut [[f64; 3]], palette: &Palette)` operating on linear RGB in place; no-op when the palette has no inks.

The method is the builder's choice (spec: "Left to the builder"). It must be exact at every anchor, continuous, monotonic in lightness along the neutral axis, and a no-op without inks. Decide and document the order relative to `tone` in the `DitherOptions` doc comment and in the CLAUDE.md pipeline line (Task 9).

- [ ] **Step 1: Write failing tests in `anchor.rs`**

```rust
#[test] fn every_ink_nominal_lands_on_its_appearance() { /* BWRY-16 fixture palette: for each ink, anchor_map([nominal_linear]) → Lab within ΔE76 0.01 of appearance */ }
#[test] fn neutral_axis_is_monotonic() { /* 256-step grey ramp → L* strictly non-decreasing after mapping */ }
#[test] fn continuous() { /* neighbouring inputs 1/255 apart move < ΔE76 3 apart, across a 16³ grid */ }
#[test] fn no_inks_is_identity() { /* palette without names: pixels unchanged bit-for-bit */ }
#[test] fn anchor_false_matches_old_behaviour() { /* dither with anchor: false equals Task 3 output on the regression images */ }
```

- [ ] **Step 2: Run** `cargo test -p epaper-dithering-core anchor` — Expected: FAIL.
- [ ] **Step 3: Implement `anchor_map`** and wire it in after `tone`, before `gamut` (or the order chosen, documented).
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS. Run `cargo run --example anchor_compare` and look at the output: photos must not get worse with anchoring on.
- [ ] **Step 5: Commit** `feat(core): anchor ink nominals onto measured appearances before matching`

---

### Task 5: Success-test fixture (dashboard + photo)

**Files:**
- Create: `tools/make_dashboard_fixture.py` (PIL; writes the fixture image once)
- Create: `packages/rust/core/tests/fixtures/images/dashboard_photo.png` (committed output, 800×480)
- Create: `packages/rust/core/tests/fixtures/palettes/bwry16_odbeeba6.json` and `README.md`
- Create: `packages/rust/core/tests/success.rs`

The image: left half white `#ffffff` with `#000000` text at several sizes (anti-aliased), solid `#000000`, `#ff0000`, `#ffff00` rectangles and a 1 px black grid; right half `river.png` scaled to fit.

The fixture palette: `Firmware_Silabs/panels/palette16_photo_ODBEEBA6.json` (16 entries, absolute Lab, paper white = entry 1), converted with `from_absolute_lab(white_index = 1)`. Inks for the fixture only (the data doesn't say which entries are inks): entry 1 `white #ffffff`, entry 14 `black #000000` (darkest, L\* 3.19), entry 2 `yellow #ffff00` (L\* 63.18, b\* 68.64), entry 3 `red #ff0000` (a\* 44.96). The README records origin, conversion and that the ink assignment is a fixture assumption. Second case: `examples/palettes/spectra_7_3_6color.json`.

- [ ] **Step 1: Write `tests/success.rs`**

```rust
fn assert_inks_solid(palette: &Palette, options: &DitherOptions) {
    // for every input pixel equal to an ink's nominal, output == that ink's index; count > 0 for each ink present
}
#[test] fn pure_inks_are_solid_bwry16() { assert_inks_solid(&bwry16(), &DitherOptions::default()); }
#[test] fn pure_inks_are_solid_spectra6() { assert_inks_solid(&spectra6(), &DitherOptions::default()); }
#[test] fn pure_inks_survive_preprocessing() { assert_inks_solid(&bwry16(), &DitherOptions { exposure: 1.3, tone: ToneCompression::Auto, ..Default::default() }); }
#[test] fn solid_rectangles_are_one_index() { /* each rectangle region: exactly one distinct output index */ }
```

- [ ] **Step 2: Run** `cargo test --test success` — Expected: PASS (Tasks 3 and 4 provide the behaviour). If it fails, the defect is in Task 3/4, not here.
- [ ] **Step 3: Extend `examples/anchor_compare.rs`** to render the dashboard fixture with both palettes (preview colours from `preview_srgb()`), and look at the result: text edges clean, photo natural, no colour cast. Record the observation in the commit body.
- [ ] **Step 4: Commit** `test(core): add dashboard + photo success fixture with measured BWRY-16 and Spectra 6 palettes`

---

### Task 6: Python binding

**Files:**
- Modify: `packages/python/src/lib.rs` (pyclass `Palette`, `dither_image`; drop `measured_palettes`, `tone_compress`, `gamut_compress`, `rgb_to_oklab_buffer` from the public module; keep them only if a test needs them, under `_testing`)
- Rewrite: `packages/python/src/epaper_dithering/{__init__.py, core.py, enums.py, _rs.pyi}`
- Delete: `packages/python/src/epaper_dithering/palettes.py`
- Modify: `packages/python/scripts/*` to the new API (moved in Task 9)
- Rewrite: `packages/python/tests/test_palettes.py`, `test_dithering.py`

**Interfaces:**
- Consumes: core `Palette`, `DitherOptions`, `dither`, `DitherMode` names.
- Produces (Python):
  - `PaletteEntry(appearance: tuple[float, float, float], name: str | None = None, nominal: str | tuple[int, int, int] | None = None)` (frozen dataclass)
  - `Palette(entries: Sequence[PaletteEntry], measured: bool = False)`; `Palette.from_json(s: str)`; `Palette.load(path: str | os.PathLike)`; `Palette.from_absolute_lab(entries, white_index: int, measured: bool = False)`; `Palette.from_srgb(colors: Sequence[tuple[tuple[int, int, int], str | None, str | tuple[int, int, int] | None]], measured: bool = False)`; `.to_json() -> str`; `.entries`; `.measured`; `.source`; `.index_of(name) -> int | None`; `.preview_rgb() -> list[tuple[int, int, int]]`; `len()`.
  - `class DitherMode(StrEnum)` with the ten snake_case names.
  - `@dataclass DitherOptions(mode: DitherMode | str = "burkes", serpentine: bool = True, exposure: float = 1.0, saturation: float = 1.0, shadows: float = 0.0, highlights: float = 0.0, tone: float | Literal["auto"] = 0.0, gamut: float | Literal["auto"] = 0.0, dbs: DbsParams | bool | None = None, anchor: bool = True, snap: bool = True)`
  - `dither_image(image: Image.Image, palette: Palette, options: DitherOptions | None = None) -> Image.Image` returning mode `"P"` with `preview_rgb()` as its palette.
  - Errors: `PaletteError(ValueError)` and `ValueError` for bad images/modes.

- [ ] **Step 1: Write failing tests**: `test_load_example_palette` (every `examples/palettes/*.json`), `test_rejects_unnormalised_white_mentions_from_absolute_lab`, `test_duplicate_name_raises_palette_error`, `test_mode_accepts_string_and_enum`, `test_unknown_mode_raises_value_error`, `test_pure_black_text_is_solid` (dashboard fixture + BWRY-16, all `#000000` pixels → black index), `test_options_default_anchor_and_snap_on`, `test_output_is_p_image_with_preview_palette`, `test_rgba_composited_on_white`.
- [ ] **Step 2: Run** `cd packages/python && uv run maturin develop --release && uv run pytest tests/ -v` — Expected: FAIL.
- [ ] **Step 3: Implement.** The pyclass wraps the core `Palette`; no validation logic in Python.
- [ ] **Step 4: Run** the same, plus `uv run prek run --all-files --config .pre-commit-config.yaml` — Expected: PASS, lint clean.
- [ ] **Step 5: Commit** `feat(python)!: Palette and DitherOptions API; remove ColorScheme and built-in palettes`

---

### Task 7: wasm binding and JavaScript package

**Files:**
- Modify: `packages/rust/wasm/src/lib.rs` (`#[wasm_bindgen] struct Palette` with `fromJson`, `new(entries: JsValue)`, `toJson`, `indexOf`, `previewRgb`, `length`; `ditherImage(pixels, width, height, palette, options: JsValue)`)
- Rewrite: `packages/javascript/src/{core.ts, index.ts, types.ts, enums.ts}`
- Delete: `packages/javascript/src/palettes.ts`, `palettes.generated.ts`
- Modify: `packages/javascript/tests/dithering.test.ts`, `tests/fixtures/index.ts`, `demo.html`, `dev.html`
- Modify: `packages/javascript/src/index.ts` (remove `VERSION = '0.1.0'`; export `VERSION` from `package.json` via the build, or drop it)

**Interfaces:**
- Produces (TS): `class Palette { static fromJson(json: string): Palette; constructor(entries: PaletteEntry[], measured?: boolean); toJson(): string; indexOf(name: string): number | undefined; previewRgb(): RGB[]; readonly length: number }`, `interface PaletteEntry { appearance: [number, number, number]; name?: string; nominal?: string }`, `type DitherMode = 'none' | 'burkes' | …`, `interface DitherOptions { mode?: DitherMode; serpentine?; exposure?; saturation?; shadows?; highlights?; tone?: number | 'auto'; gamut?: number | 'auto'; dbs?: DbsParams | boolean; anchor?: boolean; snap?: boolean }`, `ditherImage(image: ImageBuffer, palette: Palette, options?: DitherOptions): PaletteImageBuffer`.

- [ ] **Step 1: Write failing vitest tests** mirroring Task 6's list (JSON load, white error text, duplicate name, unknown mode throws, pure black solid, defaults, bad buffer length throws).
- [ ] **Step 2: Run** `wasm-pack build packages/rust/wasm --target web --out-dir ../../javascript/src/wasm-core && cd packages/javascript && bun run test` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `bun run test && bun run type-check && bun run lint && bun run build` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(js)!: Palette and DitherOptions API; remove ColorScheme and generated palettes`

---

### Task 8: iOS C ABI

**Files:**
- Modify: `packages/rust/ios/src/lib.rs`, `include/epaper_dithering.h`, `ci/link_check.c`, `README.md`

**Interfaces:** handle design is the builder's choice (spec). Required: a palette is created once from JSON (`ed_palette_from_json(const char *json, size_t len, EdPalette **out) -> int32_t`) and freed (`ed_palette_free`); `ed_dither` takes the palette handle and an options struct whose `mode` is a NUL-terminated name and which includes `anchor` and `snap`; the six raw palette arguments are gone; every failure returns a status code, never panics across FFI. Bump `ed_abi_version()`.

- [ ] **Step 1: Port the existing Rust-side FFI tests** (`defaults_mirror_the_core`, `solid_red_maps_to_red_index`, `ffi_matches_core_dither`, `ffi_matches_core_dither_with_dbs`, `rejects_bad_output_len`, `rejects_null_pixels_and_options`, `rejects_unknown_mode`) to the handle API; add `rejects_invalid_palette_json` and `palette_handle_double_free_is_caught_or_documented`.
- [ ] **Step 2: Run** `cd packages/rust/ios && cargo test` — Expected: FAIL.
- [ ] **Step 3: Implement**, update the header and `link_check.c`.
- [ ] **Step 4: Run** `cargo test` and `./build-xcframework.sh` — Expected: PASS, framework builds.
- [ ] **Step 5: Commit** `feat(ios)!: palette handle and named modes in the C ABI`

---

### Task 9: Repo cleanup and CI

**Files:**
- Delete: `scripts/check_enum_parity.py`, `.github/workflows/enum-parity-header.yml`, `docs/MIGRATION-colorscheme-protocol-v2.md`, `packages/python/colors.txt`
- Modify: `.github/workflows/{rust-test,python-test,javascript-test}.yml` (remove the enum-parity and `gen_ts_palettes --check` steps)
- Modify: `AGENTS.md` (single source; update for the new model), `CLAUDE.md` (replace content with a one-line pointer to `AGENTS.md`)
- Create: `tools/` (move `packages/python/scripts/*`, `docs/examples/generate.py`; Rust examples stay in `core/examples/` because cargo needs them there — say so in `tools/README.md`)
- Add: `packages/rust/core/examples/wab_sweep.rs`, `wab_compare.rs` (currently untracked; port to the new API)
- Decide and apply: commit `packages/rust/Cargo.lock` and `packages/rust/wasm/Cargo.lock`, or add them to `.gitignore` (choose one, state why in the commit)
- Modify: `packages/python/pyproject.toml`, `packages/javascript/package.json`, `packages/rust/core/Cargo.toml` so `examples/palettes/` and `tests/fixtures/` are excluded from published artifacts

- [ ] **Step 1:** Make the changes above. In `AGENTS.md`: remove the "Hand-mirrored enums" and "Generated palettes" sections; add the palette model, the white-normalisation rule, anchoring/snapping and the final pipeline order; keep the colour-science and release sections.
- [ ] **Step 2: Verify** `cargo package -p epaper-dithering-core --list`, `uv build` + list the wheel, `npm pack --dry-run` — Expected: no `examples/palettes` or fixture files in any of them. `grep -rn "check_enum_parity\|gen_ts_palettes\|ColorScheme" --include=*.yml --include=*.md --include=*.py --include=*.ts --include=*.rs .` — Expected: only CHANGELOG history hits.
- [ ] **Step 3: Run** the full test commands from Tasks 3, 6 and 7 — Expected: PASS.
- [ ] **Step 4: Commit** `chore: remove scheme parity tooling, consolidate tools, single agent instructions file`

---

### Task 10: Documentation

**Files:**
- Rewrite: `README.md`, `packages/python/README.md`, `packages/javascript/README.md`, `packages/rust/core/README.md`, `packages/python/docs/CALIBRATION.md`
- Modify: `docs/index.html` and `docs/examples/` output if they reference schemes

Each README covers, with runnable snippets: loading a palette file (`examples/palettes/` as the starting point), building one from measurements (`from_absolute_lab`, what "normalised to white" means and why the library rejects otherwise), how named inks behave (anchoring, snapping, "a requested ink stays that ink"), `DitherOptions` with the pipeline order, and a migration section from 6.x (`ColorScheme.BWRY` → `Palette.load("…/ideal_bwry.json")`, measured constants → files, `dither_with_canonical` → inks with nominals, mode integers → names).

- [ ] **Step 1:** Write the docs. Run every snippet (Python: `uv run python` on each; JS: a scratch vitest; Rust: doc-tests in `core/README.md` via `#![doc = include_str!]` if already used, else a test).
- [ ] **Step 2: Commit** `docs: rewrite READMEs around runtime palettes, with a 6.x migration guide`

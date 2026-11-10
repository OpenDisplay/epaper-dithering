# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Dithering algorithms for e-paper/e-ink displays. All logic lives in a single Rust core (`packages/rust/core/`); the Python package (PyO3/maturin) and JavaScript package (wasm-bindgen/wasm-pack) are thin wrappers around it. Performance-sensitive code goes in Rust, never in the wrappers.

## Commands

### Rust (`packages/rust/`)

```bash
cd packages/rust
cargo test --workspace                 # all tests
cargo test --workspace <name>          # single test by substring
cargo clippy --workspace -- -D warnings
```

The workspace deliberately excludes `wasm` and `ios` (see comment in `packages/rust/Cargo.toml`, issue #40): the Python sdist embeds this workspace manifest, and listing members it doesn't ship breaks downstream cargo vendoring. Do not add them back as members.

### Python (`packages/python/`)

Requires a Rust toolchain (rustup).

```bash
cd packages/python
uv sync --all-extras
uv run maturin develop --release       # rebuild the extension after any Rust change
uv run pytest tests/ -v
uv run pytest tests/test_file.py::test_name -v   # single test
uv run prek run --all-files --config .pre-commit-config.yaml   # lint (ruff, mypy, pylint via prek)
```

### JavaScript (`packages/javascript/`)

The WASM bundle must be built into `src/wasm-core/` before anything else works. Use `--target web`: `core.ts` inlines the `.wasm` bytes (tsup binary loader) and initialises with the generated `initSync`, so it never touches wasm-bindgen's internal glue.

```bash
# from repo root; requires wasm-pack and the wasm32-unknown-unknown target
wasm-pack build packages/rust/wasm --target web --out-dir ../../javascript/src/wasm-core

cd packages/javascript
bun install
bun run test                           # vitest
bunx vitest run tests/file.test.ts     # single test file
bun run type-check
bun run lint
bun run build                          # tsup
```

### Dev tools (`tools/`)

Python scripts for visual comparison and docs images (`compare.py`, `pipeline.py`,
`generate_patches.py`, `generate_docs_examples.py`, `make_dashboard_fixture.py`). Run them with the
Python package's environment, e.g. `uv run --project packages/python python tools/compare.py`.
Rust tuning harnesses stay in `packages/rust/core/examples/` because cargo needs them there
(`anchor_compare`, `wab_sweep`, `wab_compare`, `dbs_compare`, `dizzy_compare`, `dither`).

## Architecture

### One core, three bindings

- `packages/rust/core/` — all algorithms: error-diffusion kernels (`kernels.rs`, `algorithms.rs`), ordered dither, dizzy dither (`dizzy.rs`), OKLab color matching (`color_space_lab.rs`), tone/gamut compression (`tone_map.rs`), palettes (`palette.rs`, `palette_file.rs`, `lab.rs`), anchoring (`anchor.rs`).
- `packages/python/src/lib.rs` — PyO3 module (`epaper_dithering._rs`); `src/epaper_dithering/` is the pure-Python wrapper (Pillow integration, enums, `.pyi` stubs).
- `packages/rust/wasm/` — wasm-bindgen bindings; `packages/javascript/src/` is the TS wrapper.
- `packages/rust/ios/` — standalone XCFramework build (`build-xcframework.sh`), not part of the workspace.

The pre-processing pipeline order is fixed and documented on `DitherConfig` in `core/src/lib.rs`: `exposure → saturation → shadows/highlights → anchor → tone → gamut → match + diffuse → dbs`; ink snapping is decided on the original input. Each step is a no-op at its identity value.

### Palettes are runtime data

There are no built-in palettes and no colour-scheme enum. A `Palette` (`core/src/palette.rs`) is an
ordered list of entries; each has an `appearance` (CIELAB D65, **normalised so the entry named
`white` is L\* 100, a\* = b\* = 0**; constructors reject anything else, with an error naming
`from_absolute_lab`) and an optional `Ink { name, nominal }`. Palettes are loaded from versioned
`epaper-palette` JSON files (`core/src/palette_file.rs`), built from absolute measurements
(`from_absolute_lab`) or from sRGB (`from_srgb`). Example files live in `examples/palettes/`; they
are never shipped in the crate, wheel or npm package and no API returns them.

- Named inks drive two behaviours, both on by default: **anchoring** (`core/src/anchor.rs`: black-point
  compensation between the darkest and lightest ink plus a damped local correction, in CIELAB, exact
  at every ink) and **snapping** (a pixel exactly equal to an ink's nominal takes that ink, decided on
  the original input, pinned through diffusion and DBS).
- Algorithms run on `SrgbPalette` (the clamped sRGB of each appearance, `#[doc(hidden)]`); the
  supported API is `Palette` + `dither()`.
- `DitherMode` is named across every binding (`"floyd_steinberg"` …); there is no integer wire value
  and no parity script. Nothing in this library knows about OpenDisplay or any firmware.

### Color science decisions (pinned by tests — don't "simplify" them away)

- Pixel↔palette matching uses weighted Cartesian OKLab distance (`dist² = dL² + Wab²·(da² + db²)`, `Wab = 1.5`) in `color_space_lab.rs`. LCH-weighted distance was rejected for an achromatic-attractor bug; hue preservation matters more than lightness because error diffusion compensates spatially for lightness but not hue. Re-tuning harness: `core/examples/wab_sweep.rs`.
- Ordered dither applies the Bayer threshold in sRGB-fraction space, not linear (pinned by `ordered_dither_activity_is_perceptually_uniform`).
- Palette appearances are normalised to the panel's white. The 6.x "measured" constants were not, so the exported example files are brighter than 6.x.
- Anchoring works in CIELAB, not OKLab: OKLab's cube root has no linear toe, so lifting black turned single sRGB steps near black into visible jumps.

## Releases and versioning

Release-please CI manages versions and changelogs from conventional-commit messages. Never edit `version` fields in `Cargo.toml`, `pyproject.toml`, or `package.json` manually. Use conventional commit messages (`feat:`, `fix:`, `chore:` …).

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

The WASM bundle must be built into `src/wasm-core/` before anything else works:

```bash
# from repo root; requires wasm-pack and the wasm32-unknown-unknown target
wasm-pack build packages/rust/wasm --target bundler --out-dir ../../javascript/src/wasm-core

cd packages/javascript
bun install
bun run test                           # vitest
bunx vitest run tests/file.test.ts     # single test file
bun run type-check
bun run lint
bun run build                          # tsup
```

### Cross-language gates (run before pushing enum or palette changes)

```bash
python3 scripts/check_enum_parity.py                      # enum mirrors agree across Rust/Python/TS
cd packages/rust && cargo run --example gen_ts_palettes -- --check   # generated TS palettes not drifted
```

## Architecture

### One core, three bindings

- `packages/rust/core/` — all algorithms: error-diffusion kernels (`kernels.rs`, `algorithms.rs`), ordered dither, dizzy dither (`dizzy.rs`), OKLab color matching (`color_space_lab.rs`), tone/gamut compression (`tone_map.rs`), palettes (`palettes.rs`, `measured_palettes.rs`).
- `packages/python/src/lib.rs` — PyO3 module (`epaper_dithering._rs`); `src/epaper_dithering/` is the pure-Python wrapper (Pillow integration, enums, `.pyi` stubs).
- `packages/rust/wasm/` — wasm-bindgen bindings; `packages/javascript/src/` is the TS wrapper.
- `packages/rust/ios/` — standalone XCFramework build (`build-xcframework.sh`), not part of the workspace.

The pre-processing pipeline order is fixed and documented on `DitherConfig` in `core/src/lib.rs`: `exposure → saturation → shadows/highlights → tone → gamut → dither`. Each step is a no-op at its identity value.

### Hand-mirrored enums are a wire contract

`ColorScheme` and `DitherMode` are declared independently (no codegen) in six files: `core/src/palettes.rs`, `core/src/enums.rs`, `python/.../palettes.py`, `python/.../enums.py`, `javascript/src/palettes.ts`, `javascript/src/enums.ts`. `ColorScheme` integer values are the firmware wire contract, canonically defined by `enum ColorScheme` in the `opendisplay-protocol` repo's `opendisplay_structs.h`, which names `core/src/palettes.rs` as its designated mirror.

- `scripts/check_enum_parity.py` (layer 1) checks the three in-repo mirrors against each other; CI also runs layer 2 against the canonical header daily.
- When adding or changing an enum member, update all six files.
- If the header-parity CI job goes red, mirror the upstream header change here — never edit values just to make the check pass.
- Some values are library-local and intentionally absent from the header (e.g. `GRAYSCALE_8` = 9); `DitherMode` has no wire representation at all.

### Generated palettes

`packages/javascript/src/palettes.generated.ts` is generated from the Rust palette definitions by `cargo run --example gen_ts_palettes` (run from `packages/rust/`). Never edit it by hand; after changing palettes in Rust, regenerate it. CI fails on drift via the `--check` flag.

### Color science decisions (pinned by tests — don't "simplify" them away)

- Pixel↔palette matching uses weighted Cartesian OKLab distance (`dist² = dL² + Wab²·(da² + db²)`, `Wab = 1.5`) in `color_space_lab.rs`. LCH-weighted distance was rejected for an achromatic-attractor bug; hue preservation matters more than lightness because error diffusion compensates spatially for lightness but not hue. Re-tuning harness: `core/examples/wab_sweep.rs`.
- Ordered dither applies the Bayer threshold in sRGB-fraction space, not linear (pinned by `ordered_dither_activity_is_perceptually_uniform`).
- Measured palettes are calibrated per-display values normalized against the paper white point; they differ substantially from pure RGB.

## Releases and versioning

Release-please CI manages versions and changelogs from conventional-commit messages. Never edit `version` fields in `Cargo.toml`, `pyproject.toml`, or `package.json` manually. Use conventional commit messages (`feat:`, `fix:`, `chore:` …).

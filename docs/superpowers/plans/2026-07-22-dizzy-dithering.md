# Dizzy Dithering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add "dizzy dithering" — error diffusion with pseudo-random traversal instead of raster scan — as `DitherMode = 9` across the Rust core, Python and JavaScript bindings.

**Architecture:** A new `dizzy.rs` module in the Rust core reuses the existing OKLab matching, sRGB error accumulation and linear LUT from `algorithms.rs`, replacing only the traversal order and the error-distribution rule. Traversal is a stateless bijective permutation (multiply-by-odd + XOR, five rounds) so no shuffled index array is allocated and output is byte-identical across all three language surfaces. Error goes only to not-yet-quantized neighbours, weighted 1.0 orthogonal / 0.1 diagonal and normalized per pixel.

**Tech Stack:** Rust 2024 edition, PyO3 0.28.2 (`maturin`), wasm-bindgen + `wasm-pack`, TypeScript + vitest + bun, criterion.

**Spec:** `docs/superpowers/specs/2026-07-22-dizzy-dithering-design.md`

## Global Constraints

- **`DitherMode` integer values are a published contract.** `Dizzy = 9`. Never change an existing value.
- **NEVER rebaseline the 12 visual-regression fixtures** in `packages/rust/core/tests/fixtures/references/`. Adding a mode must not change output for any existing mode. `UPDATE_FIXTURES=1` exists but must only ever be used to *add* the new dizzy references in Task 5, never to overwrite an existing one.
- **The permutation constants are frozen once merged.** Changing `ODD` or `XOR` changes every image this mode ever produces.
- Do NOT edit version fields in `Cargo.toml` / `pyproject.toml` / `package.json` — release-please owns them.
- Do NOT relocate `pub enum ColorScheme` (`palettes.rs`) or `pub enum DitherMode` (`enums.rs`). `scripts/check_enum_parity.py` finds them by hardcoded path.
- `/Users/gabriel/Developer/OpenDisplay/opendisplay-protocol` is a separate READ-ONLY repo.
- Never `git add -A` / `git add .`. These files are pre-existing untracked user WIP: `packages/rust/Cargo.lock`, `packages/rust/wasm/Cargo.lock`, `packages/rust/core/examples/wab_compare.rs`, `packages/rust/core/examples/wab_sweep.rs`.
- After changing the WASM crate, rebuild before running JS tests or you test a stale binary:
  `wasm-pack build packages/rust/wasm --target bundler --out-dir ../../javascript/src/wasm-core` (from repo root).
- Before `pytest`, run `maturin develop` in `packages/python` or a stale `_rs.so` gives spurious failures.
- `cargo clippy --workspace --all-targets` fails on a PRE-EXISTING `type_complexity` warning at `packages/rust/core/src/palettes.rs:259`. Use `cargo clippy --workspace -- -D warnings` (no `--all-targets`).

---

### Task 1: The permutation walk

The load-bearing correctness unit. If the walk is not a bijection, pixels are silently skipped or quantized twice and **the output still looks plausible** — no image-quality test would catch it. Build and prove it in isolation, before any dithering exists.

**Files:**
- Create: `packages/rust/core/src/dizzy.rs`
- Modify: `packages/rust/core/src/lib.rs` (add `pub mod dizzy;` beside the existing `pub mod composite;`)

**Interfaces:**
- Consumes: nothing.
- Produces: `pub(crate) fn dizzy_order(n: usize) -> impl Iterator<Item = usize>` — yields every index in `0..n` exactly once, in pseudo-random order.

- [ ] **Step 1: Write the failing test**

Create `packages/rust/core/src/dizzy.rs` containing only:

```rust
//! Dizzy dithering: error diffusion with pseudo-random traversal.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_visits_every_index_exactly_once() {
        // Powers of two, 2^k+1 (worst-case rejection), primes, and degenerate shapes.
        for n in [1usize, 2, 3, 4, 5, 7, 8, 9, 16, 17, 31, 64, 100, 255, 256, 257, 1000, 4096] {
            let mut seen = vec![0u32; n];
            let mut count = 0usize;
            for p in dizzy_order(n) {
                assert!(p < n, "n={n}: yielded out-of-range index {p}");
                seen[p] += 1;
                count += 1;
            }
            assert_eq!(count, n, "n={n}: walk yielded {count} indices, expected {n}");
            assert!(
                seen.iter().all(|&c| c == 1),
                "n={n}: some index was visited {:?} times, expected exactly once each",
                seen.iter().max()
            );
        }
    }

    #[test]
    fn walk_is_not_the_identity() {
        // A permutation that happened to be the identity would pass the bijection
        // test while making this mode a plain raster scan.
        let order: Vec<usize> = dizzy_order(256).collect();
        let identity: Vec<usize> = (0..256).collect();
        assert_ne!(order, identity, "walk degenerated into raster order");
    }
}
```

Add `pub mod dizzy;` to `packages/rust/core/src/lib.rs` in the module list at the top (alongside `pub mod composite;`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/rust && cargo test -p epaper-dithering-core dizzy`
Expected: FAIL to compile — `cannot find function 'dizzy_order' in this scope`

- [ ] **Step 3: Write minimal implementation**

Insert above the `#[cfg(test)]` block in `packages/rust/core/src/dizzy.rs`:

```rust
// ── Traversal ─────────────────────────────────────────────────────────────────
//
// A stateless bijective permutation of `0..2^bits`, so the walk needs no shuffled
// index array. Multiplication by an odd number is invertible modulo 2^k (odd
// numbers are units in that ring) and XOR by a constant is self-inverse, so five
// rounds of (multiply, mask, xor) compose to a bijection. Indices >= n are skipped.
//
// FROZEN: changing either table changes every image this mode has ever produced.
const ODD: [u64; 5] = [0x2545_F491, 0x9E37_79B1, 0x85EB_CA6B, 0xC2B2_AE35, 0x27D4_EB2F];
const XOR: [u64; 5] = [0x1656_67B1, 0xD3A2_646C, 0xFD70_46C5, 0xB55A_4F09, 0x1B87_3593];

/// Smallest `bits` such that `2^bits >= n`. `bits_for(1) == 0`.
fn bits_for(n: usize) -> u32 {
    debug_assert!(n > 0, "bits_for requires a non-empty image");
    usize::BITS - (n - 1).leading_zeros()
}

fn permute(i: u64, mask: u64) -> u64 {
    let mut p = i;
    for r in 0..ODD.len() {
        p = p.wrapping_mul(ODD[r]) & mask;
        p ^= XOR[r] & mask;
    }
    p
}

/// Yields every index in `0..n` exactly once, in pseudo-random order.
pub(crate) fn dizzy_order(n: usize) -> impl Iterator<Item = usize> {
    let bits = bits_for(n);
    let mask = if bits >= u64::BITS { u64::MAX } else { (1u64 << bits) - 1 };
    (0..=mask).filter_map(move |i| {
        let p = permute(i, mask) as usize;
        (p < n).then_some(p)
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/rust && cargo test -p epaper-dithering-core dizzy`
Expected: PASS — `test dizzy::tests::walk_visits_every_index_exactly_once ... ok` and `walk_is_not_the_identity ... ok`

- [ ] **Step 5: Verify no existing behaviour moved**

Run: `cd packages/rust && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all pass, including the 12 fixture-regression tests.

- [ ] **Step 6: Commit**

```bash
git add packages/rust/core/src/dizzy.rs packages/rust/core/src/lib.rs
git commit -m "feat(core): add bijective permutation walk for dizzy dithering"
```

---

### Task 2: The dizzy dither core

**Files:**
- Modify: `packages/rust/core/src/dizzy.rs`
- Modify: `packages/rust/core/src/algorithms.rs` — change `fn build_palette_lab` (line 15) and `fn exact_palette_index` (line 176) to `pub(crate) fn`. Change nothing else.

**Interfaces:**
- Consumes: `dizzy_order(n)` from Task 1; `algorithms::build_palette_lab`, `algorithms::exact_palette_index`; `color_space_lab::{WAB, match_pixel_oklab, rgb_to_oklab}`; `color_space::srgb_channel_to_linear`.
- Produces:
  - `pub fn dizzy_dither(pixels: &[u8], width: usize, height: usize, palette: &Palette) -> Vec<u8>`
  - `pub fn dizzy_dither_with_canonical(pixels: &[u8], width: usize, height: usize, palette: &Palette, canonical: &Palette) -> Vec<u8>`

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `packages/rust/core/src/dizzy.rs`:

```rust
    use crate::palettes::ColorScheme;

    fn gray_image(value: u8, count: usize) -> Vec<u8> {
        std::iter::repeat_n([value, value, value], count).flatten().collect()
    }

    #[test]
    fn output_is_deterministic() {
        let img = gray_image(128, 64);
        let p = ColorScheme::Bwr.palette();
        let a = dizzy_dither(&img, 8, 8, p);
        let b = dizzy_dither(&img, 8, 8, p);
        assert_eq!(a, b, "dizzy must be deterministic for identical input");
    }

    #[test]
    fn output_length_and_indices_are_in_range() {
        let img = gray_image(90, 96);
        let p = ColorScheme::Bwgbry.palette();
        let out = dizzy_dither(&img, 12, 8, p);
        assert_eq!(out.len(), 96);
        assert!(out.iter().all(|&i| (i as usize) < p.colors.len()));
    }

    #[test]
    fn flat_midgray_uses_more_than_one_ink() {
        // A flat field must dither, not collapse to a single nearest colour.
        let img = gray_image(128, 1024);
        let out = dizzy_dither(&img, 32, 32, ColorScheme::Mono.palette());
        let black = out.iter().filter(|&&i| i == 0).count();
        assert!(black > 0 && black < 1024, "expected a mix of inks, got {black}/1024 black");
    }

    #[test]
    fn error_is_conserved_while_neighbours_remain() {
        // Every pixel's error must be fully handed on (shares sum to 1.0) unless
        // every neighbour is already quantized. Verified here on the weighting
        // itself: for any non-empty subset of the 8 neighbours, the normalized
        // shares must sum to 1.
        for mask in 1u32..256 {
            let mut denom = 0.0;
            for (bit, (_, _, w)) in NEIGHBORS.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    denom += w;
                }
            }
            let total: f64 = NEIGHBORS
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, (_, _, w))| w / denom)
                .sum();
            assert!(
                (total - 1.0).abs() < 1e-12,
                "mask {mask:08b}: shares summed to {total}, expected 1.0"
            );
        }
    }

    #[test]
    fn exact_canonical_pixels_are_pinned() {
        // Mirrors algorithms.rs's equivalent test for raster error diffusion.
        let mut image = gray_image(128, 8);
        image[0..3].copy_from_slice(&[0, 255, 0]);   // pure green -> index 5 in BWGBRY
        image[9..12].copy_from_slice(&[0, 255, 0]);
        let out = dizzy_dither_with_canonical(
            &image, 4, 2,
            &crate::measured_palettes::SPECTRA_7_3_6COLOR,
            ColorScheme::Bwgbry.palette(),
        );
        assert_eq!(out[0], 5, "exact green at pixel 0 should be pinned");
        assert_eq!(out[3], 5, "exact green at pixel 3 should be pinned");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/rust && cargo test -p epaper-dithering-core dizzy`
Expected: FAIL to compile — `cannot find function 'dizzy_dither' in this scope`

- [ ] **Step 3: Make the two helpers crate-visible**

In `packages/rust/core/src/algorithms.rs`, change exactly two signatures:

```rust
pub(crate) fn build_palette_lab(palette: &Palette) -> (Vec<[f64; 3]>, PaletteLab) {
```

```rust
pub(crate) fn exact_palette_index(rgb: &[u8], palette: &Palette) -> Option<u8> {
```

- [ ] **Step 4: Write the implementation**

Add to `packages/rust/core/src/dizzy.rs`, above the test module:

```rust
use crate::algorithms::{build_palette_lab, exact_palette_index};
use crate::color_space::srgb_channel_to_linear;
use crate::color_space_lab::{match_pixel_oklab, rgb_to_oklab, WAB};
use crate::palettes::Palette;

/// 8-neighbourhood with the source article's 10:1 orthogonal:diagonal weighting.
const NEIGHBORS: [(i64, i64, f64); 8] = [
    (0, -1, 1.0), (-1, 0, 1.0), (1, 0, 1.0), (0, 1, 1.0),
    (-1, -1, 0.1), (1, -1, 0.1), (-1, 1, 0.1), (1, 1, 0.1),
];

/// Dither with pseudo-random traversal, diffusing error only to unquantized neighbours.
pub fn dizzy_dither(pixels: &[u8], width: usize, height: usize, palette: &Palette) -> Vec<u8> {
    dizzy_dither_impl(pixels, width, height, palette, None)
}

/// As [`dizzy_dither`], but pixels exactly matching a `canonical` ink pass through
/// unchanged and are excluded from error distribution.
pub fn dizzy_dither_with_canonical(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    canonical: &Palette,
) -> Vec<u8> {
    dizzy_dither_impl(pixels, width, height, palette, Some(canonical))
}

fn dizzy_dither_impl(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    canonical_palette: Option<&Palette>,
) -> Vec<u8> {
    let (_palette_linear, palette_lab) = build_palette_lab(palette);
    let palette_srgb_f: Vec<[f64; 3]> = palette
        .colors
        .iter()
        .map(|&[r, g, b]| [r as f64, g as f64, b as f64])
        .collect();

    // Working buffer in sRGB float space [0, 255]; accumulates diffused error.
    let mut buf: Vec<f64> = pixels.iter().map(|&v| v as f64).collect();
    // LUT: u8 sRGB -> linear f64 (avoids powf per pixel in the inner loop)
    let lut: Vec<f64> = (0u8..=255).map(srgb_channel_to_linear).collect();

    let n = width * height;
    let mut output = vec![0u8; n];
    let mut processed = vec![false; n];

    // Pre-pass: pin exact canonical pixels and mark them processed BEFORE the walk,
    // so no neighbour spends error on a pixel that will ignore it. This differs
    // deliberately from the raster implementation, which lets pinned pixels
    // accumulate error that is then discarded.
    if let Some(canonical) = canonical_palette {
        for i in 0..n {
            if let Some(exact) = exact_palette_index(&pixels[i * 3..i * 3 + 3], canonical) {
                output[i] = exact;
                processed[i] = true;
            }
        }
    }

    for i in dizzy_order(n) {
        if processed[i] {
            continue;
        }
        let idx = i * 3;
        let rs = buf[idx].clamp(0.0, 255.0);
        let gs = buf[idx + 1].clamp(0.0, 255.0);
        let bs = buf[idx + 2].clamp(0.0, 255.0);

        let pixel_lab = rgb_to_oklab(
            lut[rs.round() as usize],
            lut[gs.round() as usize],
            lut[bs.round() as usize],
        );
        let best = match_pixel_oklab(pixel_lab, &palette_lab, WAB);

        output[i] = best as u8;
        processed[i] = true;

        let err = [
            rs - palette_srgb_f[best][0],
            gs - palette_srgb_f[best][1],
            bs - palette_srgb_f[best][2],
        ];

        let x = (i % width) as i64;
        let y = (i / width) as i64;

        // Pass 1: total weight of the still-unquantized neighbours.
        let mut denom = 0.0;
        for (dx, dy, w) in NEIGHBORS {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && nx < width as i64 && ny >= 0 && ny < height as i64 {
                let ni = ny as usize * width + nx as usize;
                if !processed[ni] {
                    denom += w;
                }
            }
        }

        // Every neighbour is already quantized, so this pixel's error has nowhere to
        // go and is dropped. That is inherent to the algorithm: do NOT "fix" it by
        // widening the neighbourhood or diffusing back into processed pixels --
        // either changes the output and defeats the point of the traversal order.
        if denom == 0.0 {
            continue;
        }

        // Pass 2: distribute, normalized so the full error is conserved.
        for (dx, dy, w) in NEIGHBORS {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && nx < width as i64 && ny >= 0 && ny < height as i64 {
                let ni = ny as usize * width + nx as usize;
                if !processed[ni] {
                    let share = w / denom;
                    buf[ni * 3] += err[0] * share;
                    buf[ni * 3 + 1] += err[1] * share;
                    buf[ni * 3 + 2] += err[2] * share;
                }
            }
        }
    }

    output
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd packages/rust && cargo test -p epaper-dithering-core dizzy`
Expected: PASS — 6 dizzy tests.

- [ ] **Step 6: Verify nothing else changed**

Run: `cd packages/rust && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all pass; the 12 fixture-regression tests unchanged.

- [ ] **Step 7: Commit**

```bash
git add packages/rust/core/src/dizzy.rs packages/rust/core/src/algorithms.rs
git commit -m "feat(core): implement dizzy dithering error distribution"
```

---

### Task 3: Wire into `DitherMode` and `dispatch` (Rust)

`dispatch()` is an exhaustive match with **no wildcard arm** (PR #66), so adding the variant makes the crate fail to compile until every path handles it. That is the intended safety property — let the compiler drive this task.

**Files:**
- Modify: `packages/rust/core/src/enums.rs`
- Modify: `packages/rust/core/src/lib.rs` (`dispatch`)
- Modify: `packages/rust/core/examples/dither.rs`

**Interfaces:**
- Consumes: `dizzy::dizzy_dither`, `dizzy::dizzy_dither_with_canonical` from Task 2.
- Produces: `DitherMode::Dizzy` (discriminant 9), reachable via `dither()` / `dither_with_canonical()`.

- [ ] **Step 1: Write the failing test**

Add to the `mod tests` block in `packages/rust/core/src/enums.rs`, or create one if absent:

```rust
    #[test]
    fn dizzy_is_mode_nine_and_has_no_kernel() {
        assert_eq!(DitherMode::Dizzy as u8, 9);
        assert_eq!(DitherMode::try_from(9u8), Ok(DitherMode::Dizzy));
        assert!(DitherMode::Dizzy.kernel().is_none(), "dizzy has no fixed kernel");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/rust && cargo test -p epaper-dithering-core dizzy_is_mode_nine`
Expected: FAIL to compile — `no variant named 'Dizzy' found for enum 'DitherMode'`

- [ ] **Step 3: Add the variant**

In `packages/rust/core/src/enums.rs`, add to the enum after `JarvisJudiceNinke = 8,`:

```rust
    /// Error diffusion with pseudo-random traversal instead of a raster scan,
    /// diffusing error only to not-yet-quantized neighbours. Produces no
    /// directional structure. `serpentine` is ignored: there is no scan direction.
    Dizzy          = 9,
```

Add to the `TryFrom<u8>` match, after the `8 => ...` arm:

```rust
            9 => Ok(DitherMode::Dizzy),
```

Add `Dizzy` to the kernel-less arm of `kernel()`:

```rust
            DitherMode::None | DitherMode::Ordered | DitherMode::Dizzy => None,
```

- [ ] **Step 4: Fix the compile error in `dispatch`**

`cargo build` now fails with `non-exhaustive patterns: 'DitherMode::Dizzy' not covered` at `packages/rust/core/src/lib.rs`. Add these two arms to `dispatch`, immediately after the `DitherMode::Ordered` arms:

```rust
        DitherMode::Dizzy if pin_exact_pixels => {
            dizzy::dizzy_dither_with_canonical(img.data, img.width, img.height, p, canonical)
        }
        DitherMode::Dizzy => dizzy::dizzy_dither(img.data, img.width, img.height, p),
```

Add `use crate::dizzy;` to the imports at the top of `lib.rs` if not already present.

- [ ] **Step 5: Add to the example's mode table**

In `packages/rust/core/examples/dither.rs`, add to the `match mode_name` block (around line 63):

```rust
        "dizzy"             => DitherMode::Dizzy,
```

And update the comment listing modes (line 13) to end with `, jjn, dizzy`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd packages/rust && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all pass. The 12 fixture-regression tests must be unchanged — adding a mode changes no existing output.

- [ ] **Step 7: Smoke-test end to end**

Run: `cd packages/rust && cargo run --example dither core/tests/fixtures/images/cat.png /tmp/dizzy.png bwgbry dizzy`
Expected: exits 0 and writes `/tmp/dizzy.png`. Open it — it should look dithered with no visible directional hatching.

- [ ] **Step 8: Commit**

```bash
git add packages/rust/core/src/enums.rs packages/rust/core/src/lib.rs packages/rust/core/examples/dither.rs
git commit -m "feat(core): expose dizzy dithering as DitherMode 9"
```

---

### Task 4: Python and TypeScript surfaces

`scripts/check_enum_parity.py` asserts `DitherMode` agrees across Rust, Python and TypeScript, so all three must land together — this is not optional.

**Note on the cross-language expected values:** for a novel algorithm there is no hand-derivable ground truth, so the literal indices below are captured once from the Rust implementation and then frozen into all three suites. Their value is detecting future *divergence between surfaces*, not validating the first output. Capture them in Step 3 and paste the same literals into every suite.

**Files:**
- Modify: `packages/python/src/epaper_dithering/enums.py`
- Modify: `packages/javascript/src/enums.ts`
- Modify: `packages/python/tests/test_dithering.py`
- Modify: `packages/javascript/tests/dithering.test.ts`
- Test: `packages/rust/core/src/dizzy.rs` (add the same vector as a Rust test)

**Interfaces:**
- Consumes: `DitherMode::Dizzy = 9` from Task 3.
- Produces: `DitherMode.DIZZY = 9` in Python and TypeScript.

- [ ] **Step 1: Add the enum members**

In `packages/python/src/epaper_dithering/enums.py`, after `JARVIS_JUDICE_NINKE = 8`:

```python
    #: Error diffusion with pseudo-random traversal instead of a raster scan,
    #: diffusing error only to not-yet-quantized neighbours. Produces no
    #: directional structure. ``serpentine`` is ignored: there is no scan direction.
    DIZZY = 9
```

In `packages/javascript/src/enums.ts`, after `JARVIS_JUDICE_NINKE = 8,`:

```typescript
  /**
   * Error diffusion with pseudo-random traversal instead of a raster scan,
   * diffusing error only to not-yet-quantized neighbours. Produces no
   * directional structure. `serpentine` is ignored: there is no scan direction.
   */
  DIZZY = 9,
```

Update the file's header comment from `Values match firmware conventions (0-8)` to `(0-9)`.

- [ ] **Step 2: Verify the parity gate passes**

Run: `python3 scripts/check_enum_parity.py`
Expected: exit 0, reporting 10 `DitherMode` members in each of the three languages.

Then confirm the gate would have caught a one-sided change: temporarily delete `DIZZY = 9` from `enums.ts`, re-run, confirm exit 1 naming the TypeScript file, then restore it.

- [ ] **Step 3: Capture the reference vector from Rust**

Add this test to `mod tests` in `packages/rust/core/src/dizzy.rs`, with `todo` as a placeholder:

```rust
    #[test]
    fn cross_language_reference_vector() {
        // 4x4 horizontal ramp, BWR palette, dizzy. These literals are frozen and
        // mirrored verbatim in the Python and JavaScript suites; if any surface
        // drifts, exactly one of the three tests fails.
        let mut img = Vec::new();
        for y in 0..4u8 {
            for x in 0..4u8 {
                let v = x * 60 + y * 5;
                img.extend_from_slice(&[v, v, v]);
            }
        }
        let out = dizzy_dither(&img, 4, 4, ColorScheme::Bwr.palette());
        assert_eq!(out, vec![/* PASTE FROM STEP 3 */]);
    }
```

Run it once with a `dbg!(&out);` line added to print the actual vector, paste the 16 values into the assertion, remove the `dbg!`, and re-run to confirm PASS.

Run: `cd packages/rust && cargo test -p epaper-dithering-core cross_language_reference_vector`
Expected: PASS

- [ ] **Step 4: Mirror the vector into Python**

Add to `packages/python/tests/test_dithering.py`:

```python
class TestDizzy:
    """Dizzy dithering (DitherMode 9) must match the Rust reference byte for byte."""

    def test_cross_language_reference_vector(self):
        from PIL import Image
        img = Image.new("RGB", (4, 4))
        img.putdata([(x * 60 + y * 5,) * 3 for y in range(4) for x in range(4)])
        out = dither_image(img, ColorScheme.BWR, mode=DitherMode.DIZZY)
        # Frozen literals, identical to the Rust and JavaScript suites.
        assert list(out.getdata()) == [<PASTE THE SAME 16 VALUES>]

    def test_is_deterministic(self):
        from PIL import Image
        img = Image.new("RGB", (16, 16), (128, 128, 128))
        a = dither_image(img, ColorScheme.BWR, mode=DitherMode.DIZZY)
        b = dither_image(img, ColorScheme.BWR, mode=DitherMode.DIZZY)
        assert list(a.getdata()) == list(b.getdata())
```

- [ ] **Step 5: Mirror the vector into TypeScript**

Add to `packages/javascript/tests/dithering.test.ts`:

```typescript
describe('Dizzy dithering', () => {
  const rampImage = () => {
    const data = new Uint8ClampedArray(4 * 4 * 4);
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        const v = x * 60 + y * 5;
        const o = (y * 4 + x) * 4;
        data[o] = v; data[o + 1] = v; data[o + 2] = v; data[o + 3] = 255;
      }
    }
    return { data, width: 4, height: 4 };
  };

  it('matches the Rust reference vector byte for byte', () => {
    const result = ditherImage(rampImage(), ColorScheme.BWR, { mode: DitherMode.DIZZY });
    // Frozen literals, identical to the Rust and Python suites.
    expect(Array.from(result.indices)).toEqual([/* PASTE THE SAME 16 VALUES */]);
  });

  it('is deterministic', () => {
    const a = ditherImage(rampImage(), ColorScheme.BWR, { mode: DitherMode.DIZZY });
    const b = ditherImage(rampImage(), ColorScheme.BWR, { mode: DitherMode.DIZZY });
    expect(Array.from(a.indices)).toEqual(Array.from(b.indices));
  });
});
```

- [ ] **Step 6: Rebuild both bindings and run all three suites**

```bash
wasm-pack build packages/rust/wasm --target bundler --out-dir ../../javascript/src/wasm-core
cd packages/python && maturin develop && pytest && cd ../..
cd packages/javascript && bun run type-check && bun run test && cd ../..
python3 scripts/check_enum_parity.py
```
Expected: all pass. The three reference-vector tests must agree on the same 16 values.

- [ ] **Step 7: Commit**

```bash
git add packages/python/src/epaper_dithering/enums.py packages/javascript/src/enums.ts \
        packages/python/tests/test_dithering.py packages/javascript/tests/dithering.test.ts \
        packages/rust/core/src/dizzy.rs
git commit -m "feat: expose DIZZY dither mode in Python and JavaScript"
```

---

### Task 5: Regression fixtures and user-facing docs

**Files:**
- Modify: `packages/rust/core/tests/regression.rs`
- Create: `packages/rust/core/tests/fixtures/references/*__dizzy_*.bin` (generated)
- Modify: `README.md`, `packages/python/README.md`, `packages/javascript/README.md`
- Modify: `docs/index.html`, `packages/javascript/demo.html`, `packages/javascript/dev.html`

**Interfaces:**
- Consumes: `DitherMode::Dizzy` from Task 3.
- Produces: stored `.bin` references pinning dizzy output for future changes.

- [ ] **Step 1: Add the regression cases**

The existing suites iterate `discover_images()` rather than a hardcoded list, so images are picked up automatically. Append these two tests to the "Regression suites" section of `packages/rust/core/tests/regression.rs`, mirroring the shape of `burkes_spectra6_auto` and `floyd_steinberg_mono_raw` exactly:

```rust
/// Dizzy + 6-color measured palette + auto preprocessing.
/// Random-traversal family — pins the permutation walk against accidental change.
#[test]
fn dizzy_spectra6_auto() {
    for img in discover_images() {
        assert_regression(
            &img,
            "dizzy_spectra6_auto",
            DitherMode::Dizzy,
            &SPECTRA_7_3_6COLOR,
            ToneCompression::Auto,
            GamutCompression::Auto,
        );
    }
}

/// Dizzy + monochrome + no preprocessing.
/// Two-color palette makes any traversal-order change highly visible in the diff.
#[test]
fn dizzy_mono_raw() {
    for img in discover_images() {
        assert_regression(
            &img,
            "dizzy_mono_raw",
            DitherMode::Dizzy,
            ColorScheme::Mono,
            ToneCompression::Fixed(0.0),
            GamutCompression::None,
        );
    }
}
```

- [ ] **Step 2: Generate the new references**

Run: `cd packages/rust && UPDATE_FIXTURES=1 cargo test --test regression`
Expected: writes 8 new `.bin` files under `tests/fixtures/references/`.

- [ ] **Step 3: Verify no existing reference was touched**

Run: `git status --short packages/rust/core/tests/fixtures/references/`
Expected: **exactly 8 new (`??`) files and zero modified (` M`) files.** If any existing `.bin` shows as modified, STOP — adding a mode has changed existing output, which is a bug. Do not commit.

- [ ] **Step 4: Re-run without the env var to confirm they hold**

Run: `cd packages/rust && cargo test --test regression`
Expected: PASS, now 20 reference comparisons.

- [ ] **Step 5: Update the user-facing mode lists**

Add Dizzy to the dithering-mode tables in `README.md`, `packages/python/README.md` and `packages/javascript/README.md`, matching each file's existing format. Describe it as: *"Error diffusion with pseudo-random traversal — no directional structure. Ignores `serpentine`."*

Add a `<option value="9">Dizzy</option>` entry to the mode `<select>` in `docs/index.html`, `packages/javascript/demo.html` and `packages/javascript/dev.html`, matching each file's existing option format.

- [ ] **Step 6: Verify everything still passes**

```bash
cd packages/rust && cargo test --workspace && cargo clippy --workspace -- -D warnings && cd ../..
python3 scripts/check_enum_parity.py
cargo run --manifest-path packages/rust/Cargo.toml --example gen_ts_palettes -- --check
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add packages/rust/core/tests/regression.rs packages/rust/core/tests/fixtures/references/ \
        README.md packages/python/README.md packages/javascript/README.md \
        docs/index.html packages/javascript/demo.html packages/javascript/dev.html
git commit -m "test: pin dizzy dithering output and document the mode"
```

---

### Task 6: Evaluation and benchmark

The spec's acceptance criterion. A negative result is a legitimate outcome and must be reported honestly in the PR body, not buried.

**Files:**
- Create: `packages/rust/core/examples/dizzy_compare.rs`
- Modify: `packages/rust/core/benches/dithering.rs`

**Interfaces:**
- Consumes: `DitherMode::Dizzy` from Task 3.
- Produces: a printed ΔE comparison table and a criterion benchmark; no library API.

- [ ] **Step 1: Write the comparison harness**

Block-averaging is the correct metric: it measures whether *local average colour* is preserved, which is what dithering is for. Per-pixel ΔE would just measure dither noise and rank every mode as terrible.

Create `packages/rust/core/examples/dizzy_compare.rs`:

```rust
//! Compare dizzy dithering against Burkes and Floyd-Steinberg.
//!
//! Metric: mean OKLab dE between the source and the dithered result, both
//! averaged over 4x4 blocks. Block-averaging measures whether local average
//! colour is preserved; per-pixel dE would only measure dither noise.
//!
//!   cargo run --release --example dizzy_compare

use epaper_dithering_core::{
    color_space::srgb_channel_to_linear,
    color_space_lab::rgb_to_oklab,
    dither, dither_with_canonical,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::SPECTRA_7_3_6COLOR,
    palettes::{ColorScheme, Palette},
    types::ImageBuffer,
    DitherConfig,
};

const BLOCK: usize = 4;

/// Mean OKLab dE between two flat RGB buffers, averaged over BLOCK x BLOCK tiles.
fn block_delta_e(a: &[u8], b: &[u8], width: usize, height: usize) -> f64 {
    let mut total = 0.0;
    let mut blocks = 0usize;
    for by in (0..height).step_by(BLOCK) {
        for bx in (0..width).step_by(BLOCK) {
            let (mut sa, mut sb, mut n) = ([0.0; 3], [0.0; 3], 0.0);
            for y in by..(by + BLOCK).min(height) {
                for x in bx..(bx + BLOCK).min(width) {
                    let i = (y * width + x) * 3;
                    for c in 0..3 {
                        sa[c] += srgb_channel_to_linear(a[i + c]);
                        sb[c] += srgb_channel_to_linear(b[i + c]);
                    }
                    n += 1.0;
                }
            }
            let la = rgb_to_oklab(sa[0] / n, sa[1] / n, sa[2] / n);
            let lb = rgb_to_oklab(sb[0] / n, sb[1] / n, sb[2] / n);
            let d = ((la.l - lb.l).powi(2) + (la.a - lb.a).powi(2) + (la.b - lb.b).powi(2)).sqrt();
            total += d;
            blocks += 1;
        }
    }
    total / blocks as f64
}

/// Expand palette indices back into a flat RGB buffer.
fn to_rgb(indices: &[u8], palette: &Palette) -> Vec<u8> {
    indices
        .iter()
        .flat_map(|&i| palette.colors[i as usize])
        .collect()
}

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images");
    let mut images: Vec<_> = std::fs::read_dir(&dir)
        .expect("fixtures/images")
        .filter_map(|e| {
            let e = e.ok()?;
            e.file_type().ok()?.is_file().then(|| e.file_name().into_string().ok())?
        })
        .filter(|n| n.ends_with(".png") || n.ends_with(".jpg") || n.ends_with(".jpeg"))
        .collect();
    images.sort();

    let modes = [
        ("dizzy", DitherMode::Dizzy),
        ("burkes", DitherMode::Burkes),
        ("floyd_steinberg", DitherMode::FloydSteinberg),
    ];

    println!("{:<22} {:<10} {:<16} {:>10}", "image", "palette", "mode", "mean dE");
    let mut totals = [(0.0, 0usize); 3];

    for name in &images {
        let img = image::open(dir.join(name)).expect("load").to_rgb8();
        let (w, h) = img.dimensions();
        let (w, h) = (w as usize, h as usize);
        let src = img.into_raw();
        let buf = ImageBuffer::new(&src, w);

        for (pal_name, is_measured) in [("spectra6", true), ("mono", false)] {
            for (mi, (mode_name, mode)) in modes.iter().enumerate() {
                let (indices, out_palette): (Vec<u8>, &Palette) = if is_measured {
                    let cfg = DitherConfig {
                        mode: *mode,
                        tone: ToneCompression::Auto,
                        gamut: GamutCompression::Auto,
                        ..Default::default()
                    };
                    (
                        dither_with_canonical(
                            &buf,
                            &SPECTRA_7_3_6COLOR,
                            ColorScheme::Bwgbry.palette(),
                            cfg,
                        ),
                        &SPECTRA_7_3_6COLOR,
                    )
                } else {
                    let cfg = DitherConfig { mode: *mode, ..Default::default() };
                    (dither(&buf, ColorScheme::Mono.palette(), cfg), ColorScheme::Mono.palette())
                };

                let rgb = to_rgb(&indices, out_palette);
                let de = block_delta_e(&src, &rgb, w, h);
                totals[mi].0 += de;
                totals[mi].1 += 1;
                println!("{name:<22} {pal_name:<10} {mode_name:<16} {de:>10.4}");
            }
        }
    }

    println!("\n{:<33} {:<16} {:>10}", "SUMMARY", "mode", "mean dE");
    for (mi, (mode_name, _)) in modes.iter().enumerate() {
        let (sum, n) = totals[mi];
        println!("{:<33} {mode_name:<16} {:>10.4}", "", sum / n as f64);
    }
}
```

If `image` is not already a dev-dependency of the core crate, it is — the regression tests use it. Confirm with `grep -n "^image" packages/rust/core/Cargo.toml`; examples can use dev-dependencies.

- [ ] **Step 2: Run the comparison and record the numbers**

Run: `cd packages/rust && cargo run --release --example dizzy_compare`
Expected: a table of mean ΔE values. Save the full output — it goes in the PR body verbatim.

- [ ] **Step 3: Add the benchmark**

In `packages/rust/core/benches/dithering.rs`, extend the existing `bench_error_diffusion` group (or add a sibling group following its exact pattern) to include `DitherMode::Dizzy` at the same image sizes used for the other modes.

- [ ] **Step 4: Run the benchmark**

Run: `cd packages/rust && cargo bench --bench dithering -- dizzy`
Expected: completes and reports throughput. Record dizzy's time alongside Burkes' at the same size — dizzy makes random access across a multi-hundred-KB f64 buffer, so it is expected to be slower. **The absolute number matters:** Home Assistant renders on this path, which is why PR #57 released the GIL.

- [ ] **Step 5: Commit**

```bash
git add packages/rust/core/examples/dizzy_compare.rs packages/rust/core/benches/dithering.rs
git commit -m "test: add dizzy quality comparison harness and benchmark"
```

- [ ] **Step 6: Open the PR**

```bash
git push -u origin feat/dizzy-dithering
gh pr create --title "feat: add dizzy dithering (DitherMode 9)"
```

The PR body must contain:
- the full ΔE comparison table from Step 2, with an honest reading of where dizzy wins and loses;
- the benchmark numbers from Step 4 versus Burkes;
- a note that `odl-renderer` and `py-opendisplay` must adopt id 9 before this is reachable from Home Assistant.

---

## Verification (whole plan)

```bash
cd packages/rust && cargo test --workspace && cargo clippy --workspace -- -D warnings
cd ../.. && wasm-pack build packages/rust/wasm --target bundler --out-dir ../../javascript/src/wasm-core
cd packages/python && maturin develop && pytest && cd ../..
cd packages/javascript && bun run type-check && bun run test && bun run build && cd ../..
python3 scripts/check_enum_parity.py
python3 scripts/check_enum_parity.py --header /Users/gabriel/Developer/OpenDisplay/opendisplay-protocol/src/opendisplay_structs.h
cargo run --manifest-path packages/rust/Cargo.toml --example gen_ts_palettes -- --check
```

All must pass, and `git status --short packages/rust/core/tests/fixtures/references/` must show **8 added files and zero modified files**.

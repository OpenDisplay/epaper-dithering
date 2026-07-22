# Design: Dizzy dithering (random-traversal error diffusion)

**Status:** approved design, ready for implementation planning
**Date:** 2026-07-22
**Source:** [Dizzy Dithering, Liam Appelbe](https://liamappelbe.medium.com/dizzy-dithering-2ae76dbceba1)

## Context

Every error-diffusion mode in this library scans in raster order and pushes error forward
along a fixed kernel. That fixed direction is what produces the characteristic worm and
hatch artifacts of Floyd-Steinberg and friends, and serpentine scanning only softens it.

Dizzy dithering removes the scan direction entirely: it visits pixels in a pseudo-random
order and pushes each pixel's error to whichever neighbours have not been quantized yet.
The article reports "essentially no artificial structure", detail retention competitive with
blue-noise dithering, and better preservation of thin features.

We are adding it as a tenth `DitherMode`. `opendisplay_structs.h:187` states that "the image
dither-algorithm ids (DitherMode) are owned by epaper-dithering", so unlike `ColorScheme`
this needs no protocol-repo change: **the next free value, 9, is ours to assign.**

The preparatory refactor is already merged (`e89aacf`, PR #66): `dispatch()` is now an
exhaustive match with no wildcard arm, so adding a variant is a compile error until handled,
and the kernel table lives in `kernels.rs`.

## The algorithm

Setup is identical to `error_diffusion_dither_impl` and is reused unchanged: OKLab palette
matching at `WAB = 1.5`, an f64 working buffer in sRGB space, and the u8→linear LUT. Only
traversal and error distribution differ.

### Traversal — a stateless permutation

Let `n = width * height` and `bits` be the smallest integer with `2^bits >= n`;
`MASK = 2^bits - 1`. For each `i` in `0..2^bits`, map it through five rounds:

```
p = i
for r in 0..5 {
    p = (p.wrapping_mul(ODD[r])) & MASK;
    p ^= XOR[r] & MASK;
}
```

Skip `p >= n`, and skip pixels already processed.

This is a bijection on `0..2^bits`: multiplication by an odd number is invertible modulo
`2^k` (odd numbers are units in that ring), and XOR by a constant is self-inverse. A
composition of bijections is a bijection, so **every pixel is visited exactly once** with no
shuffled index array to allocate.

Constants — fixed forever, since changing them changes every rendered image:

```
ODD: [0x2545F491, 0x9E3779B1, 0x85EBCA6B, 0xC2B2AE35, 0x27D4EB2F]   (all odd)
XOR: [0x165667B1, 0xD3A2646C, 0xFD7046C5, 0xB55A4F09, 0x1B873593]
```

Arithmetic is `u64` with `wrapping_mul`, masked to `bits` each round.

Rejection cost is bounded: worst case is `n = 2^k + 1`, giving just under 2× iterations. At
800×480 (384,000 px) the walk covers 524,288 candidates — a 1.36× overhead on an
already-cheap loop counter.

### Error distribution

For the eight neighbours of the current pixel, considering **only those not yet processed**:

- weight `1.0` for the four orthogonal neighbours,
- weight `0.1` for the four diagonal neighbours,
- `denom` = sum of the weights of the qualifying neighbours,
- each qualifying neighbour receives `err * weight / denom`.

The 10:1 orthogonal:diagonal ratio is the article's. Because `denom` is recomputed per pixel
over only the unprocessed neighbours, error is fully conserved except in one case: when
**every** neighbour is already processed, `denom == 0` and the error is dropped. That is
inherent to the algorithm and must carry an explicit comment, or a future reader will
"fix" it into a divide-by-zero or a redistribution that changes output.

### Canonical pinning

`dither_with_canonical` pins pixels whose colour exactly matches a canonical ink. A pre-pass
marks every such pixel as **processed** and writes its index before the walk starts, so no
neighbour spends error on a pixel that will ignore it; the error redistributes to genuinely
unprocessed neighbours instead.

This differs deliberately from the raster implementation, where a pinned pixel accumulates
error that is then discarded when the scan reaches it. Dizzy can do better because it already
tracks processed state.

### Serpentine

Ignored — there is no scan direction to reverse. Documented alongside `None` and `Ordered`,
which ignore it for the same structural reason.

## Decisions taken

| decision | choice | rationale |
|---|---|---|
| Tunables | **None.** Constants and the 0.1 diagonal weight are hardcoded. | Determinism is mandatory: Rust, WASM and Python must produce byte-identical output, and the regression fixtures must hold. A seed field would have to cross four bindings for no demonstrated need. YAGNI. |
| Pinned pixels | **Pre-marked as processed.** | Avoids silently discarding error on pixels that cannot use it. |
| Acceptance | **Measure, then ship.** | The article's evidence is greyscale photographs; these palettes are 2–7 measured inks where hue error dominates. Ship it either way, but with evidence about where it wins. |

## Module structure

New `packages/rust/core/src/dizzy.rs`. `algorithms.rs` is the crate's largest file and dizzy
is a self-contained traversal, so it gets its own module — following the precedent set by
`composite.rs`.

Promote to `pub(crate)` in `algorithms.rs`: `build_palette_lab`, `exact_palette_index`, `WAB`.

Public functions mirroring the existing pair:

```rust
pub fn dizzy_dither(pixels, width, height, palette) -> Vec<u8>
pub fn dizzy_dither_with_canonical(pixels, width, height, palette, canonical) -> Vec<u8>
```

`DitherMode::kernel()` returns `None` for `Dizzy`, and `dispatch()` gains one arm. Because
the match is now exhaustive with no wildcard, the compiler will refuse to build until that
arm exists.

## Surfaces

`Dizzy = 9` / `DIZZY = 9` in all three languages — `scripts/check_enum_parity.py` makes this
mandatory, not optional:

- `packages/rust/core/src/enums.rs`
- `packages/python/src/epaper_dithering/enums.py`
- `packages/javascript/src/enums.ts`

Plus the user-facing lists: three READMEs, `docs/index.html`, `packages/javascript/demo.html`,
`dev.html`, and the mode table in `packages/rust/core/examples/dither.rs`.

## Testing

**The load-bearing test is the bijection check.** For a range of sizes — powers of two,
`2^k + 1`, primes, 1×N, N×1, and 1×1 — assert that walking the permutation visits every index
in `0..n` exactly once. If this breaks, pixels are silently skipped or processed twice and
the output still looks entirely plausible. No other test would catch it.

Also:

- **Determinism:** the same input dithered twice yields identical bytes.
- **Cross-language identity:** a fixed small input produces the same indices in Rust, Python
  and JavaScript, asserted against literal expected values written independently in each
  suite — not one implementation generating another's expectation.
- **Canonical pinning:** mirror `exact_canonical_pixels_are_pinned_inside_mixed_error_diffusion_image`.
- **Error conservation:** on an image with no pinning, total distributed error equals total
  quantization error minus the dropped-at-the-end remainder.
- **Regression fixtures:** add `<image>__dizzy_spectra6_auto.bin` and
  `<image>__dizzy_mono_raw.bin` alongside the existing 12.
- All output indices `< palette.len()`.

## Evaluation

`packages/rust/core/examples/dizzy_compare.rs`, in the style of the existing
`examples/wab_sweep.rs`: mean OKLab ΔE on 4×4-block-averaged output across the four fixture
images, comparing Dizzy against Burkes and Floyd-Steinberg on both Spectra-6 and mono.

Block-averaging is the right metric because it measures whether local *average* colour is
preserved, which is what dithering is for; per-pixel ΔE would just measure noise.

A criterion benchmark alongside the existing `bench_error_diffusion`. Dizzy makes random
access across a multi-hundred-KB f64 buffer, so it is cache-hostile and expected to be
slower than Burkes. The number matters because Home Assistant renders on this path — PR #57
released the GIL precisely because that dither blocks the event loop.

## Risks

- **Quality may not transfer.** Random traversal was demonstrated on continuous-tone
  greyscale. On a 6-ink measured palette, the absence of directional structure may read as
  chromatic noise. The evaluation is the mitigation, and a negative result is a legitimate
  outcome to publish in the PR.
- **Performance.** If it lands far slower than Burkes, it should not become anyone's default.
- **Downstream reachability.** `odl-renderer` maps ODL algorithm ids and py-opendisplay
  passes them through, so Dizzy is not selectable from Home Assistant until those adopt id 9
  — the same propagation shape as the SEVEN_COLOR work, and out of scope here.

## Out of scope

- Any tunable parameter (seed, diagonal weight, decay variant from the article's appendix).
- Changing the default mode. Burkes stays the default.
- The article's "decay" smoothing variant.
- Adopting id 9 in odl-renderer / py-opendisplay.

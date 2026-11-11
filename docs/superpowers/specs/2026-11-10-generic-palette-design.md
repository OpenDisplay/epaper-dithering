# Generic palettes for epaper-dithering (next major release)

Status: draft for review · 2026-11-10

## Why

epaper-dithering is used by OpenDisplay, OEPL and other projects, but its palette model is built
around OpenDisplay. `ColorScheme` integers are OpenDisplay firmware wire values, mirrored by hand in
six files and checked daily against the OpenDisplay protocol header. Built-in palettes are either
ideal RGB (`#000000`, `#ff0000`, …) or a compiled catalog of specific panels.

Two problems come out of that:

1. **New panels need library releases, and some can't be expressed at all.** OpenDisplay's new
   BWRY 16-colour mode reports a per-unit measured palette from the device: 16 entries, each a
   CIELAB value plus per-pass panel codes. Same-part-number panels differ by ΔE 6.4, so no built-in
   table can be right. The experimental OpenDisplay branch had to skip epaper-dithering entirely and
   reimplement a slow, plain Floyd-Steinberg in Python.
2. **Requested pure colours get dithered.** A dashboard asks for `#000000`, but a panel's black ink
   is measurably lighter. Matching against the measured palette leaves an error on every "black"
   pixel, which is diffused into speckle. Text and fills that should be solid come out grainy. The
   same happens with every other ink.

## Goal

epaper-dithering becomes a general library: **image + palette in, palette indices out.** It knows
nothing about OpenDisplay or any other firmware. A palette is data supplied by whoever owns the
hardware. A colour that names an ink comes out as that ink, solid.

### Success test

A single image with a dashboard of pure `#000000` text and fills next to a photo, dithered against a
panel's measured palette:

- **Pass:** text and fills come out as solid ink (no speckle). The photo looks natural and is
  dithered against the panel's real colours.
- **Fail:** speckled black, a colour cast on the photo, or a photo that only looks right with
  anchoring disabled.

This becomes a committed regression fixture: the dashboard + photo image and a stored measured
palette, asserting that every pure-ink region is 100% one index. **Fixture palette** *(my call)*:
the BWRY 16-colour palette measured on ODBEEBA6 (`Firmware_Silabs/panels/palette16_photo_ODBEEBA6.json`).
Its convention is known from its own notes: sRGB from a camera-white-balanced photo, converted to
Lab (D65), absolute. Convert it once with `from_absolute_lab` using its white entry, and commit the
converted file plus a note of its origin. It doesn't depend on the open firmware question, because
the fixture only needs a realistic palette, not this tag's exact one. Add one existing measured
palette (e.g. Spectra 6) as a second case so the test isn't 16-colour only. Anti-aliased text edges are not
pure ink and will be dithered. The test should check those edges look clean. If they don't, that is
evidence for renderer-side inks (see Non-goals), not a reason to widen snapping.

### Non-goals

- **Anything OpenDisplay-specific**: passes, panel codes, firmware scheme numbers, the protocol
  header. These belong to py-opendisplay (separate design).
- **Renderer-side inks** (odl-renderer writing text and fills as ink indices directly, and only
  dithering images). Out of this release. Revisit if the success test's text edges fail.
- **Changing the dithering algorithms themselves** (kernels, ordered, dizzy, DBS, OKLab matching
  weight `Wab = 1.5`). They keep their behaviour and their pinned tests.

### Anti-goals

- Shipping any built-in palette, ideal or measured. An ideal preset would reproduce problem 2 as the
  easy default.
- A compatibility shim for the numbered `ColorScheme` API. This is a **hard cut** in a major
  release, decided by the owner. OEPL and others migrate by supplying palettes.

## The palette

An ordered list of entries. The index of an entry is the value written to the output.

| Field | Required | Meaning |
|---|---|---|
| `appearance` | yes | How the colour looks on this panel: CIELAB (D65), **normalised to paper white** |
| `name` | no | Ink identity: `black`, `white`, `red`, `yellow`, … |
| `nominal` | no, but required if `name` is set | The "pure" sRGB colour people ask for when they mean this ink, e.g. `#ff0000` |
| `measured` | no, default `false` (palette-level) | Metadata only: `true` if the appearances were measured on this unit. No pipeline step reads it *(my call: it changes nothing, so it shouldn't be required)* |

Rules:

- **One colour convention inside the library**: CIELAB D65, normalised so that the panel's white
  state is L\* 100, a\* = b\* = 0 (white-point adaptation from the measured white to D65 white).
  Callers who have absolute measurements or sRGB use the provided conversion helpers; the library
  never guesses which convention it was given.
- **What "white" means**: the entry named `white` is the panel's displayed white state, the thing
  the panel actually shows when asked for white. It is not a reference card or the bare substrate,
  which can differ (`colors.txt` has a Spectra "White" ink distinct from "Paper"). *(My call: it's
  what a `#ffffff` dashboard background lands on, so it's the right anchor.)*
- **Validation, no silent fixing** *(my call)*: constructors and `from_json` reject a palette whose
  `white` entry is not within ΔE 0.5 of [100, 0, 0], with an error that names
  `from_absolute_lab(white)` as the fix. `from_absolute_lab` takes absolute Labs plus the index of
  the white entry and returns a normalised palette whose white is exactly [100, 0, 0]. A palette
  with no `white` entry is accepted as already normalised.
- Entries without `name` (for example the 12 in-between mixes of a 16-colour mode) take part in
  dithering but are never targets of anchoring or snapping.
- Names are unique within a palette. Nominals are unique within a palette.
- 2 to 256 entries (output indices are `u8`).
- `accent_idx` is removed. It is stored and passed through every binding today, but no algorithm in
  the core reads it. A caller needing a "highlight" ink looks one up by name.

### Palette file format

Versioned JSON, loadable from Rust, Python and JavaScript, and the thing OEPL and py-opendisplay
ship palettes in:

```json
{
  "format": "epaper-palette",
  "version": 1,
  "measured": true,
  "source": "free text: who measured, how, when",
  "entries": [
    { "appearance": [100.0, 0.0, 0.0], "name": "white", "nominal": "#ffffff" },
    { "appearance": [18.2, 1.1, -0.4], "name": "black", "nominal": "#000000" },
    { "appearance": [52.0, 8.9, 2.1] }
  ]
}
```

Unknown top-level keys are ignored (so producers can add their own metadata). Unknown `version`
values are rejected with a clear error.

## What `dither` does

```
image ─► exposure ─► saturation ─► shadows/highlights ─► tone ─► ANCHOR ─► gamut ─► SNAP + match/diffuse ─► dbs ─► indices
```

1. **Anchored mapping (new).** Before matching, input colours are warped so that each named ink's
   `nominal` lands exactly on its `appearance`: `#000000` → measured black, `#ffffff` → paper white,
   `#ff0000` → measured red. Colours between anchors are mapped smoothly. This generalises
   black-point compensation to every ink. A flat region of a pure ink then has zero matching error.
   The mapping method is left to the builder (see below), within these constraints: exact at every
   anchor; continuous; monotonic in lightness along the neutral axis; a no-op when the palette has no
   named entries.
2. **Exact ink snap (generalises today's canonical passthrough).** A pixel exactly equal to a named
   ink's `nominal` takes that ink's index. It neither receives nor passes on diffusion error, and DBS
   leaves it pinned. This replaces `dither_with_canonical` and its second parallel palette: the
   behaviour already exists (`try_exact_palette_map`, `pin_exact_pixels`), only its source moves to
   per-entry `nominal`.
   **The snap mask is computed from the original input**, before any pre-processing, and carried
   through matching and DBS. A requested ink stays that ink even when exposure, saturation or tone
   are set for a photo in the same image. *(My call: today the check runs on the processed image, so
   turning on tone silently breaks passthrough. That is the dashboard bug again.)*
3. **Match and diffuse**: unchanged algorithms, matching against `appearance` in OKLab.

Where anchoring sits relative to `tone` is a builder decision (see below); the documented order
above is the starting point.

**Options** *(my call)*: anchoring and snapping are each an option, **on by default**. Off switches
exist for comparison images and debugging, not as a recommended mode. The success test runs with
default options, and a second case runs it with exposure and tone set, to prove snapping survives
pre-processing.

## API shape

One palette type and one options type per binding, mirroring the core one to one:

- **Rust core**: `Palette` (entries as above, constructors that validate), `Palette::from_json`,
  conversion helpers (`from_absolute_lab(white)`, `from_srgb`), `DitherOptions`, `dither(&img,
  &palette, &options) -> Vec<u8>`. Errors are `Result`s, not panics.
- **Python**: the same `Palette` type (constructed from entries or loaded from a file), `dither_image(image,
  palette, options=...)` returning a `"P"` image. The flat fifteen-argument call goes; options
  become one object.
- **JavaScript (wasm)** and **iOS (C ABI)**: the same palette and options types. The iOS ABI gets a
  palette handle or a serialised palette instead of six raw palette arguments.
- **`DitherMode` by name** across bindings instead of integer IDs. It has no wire meaning, so this
  removes the last hand-mirrored enum.
- Internal helpers that the Python extension exposes today but only scripts and tests use
  (`tone_compress`, `gamut_compress`, `rgb_to_oklab_buffer`, `measured_palettes`) become private or
  move to dev tooling.

## Removed

- `ColorScheme` in all six files, `scripts/check_enum_parity.py`, the `enum-parity-header.yml` CI
  job, `palettes.generated.ts`, the `gen_ts_palettes` example and its drift check.
- `measured_palettes.rs` and the per-language measured constants (`SPECTRA_7_3_6COLOR`,
  `SOLUM_BWR`, `HANSHOW_BWY`, …). Their data is converted to palette files and handed to its owners
  (OpenDisplay panels to py-opendisplay; the others published as examples or handed to OEPL).
  **Example palette files may live in this repo** (an `examples/palettes/` folder, test fixtures,
  README snippets) **but are never shipped inside the crate, Python wheel or npm package** and never
  exposed through an API. *(My call: that keeps the anti-goal while still giving newcomers a file to
  start from.)*
- `dither_with_canonical` and the canonical-palette plumbing in the core and all bindings.
- Python's "ideal scheme forces tone/gamut off" branch.
- `docs/MIGRATION-colorscheme-protocol-v2.md` (uncommitted, made obsolete by the hard cut).

## Repo cleanup in the same release

- One agent instructions file: `AGENTS.md` and `CLAUDE.md` are identical copies; keep one and point
  the other at it. Update its content for the new model (the enum-parity and generated-palette
  sections go).
- Stale `VERSION = '0.1.0'` in `packages/javascript/src/index.ts`: derive it from the package
  version or add it to release-please's extra files.
- Commit the OKLab re-tuning harness (`core/examples/wab_sweep.rs`, `wab_compare.rs`), which the
  agent docs already reference but which only exists locally. Decide whether `packages/rust/Cargo.lock`
  and `packages/rust/wasm/Cargo.lock` are committed or ignored.
- Consolidate developer tooling (`packages/python/scripts/`, `docs/examples/generate.py`,
  `rust/core/examples/`) into one tools location. Fold `packages/python/colors.txt` (measured RGB
  notes) into palette files or delete it.
- Rewrite the root, Python and JavaScript READMEs around palettes: how to load or build one, what
  normalisation means, how named inks behave.

## Constraints

- Performance stays in Rust; bindings stay thin (existing rule).
- Release-please manages versions; this ships as a major bump across all packages.
- All three bindings and the iOS build ship the new API in the same release.
- Existing pinned colour-science tests stay green except those that only exist to test removed
  features (scheme enum, built-in catalog).

## Left to the builder

- **The anchoring method**: for example a smooth interpolating warp (radial basis or barycentric
  over the anchors) versus per-ink affine corrections. It must meet the constraints in step 1 and the
  success test. Include a comparison image set.
- **Order of anchoring vs `tone`**: tone compression already maps the image's dynamic range into the
  palette's black…white range, which overlaps with anchoring black and white. Decide whether
  anchoring subsumes tone along the neutral axis, runs after it, or tone is redefined. Document the
  result in the pipeline docs.
- How an entry with `name` but where nominal matching should be tolerant (e.g. `#010101`) behaves:
  exact-only is the default; any tolerance needs a test showing it doesn't hurt photos.
- Exact Rust/Python/JS type and method names; error types; the iOS handle design.
- Where the consolidated tools live.

## Release order

Decided by the owner: **epaper-dithering ships its major release first.** py-opendisplay (and
through it the HA integration) stays pinned to the current major until its own adapter is built in
the follow-up design. Nothing in this plan changes files outside this repo. The converted
OpenDisplay palette files are produced here as part of removing the catalog and kept under
`examples/palettes/` until py-opendisplay takes them over.

## Downstream (separate designs, not built here)

- **py-opendisplay**: palette adapter (device mode description → palette), scheme presets for older
  tags (OpenDisplay scheme number → palette, now owned there), pass packer (index → per-pass panel
  codes → frames). Replaces the per-scheme encoder chain.
- **Firmware asks** (for the firmware owner): mark which palette entries are pure inks (a name or
  flag per entry); define exactly what the measured Lab values are (illuminant; absolute or
  normalised to paper white; today's data is absolute, paper white at L\* 68).

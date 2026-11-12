# Dev tools

Scripts for visual comparison, calibration patches and documentation images. They use the Python
package, so run them with its environment from the repo root:

```sh
uv run --project packages/python python tools/<script>.py --help
```

| Script | What it does |
|---|---|
| `compare.py` | Grids of every algorithm × palette, plus a gamut compression sheet |
| `generate_patches.py` | Calibration patch images for a palette, to photograph and measure |
| `generate_docs_examples.py` | Regenerates the images in `docs/examples/` used by the READMEs |
| `make_dashboard_fixture.py` | Regenerates the dashboard + photo success-test fixture |
| `example_palettes.py` | Shared helper: loads `examples/palettes/*.json` |

The Rust tuning harnesses (`anchor_compare`, `wab_sweep`, `wab_compare`, `dbs_compare`,
`dizzy_compare`, `dither`) stay in `packages/rust/core/examples/`, because cargo only builds
examples from there:

```sh
cd packages/rust && cargo run --release -p epaper-dithering-core --example anchor_compare -- out_dir
```

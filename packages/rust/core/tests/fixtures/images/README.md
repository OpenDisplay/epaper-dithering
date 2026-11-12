# Test fixture images

Photographs used for regression testing and benchmarking.

## Copyright

All images © Gabriel. Included in this repository for testing purposes only.

## Images

| File | Content | Primary test coverage |
|---|---|---|
| `frankfurt_nacht.png` | Frankfurt skyline at night | Dark scene, black-point lift (`tone`) |
| `unicorn.png` | Unicorn sculpture, Tollwood festival Munich | Gamut compression — vivid colors |
| `cat_orange.png` | Orange cat in grass, golden hour | Fine detail, warm tones |
| `cat.png` | Calico cat on tiles | High contrast B&W + color patches |
| `olympiapark.png` | Olympic Park, Munich | Outdoor scene, sky gradients |
| `river.png` | River/waterway with sky | Daylight outdoor, sky gradients |
| `ubahn_station.png` | Marienplatz U-Bahn station | Graphic/saturated, architecture |
| `seeed_opendisplay.png` | Synthetic color chart | Pure primaries, checkerboard, text |

## Benchmark-only

`benchmark_only/marienplatz.png` (Marienplatz U-Bahn, 1280×960) is used only by the `full_res`
benchmark in `benches/dithering.rs`. It is not included in regression tests.

`success/dashboard_photo.png` is the dashboard + photo fixture for `tests/success.rs`, written by
`tools/make_dashboard_fixture.py`.

## Adding more images

Drop any `.png` or `.jpg` file into this directory and run:

```bash
UPDATE_FIXTURES=1 cargo test --test regression
```

This generates reference outputs for every regression suite and locks them in.
No code changes needed. Regression images are 800×480 (the `real_images` benchmark relies on it).
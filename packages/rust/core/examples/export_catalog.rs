//! One-off: export the 6.x built-in palettes as `epaper-palette` files into
//! `examples/palettes/` at the repo root, before the built-ins are removed.
//!
//!   cargo run -p epaper-dithering-core --example export_catalog

use std::path::Path;

use epaper_dithering_core::measured_palettes::CATALOG;
use epaper_dithering_core::palette::{Ink, Palette};
use epaper_dithering_core::palettes::{ColorScheme, Palette as SchemePalette};

const SCHEMES: &[(ColorScheme, &str)] = &[
    (ColorScheme::Mono, "mono"),
    (ColorScheme::Bwr, "bwr"),
    (ColorScheme::Bwy, "bwy"),
    (ColorScheme::Bwry, "bwry"),
    (ColorScheme::Bwgbry, "bwgbry"),
    (ColorScheme::Grayscale4, "grayscale4"),
    (ColorScheme::Grayscale16, "grayscale16"),
    (ColorScheme::SevenColor, "seven_color"),
    (ColorScheme::Grayscale8, "grayscale8"),
];

/// Each colour becomes a named ink whose nominal is the ideal scheme colour of that name.
fn convert(colors: &SchemePalette, scheme: ColorScheme, measured: bool, source: String) -> Palette {
    let ideal = scheme.palette();
    let names = scheme.color_names();
    let entries = colors
        .colors
        .iter()
        .zip(names)
        .zip(ideal.colors.iter())
        .map(|((&rgb, &name), &nominal)| (rgb, Some(Ink { name: name.to_owned(), nominal })))
        .collect();
    Palette::from_srgb(entries, measured)
        .unwrap_or_else(|e| panic!("{source}: {e}"))
        .with_source(source)
}

fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/palettes");
    std::fs::create_dir_all(&out).unwrap();
    for &(scheme, name) in SCHEMES {
        let p = convert(scheme.palette(), scheme, false, format!("epaper-dithering 6.x built-in ideal {name}"));
        let path = out.join(format!("ideal_{name}.json"));
        std::fs::write(&path, p.to_json() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
    for entry in CATALOG {
        let id = entry.id.to_lowercase();
        let p = convert(entry.palette, entry.scheme, true, format!("epaper-dithering 6.x built-in measured {}", entry.id));
        let path = out.join(format!("{id}.json"));
        std::fs::write(&path, p.to_json() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

//! The `epaper-palette` JSON file format.

use std::path::Path;

use epaper_dithering_core::palette::{Palette, PaletteError};

const SPEC_EXAMPLE: &str = r##"{
  "format": "epaper-palette",
  "version": 1,
  "measured": true,
  "source": "free text: who measured, how, when",
  "entries": [
    { "appearance": [100.0, 0.0, 0.0], "name": "white", "nominal": "#ffffff" },
    { "appearance": [18.2, 1.1, -0.4], "name": "black", "nominal": "#000000" },
    { "appearance": [52.0, 8.9, 2.1] }
  ]
}"##;

fn with(replace: &str, by: &str) -> String {
    assert!(SPEC_EXAMPLE.contains(replace), "fixture does not contain {replace:?}");
    SPEC_EXAMPLE.replacen(replace, by, 1)
}

#[test]
fn parses_spec_example() {
    let p = Palette::from_json(SPEC_EXAMPLE).unwrap();
    assert_eq!(p.len(), 3);
    assert_eq!(p.index_of("black"), Some(1));
    assert!(p.measured());
    assert_eq!(p.source(), Some("free text: who measured, how, when"));
    assert_eq!(p.entries()[1].ink.as_ref().unwrap().nominal, [0, 0, 0]);
}

#[test]
fn round_trips() {
    let p = Palette::from_json(SPEC_EXAMPLE).unwrap();
    assert_eq!(Palette::from_json(&p.to_json()).unwrap(), p);
}

#[test]
fn ignores_unknown_top_level_keys() {
    assert!(Palette::from_json(&with("\"version\": 1,", "\"version\": 1, \"device\": \"ODBEEBA6\",")).is_ok());
}

#[test]
fn measured_defaults_false() {
    let p = Palette::from_json(&with("\"measured\": true,", "")).unwrap();
    assert!(!p.measured());
}

#[test]
fn rejects_wrong_format() {
    let err = Palette::from_json(&with("\"epaper-palette\"", "\"something-else\"")).unwrap_err();
    assert_eq!(err, PaletteError::UnsupportedFormat("something-else".into()));
}

#[test]
fn rejects_unknown_version() {
    let err = Palette::from_json(&with("\"version\": 1", "\"version\": 2")).unwrap_err();
    assert_eq!(err, PaletteError::UnsupportedVersion(2));
}

#[test]
fn rejects_bad_nominal() {
    let err = Palette::from_json(&with("\"#000000\"", "\"red\"")).unwrap_err();
    assert!(matches!(err, PaletteError::InvalidNominal(ref s) if s == "red"), "{err:?}");
}

#[test]
fn rejects_name_without_nominal() {
    let err = Palette::from_json(&with(", \"nominal\": \"#000000\"", "")).unwrap_err();
    assert!(matches!(err, PaletteError::InvalidJson(_)), "{err:?}");
}

#[test]
fn rejects_nominal_without_name() {
    let err = Palette::from_json(&with("\"name\": \"black\", ", "")).unwrap_err();
    assert!(matches!(err, PaletteError::InvalidJson(_)), "{err:?}");
}

#[test]
fn malformed_json_is_an_error_not_a_panic() {
    assert!(matches!(Palette::from_json("{"), Err(PaletteError::InvalidJson(_))));
}

#[test]
fn validation_still_applies_to_files() {
    let err = Palette::from_json(&with("[100.0, 0.0, 0.0]", "[68.0, 0.0, 0.0]")).unwrap_err();
    assert!(matches!(err, PaletteError::WhiteNotNormalised { .. }), "{err:?}");
}

#[test]
fn every_example_palette_loads() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/palettes");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let text = std::fs::read_to_string(&path).unwrap();
            Palette::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            count += 1;
        }
    }
    assert!(count > 0, "no example palettes in {}", dir.display());
}

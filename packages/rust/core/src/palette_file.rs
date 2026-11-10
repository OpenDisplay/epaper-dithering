//! The `epaper-palette` JSON file format (version 1).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::palette::{Ink, Lab, Palette, PaletteEntry, PaletteError};

const FORMAT: &str = "epaper-palette";
const VERSION: u64 = 1;

#[derive(Deserialize)]
struct FileIn {
    #[serde(default)]
    measured: bool,
    #[serde(default)]
    source: Option<String>,
    entries: Vec<EntryIn>,
}

#[derive(Deserialize)]
struct EntryIn {
    appearance: [f64; 3],
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    nominal: Option<String>,
}

#[derive(Serialize)]
struct FileOut<'a> {
    format: &'static str,
    version: u64,
    measured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    entries: Vec<EntryOut<'a>>,
}

#[derive(Serialize)]
struct EntryOut<'a> {
    appearance: [f64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nominal: Option<String>,
}

fn parse_nominal(s: &str) -> Result<[u8; 3], PaletteError> {
    let invalid = || PaletteError::InvalidNominal(s.to_owned());
    let hex = s.strip_prefix('#').filter(|h| h.len() == 6 && h.is_ascii()).ok_or_else(invalid)?;
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| invalid());
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

fn format_nominal([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

impl Palette {
    /// Load a palette from an `epaper-palette` JSON document. Unknown top-level keys are
    /// ignored; the palette is validated exactly like [`Palette::new`].
    pub fn from_json(s: &str) -> Result<Self, PaletteError> {
        let value: Value = serde_json::from_str(s).map_err(|e| PaletteError::InvalidJson(e.to_string()))?;
        let format = value.get("format").and_then(Value::as_str).unwrap_or_default();
        if format != FORMAT {
            return Err(PaletteError::UnsupportedFormat(format.to_owned()));
        }
        let version = value
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| PaletteError::InvalidJson("missing or non-integer \"version\"".into()))?;
        if version != VERSION {
            return Err(PaletteError::UnsupportedVersion(version));
        }
        let file: FileIn = serde_json::from_value(value).map_err(|e| PaletteError::InvalidJson(e.to_string()))?;
        let entries = file
            .entries
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                let ink = match (e.name, e.nominal) {
                    (Some(name), Some(nominal)) => Some(Ink { name, nominal: parse_nominal(&nominal)? }),
                    (None, None) => None,
                    (Some(_), None) => {
                        return Err(PaletteError::InvalidJson(format!("entry {i} has a name but no nominal")));
                    }
                    (None, Some(_)) => {
                        return Err(PaletteError::InvalidJson(format!("entry {i} has a nominal but no name")));
                    }
                };
                let [l, a, b] = e.appearance;
                Ok(PaletteEntry { appearance: Lab { l, a, b }, ink })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let palette = Palette::new(entries, file.measured)?;
        Ok(match file.source {
            Some(source) => palette.with_source(source),
            None => palette,
        })
    }

    /// Serialise as a pretty-printed `epaper-palette` JSON document.
    pub fn to_json(&self) -> String {
        let file = FileOut {
            format: FORMAT,
            version: VERSION,
            measured: self.measured(),
            source: self.source(),
            entries: self
                .entries()
                .iter()
                .map(|e| EntryOut {
                    appearance: [e.appearance.l, e.appearance.a, e.appearance.b],
                    name: e.ink.as_ref().map(|i| i.name.as_str()),
                    nominal: e.ink.as_ref().map(|i| format_nominal(i.nominal)),
                })
                .collect(),
        };
        serde_json::to_string_pretty(&file).expect("palette serialisation cannot fail")
    }
}

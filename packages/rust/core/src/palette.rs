//! Runtime palettes: measured appearance plus optional ink identity.
//!
//! A palette is data supplied by whoever owns the hardware. Each entry carries how the
//! colour *looks* on the panel (CIELAB D65, normalised so the panel's white is
//! L\* 100, a\* = b\* = 0) and, optionally, which ink it *is* (a name plus the "pure"
//! sRGB colour people ask for when they mean that ink).

use std::fmt;

use crate::lab::{adapt_to_white, delta_e76, lab_to_srgb, srgb_to_lab};
pub use crate::lab::Lab;

/// Maximum ΔE76 distance of a `white` ink from `[100, 0, 0]` that still counts as normalised.
pub const WHITE_TOLERANCE: f64 = 0.5;

const MIN_ENTRIES: usize = 2;
const MAX_ENTRIES: usize = 256;

/// The identity of a pure ink: its name and the nominal sRGB colour that requests it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ink {
    pub name: String,
    pub nominal: [u8; 3],
}

/// One palette entry. Its position in the palette is the index written to the output.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteEntry {
    pub appearance: Lab,
    pub ink: Option<Ink>,
}

/// A validated palette.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    entries: Vec<PaletteEntry>,
    measured: bool,
    source: Option<String>,
    srgb: Vec<[u8; 3]>,
}

/// Why a palette was rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteError {
    TooFewEntries(usize),
    TooManyEntries(usize),
    DuplicateName(String),
    DuplicateNominal([u8; 3]),
    WhiteNotNormalised { delta_e: f64 },
    WhiteIndexOutOfRange(usize),
    /// An appearance component is NaN or infinite.
    InvalidAppearance { index: usize },
    InvalidJson(String),
    UnsupportedFormat(String),
    UnsupportedVersion(u64),
    InvalidNominal(String),
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewEntries(n) => write!(f, "a palette needs at least {MIN_ENTRIES} entries, got {n}"),
            Self::TooManyEntries(n) => write!(f, "a palette can have at most {MAX_ENTRIES} entries, got {n}"),
            Self::DuplicateName(name) => write!(f, "ink name {name:?} is used by more than one entry"),
            Self::DuplicateNominal([r, g, b]) => {
                write!(f, "nominal colour #{r:02x}{g:02x}{b:02x} is used by more than one ink")
            }
            Self::WhiteNotNormalised { delta_e } => write!(
                f,
                "the white entry is ΔE {delta_e:.2} from L* 100, a* = b* = 0; appearances must be \
                 normalised to the panel's white. Build the palette with from_absolute_lab to normalise \
                 absolute measurements"
            ),
            Self::WhiteIndexOutOfRange(i) => write!(f, "white entry index {i} is out of range"),
            Self::InvalidAppearance { index } => {
                write!(f, "entry {index} has an appearance that is not a finite L*a*b* value")
            }
            Self::InvalidJson(msg) => write!(f, "invalid palette file: {msg}"),
            Self::UnsupportedFormat(fmt_name) => {
                write!(f, "not an epaper-palette file (format is {fmt_name:?})")
            }
            Self::UnsupportedVersion(v) => write!(f, "unsupported palette file version {v}"),
            Self::InvalidNominal(s) => write!(f, "nominal colour {s:?} is not #rrggbb"),
        }
    }
}

impl std::error::Error for PaletteError {}

/// Parse a `#rrggbb` nominal colour, as palette files spell it.
pub fn parse_nominal(s: &str) -> Result<[u8; 3], PaletteError> {
    crate::palette_file::parse_nominal(s)
}

impl Palette {
    /// Build a palette from appearances that are already normalised to the panel's white.
    pub fn new(entries: Vec<PaletteEntry>, measured: bool) -> Result<Self, PaletteError> {
        let n = entries.len();
        if n < MIN_ENTRIES {
            return Err(PaletteError::TooFewEntries(n));
        }
        if n > MAX_ENTRIES {
            return Err(PaletteError::TooManyEntries(n));
        }
        if let Some(index) = entries.iter().position(|e| {
            let Lab { l, a, b } = e.appearance;
            !(l.is_finite() && a.is_finite() && b.is_finite())
        }) {
            return Err(PaletteError::InvalidAppearance { index });
        }
        let inks: Vec<&Ink> = entries.iter().filter_map(|e| e.ink.as_ref()).collect();
        for (i, ink) in inks.iter().enumerate() {
            if inks[..i].iter().any(|other| other.name == ink.name) {
                return Err(PaletteError::DuplicateName(ink.name.clone()));
            }
        }
        for (i, ink) in inks.iter().enumerate() {
            if inks[..i].iter().any(|other| other.nominal == ink.nominal) {
                return Err(PaletteError::DuplicateNominal(ink.nominal));
            }
        }
        if let Some(white) = entries.iter().find(|e| e.ink.as_ref().is_some_and(|i| i.name == "white")) {
            let delta_e = delta_e76(white.appearance, Lab { l: 100.0, a: 0.0, b: 0.0 });
            if delta_e > WHITE_TOLERANCE {
                return Err(PaletteError::WhiteNotNormalised { delta_e });
            }
        }
        let srgb = entries.iter().map(|e| lab_to_srgb(e.appearance)).collect();
        Ok(Self { entries, measured, source: None, srgb })
    }

    /// Build a palette from absolute measurements, normalising so that
    /// `entries[white_index]` becomes exactly L\* 100, a\* = b\* = 0.
    pub fn from_absolute_lab(
        mut entries: Vec<PaletteEntry>,
        white_index: usize,
        measured: bool,
    ) -> Result<Self, PaletteError> {
        let white = entries.get(white_index).ok_or(PaletteError::WhiteIndexOutOfRange(white_index))?.appearance;
        for (i, e) in entries.iter_mut().enumerate() {
            e.appearance = if i == white_index {
                Lab { l: 100.0, a: 0.0, b: 0.0 }
            } else {
                adapt_to_white(e.appearance, white)
            };
        }
        Self::new(entries, measured)
    }

    /// Build a palette from sRGB colours. If an ink named `white` is present, every
    /// appearance is normalised to it.
    pub fn from_srgb(colors: Vec<([u8; 3], Option<Ink>)>, measured: bool) -> Result<Self, PaletteError> {
        let white_index = colors.iter().position(|(_, ink)| ink.as_ref().is_some_and(|i| i.name == "white"));
        let entries: Vec<PaletteEntry> =
            colors.into_iter().map(|(rgb, ink)| PaletteEntry { appearance: srgb_to_lab(rgb), ink }).collect();
        match white_index {
            Some(i) => Self::from_absolute_lab(entries, i, measured),
            None => Self::new(entries, measured),
        }
    }

    /// Attach a free-text description of where the palette came from.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Always false: a valid palette has at least two entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn measured(&self) -> bool {
        self.measured
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Index of the ink with this name.
    pub fn index_of(&self, name: &str) -> Option<u8> {
        self.entries
            .iter()
            .position(|e| e.ink.as_ref().is_some_and(|i| i.name == name))
            .map(|i| i as u8)
    }

    /// sRGB rendering of each entry's appearance (clamped to gamut). This is what the
    /// dithering algorithms match against, and what a preview should show.
    pub fn preview_srgb(&self) -> &[[u8; 3]] {
        &self.srgb
    }

    /// Index and ink of every named entry.
    pub fn inks(&self) -> impl Iterator<Item = (u8, &Ink)> {
        self.entries.iter().enumerate().filter_map(|(i, e)| e.ink.as_ref().map(|ink| (i as u8, ink)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn entry(l: f64, a: f64, b: f64) -> PaletteEntry {
        PaletteEntry { appearance: Lab { l, a, b }, ink: None }
    }

    fn ink(l: f64, a: f64, b: f64, name: &str, nominal: [u8; 3]) -> PaletteEntry {
        PaletteEntry { appearance: Lab { l, a, b }, ink: Some(Ink { name: name.into(), nominal }) }
    }

    fn black() -> Ink {
        Ink { name: "black".into(), nominal: [0, 0, 0] }
    }

    fn white() -> Ink {
        Ink { name: "white".into(), nominal: [255, 255, 255] }
    }

    #[test]
    fn rejects_one_entry() {
        assert_eq!(Palette::new(vec![entry(50.0, 0.0, 0.0)], false).unwrap_err(), PaletteError::TooFewEntries(1));
    }

    #[test]
    fn rejects_257_entries() {
        let entries = (0..257).map(|i| entry(i as f64 / 3.0, 0.0, 0.0)).collect();
        assert_eq!(Palette::new(entries, false).unwrap_err(), PaletteError::TooManyEntries(257));
    }

    #[test]
    fn rejects_duplicate_name() {
        let err = Palette::new(vec![ink(0.0, 0.0, 0.0, "black", [0, 0, 0]), ink(10.0, 0.0, 0.0, "black", [1, 1, 1])], false)
            .unwrap_err();
        assert_eq!(err, PaletteError::DuplicateName("black".into()));
    }

    #[test]
    fn rejects_duplicate_nominal() {
        let err = Palette::new(vec![ink(0.0, 0.0, 0.0, "black", [0, 0, 0]), ink(10.0, 0.0, 0.0, "ink", [0, 0, 0])], false)
            .unwrap_err();
        assert_eq!(err, PaletteError::DuplicateNominal([0, 0, 0]));
    }

    #[test]
    fn rejects_unnormalised_white() {
        let err = Palette::new(
            vec![ink(68.26, 8.33, 5.98, "white", [255, 255, 255]), ink(30.47, 8.7, 4.58, "black", [0, 0, 0])],
            true,
        )
        .unwrap_err();
        assert!(matches!(err, PaletteError::WhiteNotNormalised { delta_e } if delta_e > 30.0), "{err:?}");
        assert!(err.to_string().contains("from_absolute_lab"), "{err}");
    }

    #[test]
    fn rejects_non_finite_appearance() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = Palette::new(vec![entry(20.0, 0.0, 0.0), entry(60.0, bad, 0.0)], false).unwrap_err();
            assert_eq!(err, PaletteError::InvalidAppearance { index: 1 });
        }
        let err = Palette::new(vec![ink(f64::NAN, 0.0, 0.0, "white", [255, 255, 255]), entry(20.0, 0.0, 0.0)], false).unwrap_err();
        assert_eq!(err, PaletteError::InvalidAppearance { index: 0 });
    }

    #[test]
    fn accepts_white_within_half_delta_e() {
        let p = Palette::new(vec![ink(99.7, 0.2, -0.2, "white", [255, 255, 255]), entry(20.0, 0.0, 0.0)], false);
        assert!(p.is_ok(), "{p:?}");
    }

    #[test]
    fn accepts_palette_without_white() {
        assert!(Palette::new(vec![entry(20.0, 0.0, 0.0), entry(60.0, 0.0, 0.0)], false).is_ok());
    }

    #[test]
    fn from_absolute_lab_puts_white_at_exactly_100() {
        let p = Palette::from_absolute_lab(
            vec![entry(30.47, 8.7, 4.58), ink(68.26, 8.33, 5.98, "white", [255, 255, 255])],
            1,
            true,
        )
        .unwrap();
        let w = p.entries()[1].appearance;
        assert_relative_eq!(w.l, 100.0, epsilon = 1e-9);
        assert_relative_eq!(w.a, 0.0, epsilon = 1e-9);
        assert_relative_eq!(w.b, 0.0, epsilon = 1e-9);
        assert!(p.entries()[0].appearance.l > 30.47);
    }

    #[test]
    fn from_absolute_lab_rejects_bad_white_index() {
        let err = Palette::from_absolute_lab(vec![entry(30.0, 0.0, 0.0), entry(60.0, 0.0, 0.0)], 5, false).unwrap_err();
        assert_eq!(err, PaletteError::WhiteIndexOutOfRange(5));
    }

    #[test]
    fn from_srgb_roundtrips_preview() {
        let p = Palette::from_srgb(
            vec![([0, 0, 0], Some(black())), ([255, 255, 255], Some(white())), ([200, 30, 30], None)],
            false,
        )
        .unwrap();
        assert_eq!(p.preview_srgb(), &[[0, 0, 0], [255, 255, 255], [200, 30, 30]]);
    }

    #[test]
    fn index_of_finds_named_ink() {
        let p = Palette::from_srgb(vec![([0, 0, 0], Some(black())), ([255, 255, 255], Some(white()))], false).unwrap();
        assert_eq!(p.index_of("white"), Some(1));
        assert_eq!(p.index_of("red"), None);
        let inks: Vec<(u8, &str)> = p.inks().map(|(i, ink)| (i, ink.name.as_str())).collect();
        assert_eq!(inks, vec![(0, "black"), (1, "white")]);
    }

    #[test]
    fn measured_defaults_false_and_source_is_optional() {
        let p = Palette::new(vec![entry(20.0, 0.0, 0.0), entry(60.0, 0.0, 0.0)], false).unwrap();
        assert!(!p.measured());
        assert_eq!(p.source(), None);
        assert_eq!(p.with_source("photo of the test swatches").source(), Some("photo of the test swatches"));
    }
}

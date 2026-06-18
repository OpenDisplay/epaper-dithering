//! Measured color palettes for real e-paper displays.
//!
//! These are photographically calibrated — colors reflect what the display
//! actually produces, not the ideal sRGB values. Use these for best dithering
//! quality on known hardware.
//!
//! Color order within each palette matches the Python package (firmware contract).

use std::borrow::Cow;

use crate::palettes::{ColorScheme, Palette};

// ── Catalog (used by language bindings to expose named palettes) ──────────────

/// A measured palette entry with its display name and color names.
/// Language bindings use this to expose palette constants without duplicating values.
pub struct MeasuredPaletteEntry {
    pub id: &'static str,
    pub palette: &'static Palette,
    pub scheme: ColorScheme,
    pub color_names: &'static [&'static str],
}

/// All measured palettes. Add new displays here — bindings pick them up automatically.
pub static CATALOG: &[MeasuredPaletteEntry] = &[
    MeasuredPaletteEntry {
        id: "SPECTRA_7_3_6COLOR",
        palette: &SPECTRA_7_3_6COLOR,
        scheme: ColorScheme::Bwgbry,
        color_names: &["black", "white", "yellow", "red", "blue", "green"],
    },
    MeasuredPaletteEntry {
        id: "SPECTRA_7_3_6COLOR_V2",
        palette: &SPECTRA_7_3_6COLOR_V2,
        scheme: ColorScheme::Bwgbry,
        color_names: &["black", "white", "yellow", "red", "blue", "green"],
    },
    MeasuredPaletteEntry {
        id: "INKPLATE_6COLOR",
        palette: &INKPLATE_6COLOR,
        scheme: ColorScheme::Bwgbryo,
        color_names: &["black", "white", "green", "blue", "red", "yellow", "orange"],
    },
    MeasuredPaletteEntry {
        id: "MONO_4_26",
        palette: &MONO_4_26,
        scheme: ColorScheme::Mono,
        color_names: &["black", "white"],
    },
    MeasuredPaletteEntry {
        id: "BWRY_4_2",
        palette: &BWRY_4_2,
        scheme: ColorScheme::Bwry,
        color_names: &["black", "white", "yellow", "red"],
    },
    MeasuredPaletteEntry {
        id: "BWRY_3_97",
        palette: &BWRY_3_97,
        scheme: ColorScheme::Bwry,
        color_names: &["black", "white", "yellow", "red"],
    },
    MeasuredPaletteEntry {
        id: "SOLUM_BWR",
        palette: &SOLUM_BWR,
        scheme: ColorScheme::Bwr,
        color_names: &["black", "white", "red"],
    },
    MeasuredPaletteEntry {
        id: "HANSHOW_BWR",
        palette: &HANSHOW_BWR,
        scheme: ColorScheme::Bwr,
        color_names: &["black", "white", "red"],
    },
    MeasuredPaletteEntry {
        id: "HANSHOW_BWY",
        palette: &HANSHOW_BWY,
        scheme: ColorScheme::Bwy,
        color_names: &["black", "white", "yellow"],
    },
];

// ── Spectra 7.3" 6-color ─────────────────────────────────────────────────────

/// Spectra 7.3" 6-color (BWGBRY layout).
/// Measured 2026-02-03, iPhone 15 Pro Max RAW, 6500K reference.
pub static SPECTRA_7_3_6COLOR: Palette = Palette {
    colors: Cow::Borrowed(&[
        [26,  13,  35],   // black
        [185, 202, 205],  // white
        [202, 184,   0],  // yellow
        [121,   9,   0],  // red
        [  0,  69, 139],  // blue
        [ 40,  82,  57],  // green
    ]),
    accent_idx: 3, // red
};

/// Spectra 7.3" 6-color v2.
/// Measured 2026-03-15, DNG with linear tone curve.
pub static SPECTRA_7_3_6COLOR_V2: Palette = Palette {
    colors: Cow::Borrowed(&[
        [ 31,  24,  41],  // black
        [168, 180, 182],  // white
        [180, 173,   0],  // yellow
        [113,  24,  19],  // red
        [ 36,  70, 139],  // blue
        [ 50,  84,  60],  // green
    ]),
    accent_idx: 3, // red
};

// ── Inkplate 6COLOR 7-color ACeP ─────────────────────────────────────────────

/// Inkplate 6COLOR (600x448 7-color ACeP, EP585C / panel_ic_type 0x0043).
/// Color order = panel native nibbles: black0 white1 green2 blue3 red4 yellow5 orange6.
/// Hue-representative values tuned 2026-06-18 so source colors select the correct ink
/// (these drive matching only; the panel always renders its own ink for each nibble).
/// Derived from a forced-nibble calibration capture, then de-washed: the panel's raw
/// inks photograph very desaturated (esp. green, contaminated by ghosting), which makes
/// nearest-color matching send vivid greens->yellow and blues->black. Boosting saturation
/// toward each ink's true hue fixes ink selection. TODO: precise per-ink measurement on a
/// ghosting-cleared panel for tone-accurate values.
pub static INKPLATE_6COLOR: Palette = Palette {
    colors: Cow::Borrowed(&[
        [ 45,  45,  70],  // black (dark blue-gray)
        [230, 230, 230],  // white
        [ 60, 150,  80],  // green
        [ 40,  70, 190],  // blue
        [200,  40,  40],  // red
        [235, 215,  90],  // yellow
        [225, 120,  40],  // orange
    ]),
    accent_idx: 4, // red
};

// ── Monochrome displays ───────────────────────────────────────────────────────

/// 4.26" Monochrome. TODO: measure actual display.
pub static MONO_4_26: Palette = Palette {
    colors: Cow::Borrowed(&[
        [  5,   5,   5],  // black
        [220, 220, 220],  // white
    ]),
    accent_idx: 0,
};

// ── BWRY displays ─────────────────────────────────────────────────────────────

/// 4.2" BWRY. TODO: measure actual display.
pub static BWRY_4_2: Palette = Palette {
    colors: Cow::Borrowed(&[
        [  5,   5,   5],  // black
        [200, 200, 200],  // white
        [200, 180,   0],  // yellow
        [120,  15,   5],  // red
    ]),
    accent_idx: 3,
};

/// 3.97" BWRY — EP397YR_800x480.
/// Measured 2026-03-06, iPhone RAW, paper reference RGB(205,205,205).
pub static BWRY_3_97: Palette = Palette {
    colors: Cow::Borrowed(&[
        [ 10,   7,  14],  // black
        [173, 178, 174],  // white
        [172, 128,   0],  // yellow
        [ 85,  24,  14],  // red
    ]),
    accent_idx: 3,
};

// ── Harvested displays ────────────────────────────────────────────────────────

/// Solum BWR (harvested display). TODO: measure.
pub static SOLUM_BWR: Palette = Palette {
    colors: Cow::Borrowed(&[
        [  5,   5,   5],
        [200, 200, 200],
        [120,  15,   5],
    ]),
    accent_idx: 2,
};

/// Hanshow BWR (harvested display). TODO: measure.
pub static HANSHOW_BWR: Palette = Palette {
    colors: Cow::Borrowed(&[
        [  5,   5,   5],
        [200, 200, 200],
        [120,  15,   5],
    ]),
    accent_idx: 2,
};

/// Hanshow BWY (harvested display). TODO: measure.
pub static HANSHOW_BWY: Palette = Palette {
    colors: Cow::Borrowed(&[
        [  5,   5,   5],
        [200, 200, 200],
        [200, 180,   0],
    ]),
    accent_idx: 2,
};

//! Measured color palettes for real e-paper displays.
//!
//! These are photographically calibrated — colors reflect what the display
//! actually produces, not the ideal sRGB values. Use these for best dithering
//! quality on known hardware.
//!
//! Color order within each palette matches the Python package (firmware contract).
//! That invariant is enforced by `measured_palettes_follow_canonical_color_order`
//! below: every CATALOG entry's `color_names` must equal
//! `ColorScheme::color_names()` for its scheme, so a reordered entry cannot
//! silently swap inks on the wire.

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

// ── Invariants ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Measured palettes must list their colors in their scheme's canonical
    /// index order. Index order is a wire contract with the downstream packers:
    /// a reordered CATALOG entry would silently swap inks on real hardware.
    #[test]
    fn measured_palettes_follow_canonical_color_order() {
        for entry in CATALOG {
            assert_eq!(
                entry.color_names,
                entry.scheme.color_names(),
                "{}: color_names must match the canonical order of {:?}",
                entry.id,
                entry.scheme
            );
            assert_eq!(
                entry.palette.colors.len(),
                entry.color_names.len(),
                "{}: palette length must match color_names length",
                entry.id
            );
            assert_eq!(
                entry.palette.accent_idx,
                entry.scheme.palette().accent_idx,
                "{}: accent_idx must match the canonical accent of {:?}",
                entry.id,
                entry.scheme
            );
            assert!(
                entry.palette.accent_idx < entry.color_names.len(),
                "{}: accent_idx out of range",
                entry.id
            );
        }
    }

    /// Every measured RGB triple must actually look like the color it is named.
    ///
    /// `measured_palettes_follow_canonical_color_order` compares *names* against the
    /// canonical order and never reads `palette.colors`, so swapping two RGB rows within
    /// an entry (e.g. yellow and red in `BWRY_3_97`) passes it — and then propagates to
    /// Python (which derives from `CATALOG` via FFI) and to the generated TypeScript,
    /// putting the wrong ink on real hardware. This ties the values to their names.
    ///
    /// These are photographed/measured values, not pure sRGB — measured "yellow" can be
    /// (172, 128, 0) and measured "white" (168, 180, 182) — so the thresholds are
    /// deliberately loose. They are swap detectors, not calibration pins: nothing here
    /// should need updating when a display is re-measured, only when a row is misplaced.
    #[test]
    fn measured_colors_match_their_names() {
        for entry in CATALOG {
            for (name, rgb) in entry.color_names.iter().zip(entry.palette.colors.iter()) {
                let [r, g, b] = *rgb;
                let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
                let ctx = format!("{}: {name} = [{r}, {g}, {b}]", entry.id);

                match *name {
                    "black" => {
                        assert!(r < 80 && g < 80 && b < 80, "{ctx}: black must be dark on all channels");
                    }
                    "white" => {
                        assert!(
                            r > 140 && g > 140 && b > 140,
                            "{ctx}: white must be light on all channels"
                        );
                    }
                    "red" => {
                        assert!(r > g && r > b, "{ctx}: red must have R dominant");
                        assert!(r >= 60, "{ctx}: red must have a substantial R channel");
                    }
                    "green" => {
                        assert!(g > r && g > b, "{ctx}: green must have G dominant");
                    }
                    "blue" => {
                        assert!(b > r && b > g, "{ctx}: blue must have B dominant");
                    }
                    "yellow" => {
                        assert!(r >= 120 && g >= 100, "{ctx}: yellow must have R and G high");
                        assert!(b < 100 && b < g / 2, "{ctx}: yellow must have B low");
                    }
                    "orange" => {
                        assert!(r >= 120, "{ctx}: orange must have R high");
                        assert!(g > b, "{ctx}: orange must have G above B");
                        assert!(b < 100, "{ctx}: orange must have B low");
                    }
                    // Grayscale ramp steps: no hue expectation, only neutrality.
                    other if other.starts_with("gray") => {
                        let max = r.max(g).max(b);
                        let min = r.min(g).min(b);
                        assert!(max - min <= 30, "{ctx}: gray steps must be near-neutral");
                    }
                    other => panic!("{}: unhandled color name {other:?} — add a hue assertion for it", entry.id),
                }
            }
        }
    }
}

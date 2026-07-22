//! Palette definitions and color schemes.
//!
//! `ColorScheme` integer values are firmware API contracts — never change them.

use std::borrow::Cow;

use crate::error::DitherError;

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub colors: Cow<'static, [[u8; 3]]>, // sRGB [R, G, B] for each ink color
    pub accent_idx: usize,               // index of the "accent" color in `colors`
}

impl Palette {
    /// Construct a runtime palette from owned color data.
    ///
    /// # Panics
    /// Panics if `colors.len() < 2` or `accent_idx >= colors.len()`.
    pub fn new(colors: Vec<[u8; 3]>, accent_idx: usize) -> Self {
        assert!(colors.len() >= 2, "palette must have at least 2 colors, got {}", colors.len());
        // Palette indices are emitted as `u8` (see `algorithms.rs`), so a palette
        // longer than 256 entries would silently truncate its high indices.
        assert!(colors.len() <= 256, "palette must have at most 256 colors, got {}", colors.len());
        assert!(accent_idx < colors.len(), "accent_idx {accent_idx} out of range (len={})", colors.len());
        Self { colors: Cow::Owned(colors), accent_idx }
    }
}

impl AsRef<Palette> for Palette {
    fn as_ref(&self) -> &Palette {
        self
    }
}

impl AsRef<Palette> for ColorScheme {
    fn as_ref(&self) -> &Palette {
        (*self).palette()
    }
}

/// E-paper color scheme. Integer discriminants match OpenDisplay firmware.
///
/// Canonical source of truth: `enum ColorScheme` in
/// `opendisplay-protocol/src/opendisplay_structs.h`, which names this file as its
/// designated `@external` mirror. Integer values are a wire contract — never
/// change one to make a test pass.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Mono       = 0,
    Bwr        = 1,
    Bwy        = 2,
    Bwry       = 3,
    Bwgbry     = 4,
    Grayscale4 = 5,
    Grayscale16 = 6,
    /// 7-color Spectra/ACeP panels.
    ///
    /// Protocol v2 reassigned value 7 from the former `Grayscale8` (which was a
    /// mistake — gray8 is not a real panel scheme, and was removed rather than
    /// renumbered) to `SevenColor`.
    ///
    /// Ink order is BWGBRY plus orange, matching the bb_epaper logical ink
    /// indices (`bb_epaper.h`: BLACK 0, WHITE 1, YELLOW 2, RED 3, BLUE 4,
    /// GREEN 5, ORANGE 6), over which `u8Colors_7clr` is the identity map.
    SevenColor = 7,
    /// Spectra 6 nibbles packed as left-half plane then right-half plane
    /// (dual-CS panels with no device framebuffer). Same palette as `Bwgbry`;
    /// only the firmware-side packing differs.
    BwgbrySplit = 8,
    /// 8-level grayscale dithering target, e.g. the Inkplate 10 (issue #19).
    ///
    /// **Value 9 is NOT a firmware wire value.** `opendisplay_structs.h` defines
    /// `enum ColorScheme` values 0-8 and 100-102 only; it has no value 9 and is
    /// not expected to grow one for this. This variant exists purely so the
    /// dithering library can target 8-level-gray panels that sit outside the
    /// OpenDisplay ecosystem. Never send this value to a device as a
    /// `color_scheme` field -- it will not be understood by any OpenDisplay
    /// firmware.
    Grayscale8 = 9,
}

// ── Palette data ─────────────────────────────────────────────────────────────

static PALETTE_MONO: Palette = Palette {
    colors: Cow::Borrowed(&[[0, 0, 0], [255, 255, 255]]),
    accent_idx: 0,
};
static PALETTE_BWR: Palette = Palette {
    colors: Cow::Borrowed(&[[0, 0, 0], [255, 255, 255], [255, 0, 0]]),
    accent_idx: 2,
};
static PALETTE_BWY: Palette = Palette {
    colors: Cow::Borrowed(&[[0, 0, 0], [255, 255, 255], [255, 255, 0]]),
    accent_idx: 2,
};
static PALETTE_BWRY: Palette = Palette {
    colors: Cow::Borrowed(&[[0, 0, 0], [255, 255, 255], [255, 255, 0], [255, 0, 0]]),
    accent_idx: 3,
};
static PALETTE_BWGBRY: Palette = Palette {
    colors: Cow::Borrowed(&[
        [0, 0, 0], [255, 255, 255], [255, 255, 0],
        [255, 0, 0], [0, 0, 255], [0, 255, 0],
    ]),
    accent_idx: 3,
};
static PALETTE_GRAYSCALE4: Palette = Palette {
    colors: Cow::Borrowed(&[[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]]),
    accent_idx: 0,
};
/// 7-color: the six BWGBRY inks in their canonical order, plus orange at index 6.
static PALETTE_SEVEN_COLOR: Palette = Palette {
    colors: Cow::Borrowed(&[
        [0, 0, 0], [255, 255, 255], [255, 255, 0],
        [255, 0, 0], [0, 0, 255], [0, 255, 0],
        [255, 128, 0],
    ]),
    accent_idx: 3,
};
static PALETTE_GRAYSCALE16: Palette = Palette {
    colors: Cow::Borrowed(&[
        [0, 0, 0],   [17, 17, 17],  [34, 34, 34],  [51, 51, 51],
        [68, 68, 68],  [85, 85, 85],  [102, 102, 102], [119, 119, 119],
        [136, 136, 136],[153, 153, 153],[170, 170, 170],[187, 187, 187],
        [204, 204, 204],[221, 221, 221],[238, 238, 238],[255, 255, 255],
    ]),
    accent_idx: 0,
};
/// 8-level grayscale (e.g. Inkplate 10). Library-local; not a firmware scheme.
static PALETTE_GRAYSCALE8: Palette = Palette {
    colors: Cow::Borrowed(&[
        [0, 0, 0], [36, 36, 36], [73, 73, 73], [109, 109, 109],
        [146, 146, 146], [182, 182, 182], [219, 219, 219], [255, 255, 255],
    ]),
    accent_idx: 0,
};

// ── Methods ───────────────────────────────────────────────────────────────────

impl ColorScheme {
    pub fn palette(self) -> &'static Palette {
        match self {
            ColorScheme::Mono        => &PALETTE_MONO,
            ColorScheme::Bwr         => &PALETTE_BWR,
            ColorScheme::Bwy         => &PALETTE_BWY,
            ColorScheme::Bwry        => &PALETTE_BWRY,
            ColorScheme::Bwgbry      => &PALETTE_BWGBRY,
            ColorScheme::Grayscale4  => &PALETTE_GRAYSCALE4,
            ColorScheme::Grayscale16 => &PALETTE_GRAYSCALE16,
            ColorScheme::SevenColor  => &PALETTE_SEVEN_COLOR,
            // Same inks as Bwgbry; only the firmware-side plane packing differs.
            ColorScheme::BwgbrySplit => &PALETTE_BWGBRY,
            ColorScheme::Grayscale8  => &PALETTE_GRAYSCALE8,
        }
    }

    /// Canonical color names for this scheme, in palette index order.
    ///
    /// Index order is a wire contract shared with the downstream packers, so this
    /// doubles as the reference order that measured palettes must reproduce (see
    /// the `measured_palettes_follow_canonical_color_order` test).
    pub fn color_names(self) -> &'static [&'static str] {
        match self {
            ColorScheme::Mono => &["black", "white"],
            ColorScheme::Bwr => &["black", "white", "red"],
            ColorScheme::Bwy => &["black", "white", "yellow"],
            ColorScheme::Bwry => &["black", "white", "yellow", "red"],
            ColorScheme::Bwgbry | ColorScheme::BwgbrySplit => {
                &["black", "white", "yellow", "red", "blue", "green"]
            }
            ColorScheme::Grayscale4 => &["black", "gray1", "gray2", "white"],
            ColorScheme::Grayscale16 => &[
                "black", "gray1", "gray2", "gray3", "gray4", "gray5", "gray6", "gray7",
                "gray8", "gray9", "gray10", "gray11", "gray12", "gray13", "gray14", "white",
            ],
            ColorScheme::SevenColor => {
                &["black", "white", "yellow", "red", "blue", "green", "orange"]
            }
            ColorScheme::Grayscale8 => &[
                "black", "gray1", "gray2", "gray3", "gray4", "gray5", "gray6", "white",
            ],
        }
    }
}

// ── Standard conversion traits ────────────────────────────────────────────────

impl From<ColorScheme> for u8 {
    fn from(s: ColorScheme) -> u8 {
        s as u8
    }
}

impl TryFrom<u8> for ColorScheme {
    type Error = DitherError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ColorScheme::Mono),
            1 => Ok(ColorScheme::Bwr),
            2 => Ok(ColorScheme::Bwy),
            3 => Ok(ColorScheme::Bwry),
            4 => Ok(ColorScheme::Bwgbry),
            5 => Ok(ColorScheme::Grayscale4),
            6 => Ok(ColorScheme::Grayscale16),
            7 => Ok(ColorScheme::SevenColor),
            8 => Ok(ColorScheme::BwgbrySplit),
            // Library-local, NOT a firmware wire value (see the `Grayscale8`
            // variant doc comment). The protocol header has no value 9.
            9 => Ok(ColorScheme::Grayscale8),
            _ => Err(DitherError::UnknownColorScheme(v)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins every wire value against `enum ColorScheme` in
    /// `opendisplay-protocol/src/opendisplay_structs.h`.
    #[test]
    fn firmware_values_are_correct() {
        assert_eq!(u8::from(ColorScheme::Mono), 0);
        assert_eq!(u8::from(ColorScheme::Bwr), 1);
        assert_eq!(u8::from(ColorScheme::Bwy), 2);
        assert_eq!(u8::from(ColorScheme::Bwry), 3);
        assert_eq!(u8::from(ColorScheme::Bwgbry), 4);
        assert_eq!(u8::from(ColorScheme::Grayscale4), 5);
        assert_eq!(u8::from(ColorScheme::Grayscale16), 6);
        assert_eq!(u8::from(ColorScheme::SevenColor), 7);
        assert_eq!(u8::from(ColorScheme::BwgbrySplit), 8);
    }

    /// `Grayscale8` = 9 is NOT in the header (it stops at 8, then jumps to
    /// 100-102) -- this pins the library-local value, not a firmware contract.
    #[test]
    fn grayscale8_is_library_local_value_nine() {
        assert_eq!(u8::from(ColorScheme::Grayscale8), 9);
    }

    #[test]
    fn seven_color_is_bwgbry_plus_orange() {
        let seven = ColorScheme::SevenColor.palette();
        let six = ColorScheme::Bwgbry.palette();
        assert_eq!(seven.colors.len(), 7);
        assert_eq!(&seven.colors[..6], &six.colors[..]);
        assert_eq!(seven.colors[6], [255, 128, 0]);
    }

    #[test]
    fn bwgbry_split_shares_the_bwgbry_palette() {
        assert_eq!(
            ColorScheme::BwgbrySplit.palette().colors,
            ColorScheme::Bwgbry.palette().colors
        );
    }

    #[test]
    fn color_names_match_palette_lengths() {
        for scheme in [
            ColorScheme::Mono,
            ColorScheme::Bwr,
            ColorScheme::Bwy,
            ColorScheme::Bwry,
            ColorScheme::Bwgbry,
            ColorScheme::Grayscale4,
            ColorScheme::Grayscale16,
            ColorScheme::SevenColor,
            ColorScheme::BwgbrySplit,
            ColorScheme::Grayscale8,
        ] {
            assert_eq!(
                scheme.color_names().len(),
                scheme.palette().colors.len(),
                "{scheme:?}: color_names length must match palette length"
            );
        }
    }

    /// Ties `color_names` to actual RGB values for every scheme, not just
    /// lengths. `color_names_match_palette_lengths` only checks array length,
    /// so a name array that is the right length but wrongly ordered (or
    /// wrongly worded) relative to `palette().colors` would pass it silently.
    /// Expected RGB values are written out literally rather than derived from
    /// `palettes.rs` data, so this test cannot pass just because the code
    /// under test agrees with itself.
    #[test]
    fn color_names_match_expected_rgb_values() {
        let cases: &[(ColorScheme, &[(&str, [u8; 3])])] = &[
            (
                ColorScheme::Mono,
                &[("black", [0, 0, 0]), ("white", [255, 255, 255])],
            ),
            (
                ColorScheme::Bwr,
                &[("black", [0, 0, 0]), ("white", [255, 255, 255]), ("red", [255, 0, 0])],
            ),
            (
                ColorScheme::Bwy,
                &[("black", [0, 0, 0]), ("white", [255, 255, 255]), ("yellow", [255, 255, 0])],
            ),
            (
                ColorScheme::Bwry,
                &[
                    ("black", [0, 0, 0]),
                    ("white", [255, 255, 255]),
                    ("yellow", [255, 255, 0]),
                    ("red", [255, 0, 0]),
                ],
            ),
            (
                ColorScheme::Bwgbry,
                &[
                    ("black", [0, 0, 0]),
                    ("white", [255, 255, 255]),
                    ("yellow", [255, 255, 0]),
                    ("red", [255, 0, 0]),
                    ("blue", [0, 0, 255]),
                    ("green", [0, 255, 0]),
                ],
            ),
            (
                ColorScheme::BwgbrySplit,
                &[
                    ("black", [0, 0, 0]),
                    ("white", [255, 255, 255]),
                    ("yellow", [255, 255, 0]),
                    ("red", [255, 0, 0]),
                    ("blue", [0, 0, 255]),
                    ("green", [0, 255, 0]),
                ],
            ),
            (
                ColorScheme::SevenColor,
                &[
                    ("black", [0, 0, 0]),
                    ("white", [255, 255, 255]),
                    ("yellow", [255, 255, 0]),
                    ("red", [255, 0, 0]),
                    ("blue", [0, 0, 255]),
                    ("green", [0, 255, 0]),
                    ("orange", [255, 128, 0]),
                ],
            ),
            (
                ColorScheme::Grayscale4,
                &[
                    ("black", [0, 0, 0]),
                    ("gray1", [85, 85, 85]),
                    ("gray2", [170, 170, 170]),
                    ("white", [255, 255, 255]),
                ],
            ),
            (
                ColorScheme::Grayscale16,
                &[
                    ("black", [0, 0, 0]),
                    ("gray1", [17, 17, 17]),
                    ("gray2", [34, 34, 34]),
                    ("gray3", [51, 51, 51]),
                    ("gray4", [68, 68, 68]),
                    ("gray5", [85, 85, 85]),
                    ("gray6", [102, 102, 102]),
                    ("gray7", [119, 119, 119]),
                    ("gray8", [136, 136, 136]),
                    ("gray9", [153, 153, 153]),
                    ("gray10", [170, 170, 170]),
                    ("gray11", [187, 187, 187]),
                    ("gray12", [204, 204, 204]),
                    ("gray13", [221, 221, 221]),
                    ("gray14", [238, 238, 238]),
                    ("white", [255, 255, 255]),
                ],
            ),
            (
                ColorScheme::Grayscale8,
                &[
                    ("black", [0, 0, 0]),
                    ("gray1", [36, 36, 36]),
                    ("gray2", [73, 73, 73]),
                    ("gray3", [109, 109, 109]),
                    ("gray4", [146, 146, 146]),
                    ("gray5", [182, 182, 182]),
                    ("gray6", [219, 219, 219]),
                    ("white", [255, 255, 255]),
                ],
            ),
        ];

        for (scheme, expected) in cases {
            let names = scheme.color_names();
            let colors = &scheme.palette().colors;
            assert_eq!(
                names.len(),
                expected.len(),
                "{scheme:?}: test table length mismatch"
            );
            assert_eq!(
                colors.len(),
                expected.len(),
                "{scheme:?}: palette length mismatch"
            );
            for (i, (expected_name, expected_rgb)) in expected.iter().enumerate() {
                assert_eq!(
                    names[i], *expected_name,
                    "{scheme:?}: color_names[{i}] name mismatch"
                );
                assert_eq!(
                    colors[i], *expected_rgb,
                    "{scheme:?}: palette().colors[{i}] ({}) RGB mismatch",
                    expected_name
                );
            }
        }
    }

    #[test]
    fn from_into_u8() {
        assert_eq!(u8::from(ColorScheme::Mono), 0u8);
        assert_eq!(u8::from(ColorScheme::Grayscale16), 6u8);
        let v: u8 = ColorScheme::Bwr.into();
        assert_eq!(v, 1u8);
    }

    #[test]
    fn try_from_u8() {
        assert_eq!(ColorScheme::try_from(0), Ok(ColorScheme::Mono));
        assert_eq!(ColorScheme::try_from(4), Ok(ColorScheme::Bwgbry));
        assert_eq!(ColorScheme::try_from(7), Ok(ColorScheme::SevenColor));
        assert_eq!(ColorScheme::try_from(8), Ok(ColorScheme::BwgbrySplit));
        assert_eq!(ColorScheme::try_from(9), Ok(ColorScheme::Grayscale8));
        assert_eq!(ColorScheme::try_from(10), Err(DitherError::UnknownColorScheme(10)));
        assert_eq!(ColorScheme::try_from(99), Err(DitherError::UnknownColorScheme(99)));
    }

    #[test]
    fn palette_color_counts() {
        assert_eq!(ColorScheme::Mono.palette().colors.len(), 2);
        assert_eq!(ColorScheme::Bwgbry.palette().colors.len(), 6);
        assert_eq!(ColorScheme::Grayscale16.palette().colors.len(), 16);
        assert_eq!(ColorScheme::SevenColor.palette().colors.len(), 7);
        assert_eq!(ColorScheme::Grayscale8.palette().colors.len(), 8);
    }

    #[test]
    #[should_panic(expected = "at most 256 colors")]
    fn palette_new_rejects_more_than_256_colors() {
        Palette::new(vec![[0, 0, 0]; 257], 0);
    }

    #[test]
    fn palette_new_accepts_exactly_256_colors() {
        assert_eq!(Palette::new(vec![[0, 0, 0]; 256], 0).colors.len(), 256);
    }
}

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
    /// Reserved: 8-level grayscale, pending firmware value assignment.
    Grayscale8 = 7,
    /// 7-color ACeP (e.g. Inkplate 6COLOR). Color order matches the panel's
    /// native nibble codes: black0 white1 green2 blue3 red4 yellow5 orange6.
    Bwgbryo    = 8,
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
// 7-color ACeP. Order = panel native nibbles: black0 white1 green2 blue3 red4 yellow5 orange6.
static PALETTE_BWGBRYO: Palette = Palette {
    colors: Cow::Borrowed(&[
        [0, 0, 0], [255, 255, 255], [0, 255, 0], [0, 0, 255],
        [255, 0, 0], [255, 255, 0], [255, 128, 0],
    ]),
    accent_idx: 4, // red
};
static PALETTE_GRAYSCALE4: Palette = Palette {
    colors: Cow::Borrowed(&[[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]]),
    accent_idx: 0,
};
static PALETTE_GRAYSCALE8: Palette = Palette {
    colors: Cow::Borrowed(&[
        [0, 0, 0], [36, 36, 36], [73, 73, 73], [109, 109, 109],
        [146, 146, 146], [182, 182, 182], [219, 219, 219], [255, 255, 255],
    ]),
    accent_idx: 0,
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
            ColorScheme::Grayscale8  => &PALETTE_GRAYSCALE8,
            ColorScheme::Bwgbryo     => &PALETTE_BWGBRYO,
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
            7 => Ok(ColorScheme::Grayscale8),
            8 => Ok(ColorScheme::Bwgbryo),
            _ => Err(DitherError::UnknownColorScheme(v)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_values_are_correct() {
        assert_eq!(u8::from(ColorScheme::Mono), 0);
        assert_eq!(u8::from(ColorScheme::Bwr), 1);
        assert_eq!(u8::from(ColorScheme::Grayscale16), 6);
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
        assert_eq!(ColorScheme::try_from(99), Err(DitherError::UnknownColorScheme(99)));
    }

    #[test]
    fn palette_color_counts() {
        assert_eq!(ColorScheme::Mono.palette().colors.len(), 2);
        assert_eq!(ColorScheme::Bwgbry.palette().colors.len(), 6);
        assert_eq!(ColorScheme::Grayscale16.palette().colors.len(), 16);
        assert_eq!(ColorScheme::Bwgbryo.palette().colors.len(), 7);
    }

    #[test]
    fn bwgbryo_is_value_8() {
        assert_eq!(u8::from(ColorScheme::Bwgbryo), 8);
        assert_eq!(ColorScheme::try_from(8), Ok(ColorScheme::Bwgbryo));
    }

    #[test]
    fn bwgbryo_order_matches_panel_native_nibbles() {
        // The dither output index is the 4bpp nibble sent to the panel, so the
        // palette order MUST match the Inkplate 6COLOR's native codes:
        // black0 white1 green2 blue3 red4 yellow5 orange6.
        let c = &ColorScheme::Bwgbryo.palette().colors;
        assert_eq!(c[0], [0, 0, 0], "idx0 black");
        assert_eq!(c[1], [255, 255, 255], "idx1 white");
        assert_eq!(c[2], [0, 255, 0], "idx2 green");
        assert_eq!(c[3], [0, 0, 255], "idx3 blue");
        assert_eq!(c[4], [255, 0, 0], "idx4 red");
        assert_eq!(c[5], [255, 255, 0], "idx5 yellow");
        assert_eq!(c[6], [255, 128, 0], "idx6 orange");
    }
}

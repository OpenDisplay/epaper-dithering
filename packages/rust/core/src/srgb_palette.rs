//! The sRGB form of a palette that the dithering algorithms run on.
//!
//! Not part of the supported API: build a [`crate::palette::Palette`] and call
//! [`crate::dither`]. It is public only so the tuning harnesses in `examples/` can drive
//! individual algorithms.

use crate::color_space::srgb_channel_to_linear;
use crate::palette::Palette;

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct SrgbPalette {
    /// sRGB of each entry's appearance, in palette index order.
    pub colors: Vec<[u8; 3]>,
}

impl SrgbPalette {
    /// Linear RGB (0–1) of each entry, in palette index order.
    pub(crate) fn linear(&self) -> Vec<[f64; 3]> {
        self.colors.iter().map(|c| c.map(srgb_channel_to_linear)).collect()
    }
}

impl From<&Palette> for SrgbPalette {
    fn from(p: &Palette) -> Self {
        Self { colors: p.preview_srgb().to_vec() }
    }
}

/// Fixed sRGB palettes for the algorithm unit tests (the former 6.x built-ins).
#[cfg(test)]
pub(crate) mod fixtures {
    use std::sync::LazyLock;

    use super::SrgbPalette;

    macro_rules! fixture {
        ($name:ident, [$($c:expr),+ $(,)?]) => {
            pub(crate) fn $name() -> &'static SrgbPalette {
                static P: LazyLock<SrgbPalette> = LazyLock::new(|| SrgbPalette { colors: vec![$($c),+] });
                &P
            }
        };
    }

    fixture!(mono, [[0, 0, 0], [255, 255, 255]]);
    fixture!(bwr, [[0, 0, 0], [255, 255, 255], [255, 0, 0]]);
    fixture!(bwgbry, [[0, 0, 0], [255, 255, 255], [255, 255, 0], [255, 0, 0], [0, 0, 255], [0, 255, 0]]);
    fixture!(grayscale16, [
        [0, 0, 0], [17, 17, 17], [34, 34, 34], [51, 51, 51],
        [68, 68, 68], [85, 85, 85], [102, 102, 102], [119, 119, 119],
        [136, 136, 136], [153, 153, 153], [170, 170, 170], [187, 187, 187],
        [204, 204, 204], [221, 221, 221], [238, 238, 238], [255, 255, 255],
    ]);
    // 6.x SPECTRA_7_3_6COLOR measured values (not white-normalised).
    fixture!(spectra6, [[26, 13, 35], [185, 202, 205], [202, 184, 0], [121, 9, 0], [0, 69, 139], [40, 82, 57]]);
}

use crate::algorithms::{
    Kernel, ATKINSON, BURKES, FLOYD_STEINBERG, JARVIS_JUDICE_NINKE, SIERRA, SIERRA_LITE, STUCKI,
};
use crate::error::DitherError;
use crate::palettes::Palette;
use crate::tone_map;

/// Dithering algorithm. Integer values match OpenDisplay firmware conventions.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DitherMode {
    /// Direct nearest-color mapping with no error diffusion. Intended for
    /// already-quantized graphics only. On limited palettes (especially BWR),
    /// continuous-tone photos or large flat mid-tone areas can map to an
    /// unexpected ink (e.g. a mid-gray region rendered as solid red); use an
    /// error-diffusion mode (e.g. `FloydSteinberg`, `Burkes`) for photographic input.
    None           = 0,
    #[default]
    Burkes         = 1,
    Ordered        = 2,
    FloydSteinberg = 3,
    Atkinson       = 4,
    Stucki         = 5,
    Sierra         = 6,
    SierraLite     = 7,
    JarvisJudiceNinke = 8,
    /// Error diffusion with pseudo-random traversal instead of a raster scan,
    /// diffusing error only to not-yet-quantized neighbours. Produces no
    /// directional structure. `serpentine` is ignored: there is no scan direction.
    Dizzy          = 9,
}

/// Dynamic range compression applied before dithering.
/// Only meaningful for measured palettes (ignored for `ColorScheme`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneCompression {
    /// Reinhard 2004 skewness-based auto strength. Best for photos.
    Auto,
    /// Fixed blend strength in [0.0, 1.0]. 0.0 disables.
    Fixed(f64),
}

impl Default for ToneCompression {
    /// Matches `DitherConfig::default().tone` (tone compression off).
    fn default() -> Self {
        ToneCompression::Fixed(0.0)
    }
}

impl ToneCompression {
    pub fn apply(self, pixels: &mut [[f64; 3]], palette: &Palette) {
        match self {
            ToneCompression::Auto => tone_map::auto_compress_dynamic_range(pixels, palette),
            ToneCompression::Fixed(s) => if s > 0.0 {
                tone_map::compress_dynamic_range(pixels, palette, s)
            }
        }
    }
}

/// Gamut compression applied before dithering.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum GamutCompression {
    /// No gamut compression (default).
    #[default]
    None,
    /// Full gamut compression at strength 1.0.
    Auto,
    /// Fixed blend strength in [0.0, 1.0].
    Fixed(f64),
}

impl GamutCompression {
    pub fn apply(self, pixels: &mut [[f64; 3]], palette: &Palette) {
        match self {
            GamutCompression::None => {}
            GamutCompression::Auto => tone_map::gamut_compress(pixels, palette, 1.0),
            GamutCompression::Fixed(s) => if s > 0.0 {
                tone_map::gamut_compress(pixels, palette, s)
            }
        }
    }


}

impl TryFrom<u8> for DitherMode {
    type Error = DitherError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(DitherMode::None),
            1 => Ok(DitherMode::Burkes),
            2 => Ok(DitherMode::Ordered),
            3 => Ok(DitherMode::FloydSteinberg),
            4 => Ok(DitherMode::Atkinson),
            5 => Ok(DitherMode::Stucki),
            6 => Ok(DitherMode::Sierra),
            7 => Ok(DitherMode::SierraLite),
            8 => Ok(DitherMode::JarvisJudiceNinke),
            9 => Ok(DitherMode::Dizzy),
            _ => Err(DitherError::UnknownDitherMode(v)),
        }
    }
}

impl DitherMode {
    pub fn kernel(self) -> Option<&'static Kernel> {
        match self {
            DitherMode::None | DitherMode::Ordered | DitherMode::Dizzy => None,
            DitherMode::FloydSteinberg => Some(&FLOYD_STEINBERG),
            DitherMode::Burkes => Some(&BURKES),
            DitherMode::Atkinson => Some(&ATKINSON),
            DitherMode::Stucki => Some(&STUCKI),
            DitherMode::Sierra => Some(&SIERRA),
            DitherMode::SierraLite => Some(&SIERRA_LITE),
            DitherMode::JarvisJudiceNinke => Some(&JARVIS_JUDICE_NINKE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dizzy_is_mode_nine_and_has_no_kernel() {
        assert_eq!(DitherMode::Dizzy as u8, 9);
        assert_eq!(DitherMode::try_from(9u8), Ok(DitherMode::Dizzy));
        assert!(DitherMode::Dizzy.kernel().is_none(), "dizzy has no fixed kernel");
    }
}

use crate::algorithms::{
    Kernel, ATKINSON, BURKES, FLOYD_STEINBERG, JARVIS_JUDICE_NINKE, SIERRA, SIERRA_LITE, STUCKI,
};
use crate::error::DitherError;
use crate::srgb_palette::SrgbPalette;
use crate::tone_map;

/// Dithering algorithm. Bindings name modes by their snake_case [`Display`] form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DitherMode {
    /// Direct nearest-color mapping with no error diffusion. Intended for
    /// already-quantized graphics only. On limited palettes (especially BWR),
    /// continuous-tone photos or large flat mid-tone areas can map to an
    /// unexpected ink (e.g. a mid-gray region rendered as solid red); use an
    /// error-diffusion mode (e.g. `FloydSteinberg`, `Burkes`) for photographic input.
    None,
    #[default]
    Burkes,
    Ordered,
    FloydSteinberg,
    Atkinson,
    Stucki,
    Sierra,
    SierraLite,
    JarvisJudiceNinke,
    /// Error diffusion with pseudo-random traversal instead of a raster scan,
    /// diffusing error only to not-yet-quantized neighbours. Produces no
    /// directional structure. `serpentine` is ignored: there is no scan direction.
    Dizzy,
}

/// Dynamic range compression applied before dithering.
/// Only meaningful for measured palettes.
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
    pub fn apply(self, pixels: &mut [[f64; 3]], palette: &SrgbPalette) {
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
    pub fn apply(self, pixels: &mut [[f64; 3]], palette: &SrgbPalette) {
        match self {
            GamutCompression::None => {}
            GamutCompression::Auto => tone_map::gamut_compress(pixels, palette, 1.0),
            GamutCompression::Fixed(s) => if s > 0.0 {
                tone_map::gamut_compress(pixels, palette, s)
            }
        }
    }


}

const NAMES: [(DitherMode, &str); 10] = [
    (DitherMode::None, "none"),
    (DitherMode::Burkes, "burkes"),
    (DitherMode::Ordered, "ordered"),
    (DitherMode::FloydSteinberg, "floyd_steinberg"),
    (DitherMode::Atkinson, "atkinson"),
    (DitherMode::Stucki, "stucki"),
    (DitherMode::Sierra, "sierra"),
    (DitherMode::SierraLite, "sierra_lite"),
    (DitherMode::JarvisJudiceNinke, "jarvis_judice_ninke"),
    (DitherMode::Dizzy, "dizzy"),
];

impl std::str::FromStr for DitherMode {
    type Err = DitherError;

    /// Parse a snake_case mode name, e.g. `"floyd_steinberg"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NAMES
            .iter()
            .find(|(_, name)| *name == s)
            .map(|&(mode, _)| mode)
            .ok_or_else(|| DitherError::UnknownDitherMode(s.to_owned()))
    }
}

impl std::fmt::Display for DitherMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = NAMES.iter().find(|(mode, _)| mode == self).map(|(_, name)| *name).unwrap_or("unknown");
        f.write_str(name)
    }
}

impl DitherMode {
    /// Every mode, in declaration order.
    pub const ALL: [DitherMode; 10] = [
        DitherMode::None,
        DitherMode::Burkes,
        DitherMode::Ordered,
        DitherMode::FloydSteinberg,
        DitherMode::Atkinson,
        DitherMode::Stucki,
        DitherMode::Sierra,
        DitherMode::SierraLite,
        DitherMode::JarvisJudiceNinke,
        DitherMode::Dizzy,
    ];

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
    fn dizzy_is_named_and_has_no_kernel() {
        assert_eq!("dizzy".parse::<DitherMode>(), Ok(DitherMode::Dizzy));
        assert!(DitherMode::Dizzy.kernel().is_none(), "dizzy has no fixed kernel");
    }
}

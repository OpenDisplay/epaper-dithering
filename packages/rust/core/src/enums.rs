use crate::algorithms::{
    Kernel, ATKINSON, BURKES, FLOYD_STEINBERG, JARVIS_JUDICE_NINKE, SIERRA, SIERRA_LITE, STUCKI,
};
use crate::error::DitherError;

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

impl std::str::FromStr for DitherMode {
    type Err = DitherError;

    /// Parse a snake_case mode name, e.g. `"floyd_steinberg"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|mode| mode.name() == s).ok_or_else(|| DitherError::UnknownDitherMode(s.to_owned()))
    }
}

impl std::fmt::Display for DitherMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
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

    /// The snake_case name every binding uses for this mode.
    fn name(self) -> &'static str {
        match self {
            DitherMode::None => "none",
            DitherMode::Burkes => "burkes",
            DitherMode::Ordered => "ordered",
            DitherMode::FloydSteinberg => "floyd_steinberg",
            DitherMode::Atkinson => "atkinson",
            DitherMode::Stucki => "stucki",
            DitherMode::Sierra => "sierra",
            DitherMode::SierraLite => "sierra_lite",
            DitherMode::JarvisJudiceNinke => "jarvis_judice_ninke",
            DitherMode::Dizzy => "dizzy",
        }
    }

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

    /// `ALL` is maintained by hand and parsing goes through it, so a duplicate there would
    /// leave some mode unparseable by name.
    #[test]
    fn all_has_no_duplicate_modes_or_names() {
        let names: std::collections::BTreeSet<&str> = DitherMode::ALL.iter().map(|m| m.name()).collect();
        assert_eq!(names.len(), DitherMode::ALL.len());
    }
}

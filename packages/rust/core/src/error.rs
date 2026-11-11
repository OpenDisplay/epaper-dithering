use std::fmt;

/// Errors from untrusted dithering inputs: an image buffer of the wrong size or an
/// unknown mode name. Palette problems are [`crate::palette::PaletteError`].
#[derive(Debug, Clone, PartialEq)]
pub enum DitherError {
    /// `len` is not `width × height × 3` for any height, or `width` is zero.
    InvalidImage { len: usize, width: usize },
    UnknownDitherMode(String),
}

impl fmt::Display for DitherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DitherError::InvalidImage { len, width } => {
                write!(f, "pixel buffer of {len} bytes is not width ({width}) × height × 3 RGB")
            }
            DitherError::UnknownDitherMode(v) => write!(f, "unknown dither mode: {v:?}"),
        }
    }
}

impl std::error::Error for DitherError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_bad_mode() {
        let msg = DitherError::UnknownDitherMode("bayer".into()).to_string();
        assert!(msg.contains("bayer"), "message should include the bad value: {msg}");
    }
}

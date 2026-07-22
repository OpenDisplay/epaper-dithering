use std::fmt;

/// Errors from converting an untrusted integer discriminant into a core enum.
///
/// NOT the crate-wide error type: other modules define their own where the failure is a
/// different category (e.g. `composite::InvalidRgbaLength`, a typed struct that carries the
/// offending `len`). Deliberately kept separate rather than collapsed into one enum.
#[derive(Debug, PartialEq)]
pub enum DitherError {
    UnknownColorScheme(u8),
    UnknownDitherMode(u8),
}

impl fmt::Display for DitherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DitherError::UnknownColorScheme(v) => write!(f, "unknown color scheme: {v}"),
            DitherError::UnknownDitherMode(v) => write!(f, "unknown dither mode: {v}"),
        }
    }
}

impl std::error::Error for DitherError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unknown_scheme() {
        let e = DitherError::UnknownColorScheme(42);
        let msg = e.to_string();
        assert!(msg.contains("42"), "message should include the bad value: {msg}");
    }
}

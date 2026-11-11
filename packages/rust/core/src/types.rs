use crate::error::DitherError;

/// A flat RGB image buffer. Height is derived from data length and width.
#[derive(Debug, Clone, Copy)]
pub struct ImageBuffer<'a> {
    pub data: &'a [u8],
    pub width: usize,
    pub height: usize,
}

impl<'a> ImageBuffer<'a> {
    /// Create from flat RGB bytes (len = width × height × 3, height ≥ 1).
    pub fn new(data: &'a [u8], width: usize) -> Result<Self, DitherError> {
        let invalid = DitherError::InvalidImage { len: data.len(), width };
        if width == 0 || data.is_empty() || !data.len().is_multiple_of(width * 3) {
            return Err(invalid);
        }
        Ok(Self { data, width, height: data.len() / (width * 3) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_well_formed_buffer() {
        let data = [0u8; 2 * 3 * 3]; // 3×2 RGB
        assert_eq!(ImageBuffer::new(&data, 3).unwrap().height, 2);
    }

    #[test]
    fn new_rejects_zero_width() {
        assert!(ImageBuffer::new(&[0u8; 3], 0).is_err());
    }

    #[test]
    fn new_rejects_ragged_buffer() {
        // 7 bytes is neither a whole number of pixels nor rows for width 2.
        assert_eq!(ImageBuffer::new(&[0u8; 7], 2).unwrap_err(), DitherError::InvalidImage { len: 7, width: 2 });
    }
}

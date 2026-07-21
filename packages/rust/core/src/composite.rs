//! Alpha compositing shared by every language binding.
//!
//! RGBA input has to be flattened to RGB before dithering. Doing that separately in each
//! binding invites drift, so this is the single implementation: Python, JavaScript/WASM and
//! direct Rust callers all go through [`composite_rgba_on_white`].

use std::fmt;

/// The RGBA buffer handed to [`composite_rgba_on_white`] was not a whole number of pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidRgbaLength {
    /// The offending buffer length, in bytes.
    pub len: usize,
}

impl fmt::Display for InvalidRgbaLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RGBA buffer length ({}) must be a multiple of 4",
            self.len
        )
    }
}

impl std::error::Error for InvalidRgbaLength {}

/// Composite a flat RGBA buffer onto an opaque white background, returning flat RGB bytes.
///
/// Blending is `c · a + 255 · (1 − a)` per channel in nonlinear sRGB, rounded half away from
/// zero. Fully opaque pixels pass through unchanged; fully transparent pixels become white.
///
/// # Errors
///
/// Returns [`InvalidRgbaLength`] if `rgba.len()` is not a multiple of 4. Truncating a partial
/// trailing pixel would silently shorten the image, so it is rejected instead.
pub fn composite_rgba_on_white(rgba: &[u8]) -> Result<Vec<u8>, InvalidRgbaLength> {
    if !rgba.len().is_multiple_of(4) {
        return Err(InvalidRgbaLength { len: rgba.len() });
    }
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        let a = px[3] as f64 / 255.0;
        let inv = 1.0 - a;
        rgb.push((px[0] as f64 * a + 255.0 * inv).round() as u8);
        rgb.push((px[1] as f64 * a + 255.0 * inv).round() as u8);
        rgb.push((px[2] as f64 * a + 255.0 * inv).round() as u8);
    }
    Ok(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_pixels_pass_through_unchanged() {
        let rgba = [128, 64, 192, 255, 0, 0, 0, 255, 255, 255, 255, 255];
        assert_eq!(
            composite_rgba_on_white(&rgba).unwrap(),
            vec![128, 64, 192, 0, 0, 0, 255, 255, 255],
        );
    }

    #[test]
    fn transparent_pixels_become_white() {
        let rgba = [128, 64, 192, 0];
        assert_eq!(composite_rgba_on_white(&rgba).unwrap(), vec![255, 255, 255]);
    }

    /// Literal expectations mirrored byte-for-byte in the Python and JavaScript suites.
    #[test]
    fn cross_language_reference_vector() {
        let rgba: Vec<u8> = vec![
            128, 128, 128, 0,
            128, 128, 128, 1,
            128, 128, 128, 127,
            128, 128, 128, 128,
            128, 128, 128, 254,
            128, 128, 128, 255,
            0, 64, 200, 1,
            0, 64, 200, 127,
            0, 64, 200, 128,
            0, 64, 200, 254,
            17, 200, 3, 63,
            250, 5, 130, 191,
        ];
        assert_eq!(
            composite_rgba_on_white(&rgba).unwrap(),
            vec![
                255, 255, 255,
                255, 255, 255,
                192, 192, 192,
                191, 191, 191,
                128, 128, 128,
                128, 128, 128,
                254, 254, 255,
                128, 160, 228,
                127, 159, 227,
                1, 65, 200,
                196, 241, 193,
                251, 68, 161,
            ],
        );
    }

    #[test]
    fn empty_buffer_is_ok() {
        assert_eq!(composite_rgba_on_white(&[]).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn length_not_multiple_of_four_is_rejected() {
        for len in [1usize, 2, 3, 5, 7] {
            let buf = vec![128u8; len];
            let err = composite_rgba_on_white(&buf).unwrap_err();
            assert_eq!(err, InvalidRgbaLength { len });
            assert!(err.to_string().contains(&len.to_string()));
        }
    }
}

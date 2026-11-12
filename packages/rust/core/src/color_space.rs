/// sRGB [0–255] → linear [0.0–1.0]. IEC 61966-2-1 piecewise transfer function.
pub fn srgb_channel_to_linear(value: u8) -> f64 {
    srgb_fraction_to_linear(value as f64 / 255.0)
}

/// `srgb_channel_to_linear` for every byte value, so per-pixel code indexes a table instead of
/// calling `powf`. The entries are the function's own results, so lookups are exact.
pub(crate) fn srgb_to_linear_lut() -> [f64; 256] {
    std::array::from_fn(|v| srgb_channel_to_linear(v as u8))
}

/// Continuous sRGB fraction [0.0–1.0] → linear [0.0–1.0]. Same gamma curve as
/// `srgb_channel_to_linear` but operates on continuous input — useful when sub-byte
/// precision is needed (e.g. ordered dither perturbs pixels in sRGB-fraction space
/// before the linear conversion).
pub fn srgb_fraction_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear [0.0–1.0] → continuous sRGB fraction [0.0–1.0]. Inverse of
/// `srgb_fraction_to_linear`; the continuous counterpart of `linear_channel_to_srgb`,
/// useful when a gamma-encoded value is needed without quantizing to a byte (e.g. applying
/// a tone curve about the perceptual mid-gray).
pub fn linear_fraction_to_srgb(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Linear [0.0–1.0] → sRGB [0–255]. Inverse of `srgb_channel_to_linear`.
pub fn linear_channel_to_srgb(linear: f64) -> u8 {
    (linear_fraction_to_srgb(linear) * 255.0).round() as u8
}


#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn black_and_white_are_identity() {
        assert_relative_eq!(srgb_channel_to_linear(0), 0.0);
        assert_relative_eq!(srgb_channel_to_linear(255), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn roundtrip() {
        for v in [0u8, 1, 10, 127, 128, 254, 255] {
            assert_eq!(linear_channel_to_srgb(srgb_channel_to_linear(v)), v);
        }
    }
}

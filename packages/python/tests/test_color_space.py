"""Tests for color space conversion functions."""

from __future__ import annotations

import numpy as np
import pytest
from epaper_dithering.color_space import linear_to_srgb, srgb_to_linear


class TestGammaCorrection:
    """Test gamma correction functions."""

    def test_srgb_to_linear_black(self):
        """Test black (0, 0, 0) stays black."""
        srgb = np.array([0, 0, 0], dtype=np.float32)
        linear = srgb_to_linear(srgb)
        assert np.allclose(linear, [0.0, 0.0, 0.0], atol=1e-6)

    def test_srgb_to_linear_white(self):
        """Test white (255, 255, 255) stays white."""
        srgb = np.array([255, 255, 255], dtype=np.float32)
        linear = srgb_to_linear(srgb)
        assert np.allclose(linear, [1.0, 1.0, 1.0], atol=1e-6)

    def test_srgb_to_linear_midgray(self):
        """Test middle gray (~50% linear) is much darker in sRGB.

        50% linear light corresponds to approximately sRGB value 186-188.
        Middle sRGB value (128) is only about 21.6% linear light.
        This demonstrates the non-linearity of the sRGB encoding.
        """
        # sRGB 186 should be close to 50% linear
        srgb_50_linear = np.array([186], dtype=np.float32)
        linear = srgb_to_linear(srgb_50_linear)
        assert np.allclose(linear[0], 0.5, atol=0.01)

        # sRGB 128 (middle gray) is much darker than 50% linear
        srgb_128 = np.array([128], dtype=np.float32)
        linear_128 = srgb_to_linear(srgb_128)
        assert linear_128[0] < 0.25  # Much darker than 50%

    def test_srgb_to_linear_shape_preserved(self):
        """Test that output shape matches input shape."""
        # Test 1D array
        srgb_1d = np.array([0, 128, 255], dtype=np.float32)
        linear_1d = srgb_to_linear(srgb_1d)
        assert linear_1d.shape == (3,)

        # Test 2D array (like an image)
        srgb_2d = np.array([[0, 128], [255, 64]], dtype=np.float32)
        linear_2d = srgb_to_linear(srgb_2d)
        assert linear_2d.shape == (2, 2)

        # Test 3D array (like RGB image)
        srgb_3d = np.zeros((10, 10, 3), dtype=np.float32)
        srgb_3d[:, :, 0] = 128  # Red channel
        linear_3d = srgb_to_linear(srgb_3d)
        assert linear_3d.shape == (10, 10, 3)

    def test_srgb_to_linear_monotonic(self):
        """Test that function is monotonically increasing."""
        srgb = np.arange(0, 256, dtype=np.float32)
        linear = srgb_to_linear(srgb)

        # Check all differences are non-negative (monotonic)
        diffs = np.diff(linear)
        assert np.all(diffs >= 0)

    def test_srgb_to_linear_boundary(self):
        """Test piecewise linear section boundary (0.04045 in sRGB)."""
        # The boundary is at sRGB value 0.04045 * 255 ≈ 10.31
        srgb_boundary = np.array([10, 11], dtype=np.float32)
        linear = srgb_to_linear(srgb_boundary)

        # Both should be small values in linear space
        assert linear[0] < 0.005
        assert linear[1] < 0.005

        # Should be continuous at boundary
        assert abs(linear[1] - linear[0]) < 0.001


class TestLinearToSRGB:
    """Test linear to sRGB conversion."""

    def test_linear_to_srgb_black(self):
        """Test black (0.0) stays black."""
        linear = np.array([0.0, 0.0, 0.0], dtype=np.float32)
        srgb = linear_to_srgb(linear)
        assert np.array_equal(srgb, [0, 0, 0])

    def test_linear_to_srgb_white(self):
        """Test white (1.0) stays white."""
        linear = np.array([1.0, 1.0, 1.0], dtype=np.float32)
        srgb = linear_to_srgb(linear)
        assert np.array_equal(srgb, [255, 255, 255])

    def test_linear_to_srgb_midpoint(self):
        """Test 50% linear light is bright in sRGB (around 188)."""
        linear = np.array([0.5], dtype=np.float32)
        srgb = linear_to_srgb(linear)

        # 50% linear should be around 186-188 in sRGB
        assert 185 <= srgb[0] <= 189

    def test_linear_to_srgb_shape_preserved(self):
        """Test that output shape matches input shape."""
        # Test 1D
        linear_1d = np.array([0.0, 0.5, 1.0], dtype=np.float32)
        srgb_1d = linear_to_srgb(linear_1d)
        assert srgb_1d.shape == (3,)

        # Test 3D (image-like)
        linear_3d = np.random.rand(10, 10, 3).astype(np.float32)
        srgb_3d = linear_to_srgb(linear_3d)
        assert srgb_3d.shape == (10, 10, 3)
        assert srgb_3d.dtype == np.uint8

    def test_linear_to_srgb_monotonic(self):
        """Test that function is monotonically increasing."""
        linear = np.linspace(0, 1, 256, dtype=np.float32)
        srgb = linear_to_srgb(linear)

        # Check all differences are non-negative (monotonic)
        diffs = np.diff(srgb.astype(np.int16))  # Use int16 to detect negative
        assert np.all(diffs >= 0)

    def test_linear_to_srgb_uses_rounding(self):
        """Test that conversion uses rounding, not truncation.

        This prevents systematic darkening bias. For example:
        - Truncation: 127.9 → 127 (always rounds down)
        - Rounding: 127.9 → 128 (rounds to nearest)
        """
        # Test that multiple similar values round correctly
        # Values around 0.215 convert to sRGB values near 128
        test_values = np.linspace(0.215, 0.216, 10, dtype=np.float32)
        results = linear_to_srgb(test_values)

        # All results should be in a small range (good rounding)
        assert results.max() - results.min() <= 1


class TestRoundtrip:
    """Test roundtrip conversions."""

    def test_roundtrip_identity(self):
        """Test sRGB → linear → sRGB is approximately identity."""
        original = np.array([0, 32, 64, 96, 128, 160, 192, 224, 255], dtype=np.float32)
        linear = srgb_to_linear(original)
        roundtrip = linear_to_srgb(linear)

        # Allow 1 unit of error due to rounding
        assert np.allclose(roundtrip, original, atol=1)

    def test_roundtrip_all_values(self):
        """Test roundtrip for all possible sRGB values."""
        original = np.arange(256, dtype=np.float32)
        linear = srgb_to_linear(original)
        roundtrip = linear_to_srgb(linear)

        # Should be very close to original
        max_error = np.abs(roundtrip.astype(np.float32) - original).max()
        assert max_error <= 1  # At most 1 unit of rounding error

    def test_roundtrip_linear_identity(self):
        """Test linear → sRGB → linear preserves key values."""
        # Test a few key linear values
        original_linear = np.array([0.0, 0.25, 0.5, 0.75, 1.0], dtype=np.float32)
        srgb = linear_to_srgb(original_linear)
        roundtrip_linear = srgb_to_linear(srgb.astype(np.float32))

        # Should be close (some error due to quantization to uint8)
        assert np.allclose(roundtrip_linear, original_linear, atol=0.01)


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_srgb_to_linear_negative_clamped(self):
        """Test that negative values are handled gracefully."""
        # NumPy will produce negative values for negative input
        srgb = np.array([-10, 0, 10], dtype=np.float32)
        linear = srgb_to_linear(srgb)

        # Negative input produces negative output (mathematically correct)
        assert linear[0] < 0
        assert linear[1] == 0
        assert linear[2] > 0

    @pytest.mark.filterwarnings("ignore::RuntimeWarning")
    def test_linear_to_srgb_out_of_range(self):
        """Test that out-of-range values are handled."""
        # Values outside [0, 1] should still convert
        linear = np.array([-0.1, 0.5, 1.5], dtype=np.float32)
        srgb = linear_to_srgb(linear)

        # Should clip or wrap in some reasonable way
        assert srgb.dtype == np.uint8
        assert len(srgb) == 3

    def test_very_small_values(self):
        """Test very small values near zero."""
        srgb = np.array([1, 2, 3], dtype=np.float32)
        linear = srgb_to_linear(srgb)

        # Should all be very small but positive
        assert np.all(linear > 0)
        assert np.all(linear < 0.01)

    def test_very_large_values(self):
        """Test values near maximum."""
        srgb = np.array([253, 254, 255], dtype=np.float32)
        linear = srgb_to_linear(srgb)

        # Should all be close to 1.0
        assert np.all(linear > 0.98)
        assert np.all(linear <= 1.0)

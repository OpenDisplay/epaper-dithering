"""Tests for error diffusion kernel definitions."""

import pytest
from epaper_dithering.algorithms import (
    ATKINSON,
    BURKES,
    FLOYD_STEINBERG,
    JARVIS_JUDICE_NINKE,
    SIERRA,
    SIERRA_LITE,
    STUCKI,
)

ALL_KERNELS = [
    FLOYD_STEINBERG,
    BURKES,
    SIERRA,
    SIERRA_LITE,
    STUCKI,
    JARVIS_JUDICE_NINKE,
]


class TestKernelWeights:
    """Validate error diffusion kernel weight correctness."""

    @pytest.mark.parametrize("kernel", ALL_KERNELS, ids=lambda k: k.name)
    def test_weights_sum_to_divisor(self, kernel):
        """Kernel weights must sum to divisor for full error propagation.

        If weights don't sum to divisor, error is lost (sum < divisor)
        or amplified (sum > divisor), causing brightness drift.
        This would have caught the Burkes bug where hallucinated weights
        summed to 104 with divisor 200 (52% propagation instead of 100%).
        """
        weight_sum = sum(weight for _, _, weight in kernel.offsets)
        assert weight_sum == kernel.divisor, f"{kernel.name}: weights sum to {weight_sum}, expected {kernel.divisor}"

    def test_atkinson_propagates_75_percent(self):
        """Atkinson intentionally propagates only 6/8 (75%) of error.

        This gives it a distinctive lighter appearance. The divisor is 8
        but weights sum to 6.
        """
        weight_sum = sum(weight for _, _, weight in ATKINSON.offsets)
        assert ATKINSON.divisor == 8
        assert weight_sum == 6, f"Atkinson should propagate 6/8 of error, weights sum to {weight_sum}"

    @pytest.mark.parametrize("kernel", ALL_KERNELS + [ATKINSON], ids=lambda k: k.name)
    def test_all_weights_positive(self, kernel):
        """All kernel weights must be positive."""
        for dx, dy, weight in kernel.offsets:
            assert weight > 0, f"{kernel.name}: weight at ({dx},{dy}) is {weight}"

    @pytest.mark.parametrize("kernel", ALL_KERNELS + [ATKINSON], ids=lambda k: k.name)
    def test_offsets_are_forward_only(self, kernel):
        """Kernel offsets must only distribute to unprocessed pixels.

        For left-to-right scanning: dx > 0 on same row (dy=0),
        or any dx on future rows (dy > 0).
        """
        for dx, dy, _weight in kernel.offsets:
            assert dy >= 0, f"{kernel.name}: offset ({dx},{dy}) points backwards"
            if dy == 0:
                assert dx > 0, f"{kernel.name}: offset ({dx},{dy}) on current row must be forward"

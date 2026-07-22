//! Dizzy dithering: error diffusion with pseudo-random traversal.

// `dizzy_order` and its helpers are not yet wired into the public dithering API
// (that lands in a later task); until then they are only reachable from tests.
#![allow(dead_code)]

// ── Traversal ─────────────────────────────────────────────────────────────────
//
// A stateless bijective permutation of `0..2^bits`, so the walk needs no shuffled
// index array. Multiplication by an odd number is invertible modulo 2^k (odd
// numbers are units in that ring) and XOR by a constant is self-inverse, so five
// rounds of (multiply, mask, xor) compose to a bijection. Indices >= n are skipped.
//
// FROZEN: changing either table changes every image this mode has ever produced.
const ODD: [u64; 5] = [0x2545_F491, 0x9E37_79B1, 0x85EB_CA6B, 0xC2B2_AE35, 0x27D4_EB2F];
const XOR: [u64; 5] = [0x1656_67B1, 0xD3A2_646C, 0xFD70_46C5, 0xB55A_4F09, 0x1B87_3593];

/// Smallest `bits` such that `2^bits >= n`. `bits_for(1) == 0`.
fn bits_for(n: usize) -> u32 {
    debug_assert!(n > 0, "bits_for requires a non-empty image");
    usize::BITS - (n - 1).leading_zeros()
}

fn permute(i: u64, mask: u64) -> u64 {
    let mut p = i;
    for r in 0..ODD.len() {
        p = p.wrapping_mul(ODD[r]) & mask;
        p ^= XOR[r] & mask;
    }
    p
}

/// Yields every index in `0..n` exactly once, in pseudo-random order.
pub(crate) fn dizzy_order(n: usize) -> impl Iterator<Item = usize> {
    let bits = bits_for(n);
    let mask = if bits >= u64::BITS { u64::MAX } else { (1u64 << bits) - 1 };
    (0..=mask).filter_map(move |i| {
        let p = permute(i, mask) as usize;
        (p < n).then_some(p)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_visits_every_index_exactly_once() {
        // Powers of two, 2^k+1 (worst-case rejection), primes, and degenerate shapes.
        for n in [1usize, 2, 3, 4, 5, 7, 8, 9, 16, 17, 31, 64, 100, 255, 256, 257, 1000, 4096] {
            let mut seen = vec![0u32; n];
            let mut count = 0usize;
            for p in dizzy_order(n) {
                assert!(p < n, "n={n}: yielded out-of-range index {p}");
                seen[p] += 1;
                count += 1;
            }
            assert_eq!(count, n, "n={n}: walk yielded {count} indices, expected {n}");
            assert!(
                seen.iter().all(|&c| c == 1),
                "n={n}: some index was visited {:?} times, expected exactly once each",
                seen.iter().max()
            );
        }
    }

    #[test]
    fn walk_is_not_the_identity() {
        // A permutation that happened to be the identity would pass the bijection
        // test while making this mode a plain raster scan.
        let order: Vec<usize> = dizzy_order(256).collect();
        let identity: Vec<usize> = (0..256).collect();
        assert_ne!(order, identity, "walk degenerated into raster order");
    }
}

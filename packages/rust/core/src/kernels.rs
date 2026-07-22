//! Error-diffusion kernel definitions.

// ── Kernel definition ─────────────────────────────────────────────────────────

/// Pixel offset + pre-divided weight for one kernel entry.
pub struct KernelOffset {
    pub dx: i32,
    pub dy: i32,
    pub weight: f64,
}

pub struct Kernel {
    pub offsets: &'static [KernelOffset],
}

// ── Kernel constants ──────────────────────────────────────────────────────────

pub static FLOYD_STEINBERG: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 7.0 / 16.0 },
        KernelOffset { dx: -1, dy: 1, weight: 3.0 / 16.0 },
        KernelOffset { dx:  0, dy: 1, weight: 5.0 / 16.0 },
        KernelOffset { dx:  1, dy: 1, weight: 1.0 / 16.0 },
    ],
};

pub static ATKINSON: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 1.0 / 8.0 },
        KernelOffset { dx:  2, dy: 0, weight: 1.0 / 8.0 },
        KernelOffset { dx: -1, dy: 1, weight: 1.0 / 8.0 },
        KernelOffset { dx:  0, dy: 1, weight: 1.0 / 8.0 },
        KernelOffset { dx:  1, dy: 1, weight: 1.0 / 8.0 },
        KernelOffset { dx:  0, dy: 2, weight: 1.0 / 8.0 },
    ],
};

pub static BURKES: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 8.0 / 32.0 },
        KernelOffset { dx:  2, dy: 0, weight: 4.0 / 32.0 },
        KernelOffset { dx: -2, dy: 1, weight: 2.0 / 32.0 },
        KernelOffset { dx: -1, dy: 1, weight: 4.0 / 32.0 },
        KernelOffset { dx:  0, dy: 1, weight: 8.0 / 32.0 },
        KernelOffset { dx:  1, dy: 1, weight: 4.0 / 32.0 },
        KernelOffset { dx:  2, dy: 1, weight: 2.0 / 32.0 },
    ],
};

pub static STUCKI: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 8.0 / 42.0 },
        KernelOffset { dx:  2, dy: 0, weight: 4.0 / 42.0 },
        KernelOffset { dx: -2, dy: 1, weight: 2.0 / 42.0 },
        KernelOffset { dx: -1, dy: 1, weight: 4.0 / 42.0 },
        KernelOffset { dx:  0, dy: 1, weight: 8.0 / 42.0 },
        KernelOffset { dx:  1, dy: 1, weight: 4.0 / 42.0 },
        KernelOffset { dx:  2, dy: 1, weight: 2.0 / 42.0 },
        KernelOffset { dx: -2, dy: 2, weight: 1.0 / 42.0 },
        KernelOffset { dx: -1, dy: 2, weight: 2.0 / 42.0 },
        KernelOffset { dx:  0, dy: 2, weight: 4.0 / 42.0 },
        KernelOffset { dx:  1, dy: 2, weight: 2.0 / 42.0 },
        KernelOffset { dx:  2, dy: 2, weight: 1.0 / 42.0 },
    ],
};

pub static SIERRA: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 5.0 / 32.0 },
        KernelOffset { dx:  2, dy: 0, weight: 3.0 / 32.0 },
        KernelOffset { dx: -2, dy: 1, weight: 2.0 / 32.0 },
        KernelOffset { dx: -1, dy: 1, weight: 4.0 / 32.0 },
        KernelOffset { dx:  0, dy: 1, weight: 5.0 / 32.0 },
        KernelOffset { dx:  1, dy: 1, weight: 4.0 / 32.0 },
        KernelOffset { dx:  2, dy: 1, weight: 2.0 / 32.0 },
        KernelOffset { dx: -1, dy: 2, weight: 2.0 / 32.0 },
        KernelOffset { dx:  0, dy: 2, weight: 3.0 / 32.0 },
        KernelOffset { dx:  1, dy: 2, weight: 2.0 / 32.0 },
    ],
};

pub static SIERRA_LITE: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 2.0 / 4.0 },
        KernelOffset { dx: -1, dy: 1, weight: 1.0 / 4.0 },
        KernelOffset { dx:  0, dy: 1, weight: 1.0 / 4.0 },
    ],
};

pub static JARVIS_JUDICE_NINKE: Kernel = Kernel {
    offsets: &[
        KernelOffset { dx:  1, dy: 0, weight: 7.0 / 48.0 },
        KernelOffset { dx:  2, dy: 0, weight: 5.0 / 48.0 },
        KernelOffset { dx: -2, dy: 1, weight: 3.0 / 48.0 },
        KernelOffset { dx: -1, dy: 1, weight: 5.0 / 48.0 },
        KernelOffset { dx:  0, dy: 1, weight: 7.0 / 48.0 },
        KernelOffset { dx:  1, dy: 1, weight: 5.0 / 48.0 },
        KernelOffset { dx:  2, dy: 1, weight: 3.0 / 48.0 },
        KernelOffset { dx: -2, dy: 2, weight: 1.0 / 48.0 },
        KernelOffset { dx: -1, dy: 2, weight: 3.0 / 48.0 },
        KernelOffset { dx:  0, dy: 2, weight: 5.0 / 48.0 },
        KernelOffset { dx:  1, dy: 2, weight: 3.0 / 48.0 },
        KernelOffset { dx:  2, dy: 2, weight: 1.0 / 48.0 },
    ],
};

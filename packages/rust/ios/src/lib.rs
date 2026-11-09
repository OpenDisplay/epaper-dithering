//! C-ABI FFI wrapper around `epaper-dithering-core`, intended for the iOS/Swift app.
//!
//! Scope is deliberately minimal — this exposes **only** the dithering step (palette
//! matching in OKLab + error diffusion). The tone/gamut/exposure/saturation
//! pre-processing pipeline is intentionally *not* surfaced here: the OpenDisplay iOS app
//! keeps its own tone-compression pass in Swift for now, so it hands us pixels that are
//! already pre-processed and we run pure matching + diffusion at `DitherConfig` defaults
//! (all pre-processing off).
//!
//! ## Contract
//!
//! - **Output is caller-allocated.** The number of output indices is deterministic —
//!   exactly `width * height`, one `u8` palette index per pixel — so Swift allocates the
//!   buffer and we fill it. There is no allocation handed across the boundary and no free
//!   function to call.
//! - **Indices are into the palette you pass.** Feed the app's palette in the app's own
//!   index order and the returned indices line up with the app's wire-format packing with
//!   no remap. When a `canonical` palette is supplied, matching uses `matching` (e.g. a
//!   measured palette) while already-displayable exact colors pass through via `canonical`;
//!   the returned indices are still in `matching` order (which the app keeps index-aligned
//!   with `canonical`).
//! - **Panics never cross the boundary.** The body runs inside `catch_unwind`; a panic is
//!   converted to [`ED_ERR_PANIC`].
//! - **Options mirror the core's `DitherConfig`.** [`EdDitherOptions`] is a `#[repr(C)]`
//!   struct; start from [`ed_dither_options_default`] (the core's defaults) and change only
//!   what you need, so tuning in the core reaches Swift without hard-coded numbers.
//! - **Errors are status codes**, never exceptions. `0` is success; negative values are the
//!   `ED_ERR_*` constants below.

use std::panic::{self, AssertUnwindSafe};

use epaper_dithering_core::{
    DitherConfig, dither, dither_with_canonical,
    dbs::DbsParams,
    enums::DitherMode,
    palettes::Palette,
    types::ImageBuffer,
};

/// Success.
pub const ED_OK: i32 = 0;
/// A required pointer argument was null.
pub const ED_ERR_NULL_POINTER: i32 = -1;
/// `width` was zero.
pub const ED_ERR_BAD_WIDTH: i32 = -2;
/// `pixels_len` is not a multiple of 3, or not a whole number of `width`-pixel rows.
pub const ED_ERR_BAD_PIXELS: i32 = -3;
/// A palette byte length was not a multiple of 3, held fewer than 2 colors, or had an
/// out-of-range accent index.
pub const ED_ERR_BAD_PALETTE: i32 = -4;
/// `out_len` does not equal `width * height`.
pub const ED_ERR_BAD_OUTPUT_LEN: i32 = -5;
/// `mode_id` is not a known [`DitherMode`] discriminant.
pub const ED_ERR_BAD_MODE: i32 = -6;
/// A panic was caught inside the FFI body.
pub const ED_ERR_PANIC: i32 = -7;
/// `opts` asked for DBS (`dbs_max_passes > 0`) with `dbs_viewing_distance_cm` or `dbs_ppi`
/// not finite and greater than 0.
pub const ED_ERR_BAD_DBS: i32 = -8;

/// Dithering options, the C mirror of the parts of `DitherConfig` this wrapper exposes.
///
/// Get one from [`ed_dither_options_default`] and change fields from there. Adding a field is
/// an ABI change: bump [`ed_abi_version`] and the header together.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdDitherOptions {
    /// [`DitherMode`] discriminant: 0 = None, 1 = Burkes, 2 = Ordered, 3 = FloydSteinberg,
    /// 4 = Atkinson, 5 = Stucki, 6 = Sierra, 7 = SierraLite, 8 = JarvisJudiceNinke, 9 = Dizzy.
    pub mode_id: u8,
    /// Serpentine scanning for error-diffusion modes.
    pub serpentine: bool,
    /// Direct Binary Search refinement after dithering: upper bound on full-image passes
    /// (refinement stops earlier once a pass changes nothing). `0` disables DBS. Opt-in and
    /// much slower than diffusion alone.
    pub dbs_max_passes: u32,
    /// DBS eye model: viewer-to-panel distance in centimetres. Ignored when DBS is off.
    pub dbs_viewing_distance_cm: f64,
    /// DBS eye model: panel pixel density in pixels per inch. Ignored when DBS is off.
    pub dbs_ppi: f64,
}

impl EdDitherOptions {
    /// Translate into the core's config. `Err` carries the `ED_ERR_*` code.
    fn to_config(self) -> Result<DitherConfig, i32> {
        let mode = DitherMode::try_from(self.mode_id).map_err(|_| ED_ERR_BAD_MODE)?;
        let dbs = if self.dbs_max_passes == 0 {
            None
        } else {
            let params = DbsParams {
                viewing_distance_cm: self.dbs_viewing_distance_cm,
                ppi: self.dbs_ppi,
                max_passes: self.dbs_max_passes,
            };
            params.validate().map_err(|_| ED_ERR_BAD_DBS)?;
            Some(params)
        };
        // Pre-processing intentionally left at defaults (off) — tone/gamut stay in the app.
        Ok(DitherConfig { mode, serpentine: self.serpentine, dbs, ..Default::default() })
    }
}

/// The core's defaults: `DitherConfig::default()`'s mode and serpentine, DBS off, and
/// `DbsParams::default()`'s viewing geometry pre-filled. To enable DBS, set `dbs_max_passes`
/// (e.g. to [`ed_dither_options_default_dbs_passes`]) and, ideally, the panel's real `dbs_ppi`.
#[unsafe(no_mangle)]
pub extern "C" fn ed_dither_options_default() -> EdDitherOptions {
    let config = DitherConfig::default();
    let dbs = DbsParams::default();
    EdDitherOptions {
        mode_id: config.mode as u8,
        serpentine: config.serpentine,
        dbs_max_passes: 0,
        dbs_viewing_distance_cm: dbs.viewing_distance_cm,
        dbs_ppi: dbs.ppi,
    }
}

/// The core's default DBS pass budget (`DbsParams::default().max_passes`), for turning DBS on
/// without hard-coding it.
#[unsafe(no_mangle)]
pub extern "C" fn ed_dither_options_default_dbs_passes() -> u32 {
    DbsParams::default().max_passes
}

/// Interpret a `*const u8` + length as a palette of RGB triples.
///
/// Returns `Ok(None)` when `len == 0` (meaning "no palette supplied"), `Ok(Some(palette))`
/// for a valid palette, or `Err(code)` for a malformed one.
fn build_palette(ptr: *const u8, len: usize, accent_idx: usize) -> Result<Option<Palette>, i32> {
    if len == 0 {
        return Ok(None);
    }
    if ptr.is_null() {
        return Err(ED_ERR_NULL_POINTER);
    }
    if !len.is_multiple_of(3) {
        return Err(ED_ERR_BAD_PALETTE);
    }
    // SAFETY: caller guarantees `ptr` is valid for `len` bytes; checked non-null above.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let colors: Vec<[u8; 3]> = bytes.as_chunks::<3>().0.to_vec();
    if colors.len() < 2 || accent_idx >= colors.len() {
        return Err(ED_ERR_BAD_PALETTE);
    }
    Ok(Some(Palette::new(colors, accent_idx)))
}

/// Dither a flat RGB image to palette indices, optionally refined with DBS.
///
/// # Parameters
/// - `pixels` / `pixels_len`: flat sRGB bytes, `len = width * height * 3`.
/// - `width`: image width in pixels (`> 0`).
/// - `matching_palette` / `matching_len` / `matching_accent`: the palette matched against,
///   in the caller's index order (`len` a multiple of 3, ≥ 2 colors). Required.
/// - `canonical_palette` / `canonical_len` / `canonical_accent`: optional ideal-color
///   palette for exact-pixel passthrough. Pass `canonical_len == 0` to disable (plain
///   matching against `matching_palette`).
/// - `opts`: algorithm, scan order and DBS settings; see [`EdDitherOptions`]. Required.
/// - `out` / `out_len`: caller-allocated output, `out_len` must equal `width * height`.
///
/// # Returns
/// [`ED_OK`] on success (and `out` is filled), or a negative `ED_ERR_*` code on failure
/// (and `out` is left untouched).
///
/// # Safety
/// All pointers must either be null (where documented as optional) or valid for their
/// stated lengths for the duration of the call. `opts` must point to a valid
/// [`EdDitherOptions`]. `out` must be writable for `out_len` bytes.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn ed_dither(
    pixels: *const u8,
    pixels_len: usize,
    width: usize,
    matching_palette: *const u8,
    matching_len: usize,
    matching_accent: usize,
    canonical_palette: *const u8,
    canonical_len: usize,
    canonical_accent: usize,
    opts: *const EdDitherOptions,
    out: *mut u8,
    out_len: usize,
) -> i32 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if pixels.is_null() || out.is_null() || opts.is_null() {
            return ED_ERR_NULL_POINTER;
        }
        if width == 0 {
            return ED_ERR_BAD_WIDTH;
        }
        if !pixels_len.is_multiple_of(3) || !(pixels_len / 3).is_multiple_of(width) {
            return ED_ERR_BAD_PIXELS;
        }
        let pixel_count = pixels_len / 3;
        if out_len != pixel_count {
            return ED_ERR_BAD_OUTPUT_LEN;
        }

        // SAFETY: validated non-null above; the caller guarantees it points to a valid struct.
        let config = match unsafe { *opts }.to_config() {
            Ok(c) => c,
            Err(code) => return code,
        };

        let matching = match build_palette(matching_palette, matching_len, matching_accent) {
            Ok(Some(p)) => p,
            // A matching palette is required — treat "none supplied" as malformed.
            Ok(None) => return ED_ERR_BAD_PALETTE,
            Err(code) => return code,
        };
        let canonical = match build_palette(canonical_palette, canonical_len, canonical_accent) {
            Ok(c) => c,
            Err(code) => return code,
        };

        // SAFETY: validated non-null and length above.
        let pixel_bytes = unsafe { std::slice::from_raw_parts(pixels, pixels_len) };
        let img = ImageBuffer::new(pixel_bytes, width);

        let indices = match canonical {
            Some(canon) => dither_with_canonical(&img, &matching, &canon, config),
            None => dither(&img, &matching, config),
        };

        debug_assert_eq!(indices.len(), pixel_count);
        // SAFETY: `out` validated non-null and `out_len == pixel_count == indices.len()`.
        unsafe {
            std::ptr::copy_nonoverlapping(indices.as_ptr(), out, indices.len());
        }
        ED_OK
    }));

    result.unwrap_or(ED_ERR_PANIC)
}

/// ABI version of this wrapper. Bump when the exported surface changes so the Swift side
/// can assert compatibility at load time.
///
/// - 1: `ed_dither(…, mode_id, serpentine, out, out_len)`.
/// - 2: options moved into [`EdDitherOptions`] (adds DBS); `ed_dither_options_default*` added.
#[unsafe(no_mangle)]
pub extern "C" fn ed_abi_version() -> u32 {
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(mode: DitherMode) -> EdDitherOptions {
        EdDitherOptions { mode_id: mode as u8, ..ed_dither_options_default() }
    }

    /// Call `ed_dither` with no canonical palette; returns (status, output).
    fn run(pixels: &[u8], width: usize, palette: &[u8], accent: usize, o: &EdDitherOptions) -> (i32, Vec<u8>) {
        let mut out = vec![0u8; pixels.len() / 3];
        let code = unsafe {
            ed_dither(
                pixels.as_ptr(), pixels.len(), width,
                palette.as_ptr(), palette.len(), accent,
                std::ptr::null(), 0, 0,
                o, out.as_mut_ptr(), out.len(),
            )
        };
        (code, out)
    }

    /// A smooth gradient so error diffusion (and DBS) actually have something to do.
    fn gradient(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                let v = ((x + y) * 255 / (width + height - 2)) as u8;
                pixels.extend_from_slice(&[v, v / 2, 255 - v]);
            }
        }
        pixels
    }

    const BWR: [u8; 9] = [0, 0, 0, 255, 255, 255, 255, 0, 0]; // black, white, red (app order)

    #[test]
    fn defaults_mirror_the_core() {
        let o = ed_dither_options_default();
        let config = DitherConfig::default();
        let dbs = DbsParams::default();
        assert_eq!(o.mode_id, config.mode as u8);
        assert_eq!(o.serpentine, config.serpentine);
        assert_eq!(o.dbs_max_passes, 0, "DBS is opt-in");
        assert_eq!(o.dbs_viewing_distance_cm, dbs.viewing_distance_cm);
        assert_eq!(o.dbs_ppi, dbs.ppi);
        assert_eq!(ed_dither_options_default_dbs_passes(), dbs.max_passes);
    }

    /// Solid-red 2×2 against a black/white/red palette (app order) must map every pixel to
    /// index 2 (red), exercising the exact-color bypass through the FFI.
    #[test]
    fn solid_red_maps_to_red_index() {
        let pixels: Vec<u8> = std::iter::repeat_n([255u8, 0, 0], 4).flatten().collect();
        let (code, out) = run(&pixels, 2, &BWR, 2, &opts(DitherMode::Burkes));
        assert_eq!(code, ED_OK);
        assert_eq!(out, [2, 2, 2, 2]);
    }

    /// The FFI must produce byte-identical output to calling `core::dither` directly —
    /// this is the whole point of the wrapper (no logic of its own).
    #[test]
    fn ffi_matches_core_dither() {
        let (width, height) = (16, 12);
        let pixels = gradient(width, height);
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255], [255, 0, 0]], 2);
        let img = ImageBuffer::new(&pixels, width);
        let expected = dither(
            &img,
            &palette,
            DitherConfig { mode: DitherMode::FloydSteinberg, serpentine: true, ..Default::default() },
        );
        let (code, out) = run(&pixels, width, &BWR, 2, &opts(DitherMode::FloydSteinberg));
        assert_eq!(code, ED_OK);
        assert_eq!(out, expected);
    }

    /// Same, with DBS on: the FFI must match `core::dither` with `dbs: Some(..)`.
    #[test]
    fn ffi_matches_core_dither_with_dbs() {
        let (width, height) = (24, 16);
        let pixels = gradient(width, height);
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255], [255, 0, 0]], 2);
        let params = DbsParams { viewing_distance_cm: 30.0, ppi: 100.0, max_passes: 4 };
        let img = ImageBuffer::new(&pixels, width);
        let expected = dither(
            &img,
            &palette,
            DitherConfig { mode: DitherMode::Burkes, dbs: Some(params), ..Default::default() },
        );
        let o = EdDitherOptions {
            dbs_max_passes: 4,
            dbs_viewing_distance_cm: 30.0,
            dbs_ppi: 100.0,
            ..opts(DitherMode::Burkes)
        };
        let (code, out) = run(&pixels, width, &BWR, 2, &o);
        assert_eq!(code, ED_OK);
        assert_eq!(out, expected);
        let (_, plain) = run(&pixels, width, &BWR, 2, &opts(DitherMode::Burkes));
        assert_ne!(out, plain, "DBS should change at least one pixel of a gradient");
    }

    /// DBS geometry is only checked when DBS is on, so a zeroed struct with DBS off still works.
    #[test]
    fn dbs_geometry_validated_only_when_enabled() {
        let pixels = gradient(8, 8);
        let off = EdDitherOptions { dbs_ppi: 0.0, dbs_viewing_distance_cm: f64::NAN, ..opts(DitherMode::Burkes) };
        assert_eq!(run(&pixels, 8, &BWR, 2, &off).0, ED_OK);
        for (distance, ppi) in [(0.0, 127.0), (40.0, -1.0), (f64::INFINITY, 127.0), (40.0, f64::NAN)] {
            let on = EdDitherOptions {
                dbs_max_passes: 1,
                dbs_viewing_distance_cm: distance,
                dbs_ppi: ppi,
                ..opts(DitherMode::Burkes)
            };
            assert_eq!(run(&pixels, 8, &BWR, 2, &on).0, ED_ERR_BAD_DBS, "{distance} cm, {ppi} ppi");
        }
    }

    #[test]
    fn rejects_bad_output_len() {
        let pixels = [0u8; 12]; // 4 px
        let palette: [u8; 6] = [0, 0, 0, 255, 255, 255];
        let mut out = [0u8; 3]; // wrong: should be 4
        let o = opts(DitherMode::Burkes);
        let code = unsafe {
            ed_dither(
                pixels.as_ptr(), pixels.len(), 2,
                palette.as_ptr(), palette.len(), 1,
                std::ptr::null(), 0, 0,
                &o, out.as_mut_ptr(), out.len(),
            )
        };
        assert_eq!(code, ED_ERR_BAD_OUTPUT_LEN);
    }

    #[test]
    fn rejects_null_pixels_and_options() {
        let mut out = [0u8; 4];
        let pixels = [0u8; 12];
        let palette: [u8; 6] = [0, 0, 0, 255, 255, 255];
        let o = opts(DitherMode::Burkes);
        let null_pixels = unsafe {
            ed_dither(
                std::ptr::null(), 12, 2,
                palette.as_ptr(), palette.len(), 1,
                std::ptr::null(), 0, 0,
                &o, out.as_mut_ptr(), out.len(),
            )
        };
        assert_eq!(null_pixels, ED_ERR_NULL_POINTER);
        let null_opts = unsafe {
            ed_dither(
                pixels.as_ptr(), pixels.len(), 2,
                palette.as_ptr(), palette.len(), 1,
                std::ptr::null(), 0, 0,
                std::ptr::null(), out.as_mut_ptr(), out.len(),
            )
        };
        assert_eq!(null_opts, ED_ERR_NULL_POINTER);
    }

    #[test]
    fn rejects_unknown_mode() {
        let pixels = [0u8; 12];
        let palette: [u8; 6] = [0, 0, 0, 255, 255, 255];
        let o = EdDitherOptions { mode_id: 99, ..ed_dither_options_default() };
        assert_eq!(run(&pixels, 2, &palette, 1, &o).0, ED_ERR_BAD_MODE);
    }

    #[test]
    fn abi_version_is_two() {
        assert_eq!(ed_abi_version(), 2);
    }
}

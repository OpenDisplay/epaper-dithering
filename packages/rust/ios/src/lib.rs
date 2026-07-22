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
//! - **Errors are status codes**, never exceptions. `0` is success; negative values are the
//!   `ED_ERR_*` constants below.

use std::panic::{self, AssertUnwindSafe};

use epaper_dithering_core::{
    DitherConfig, dither, dither_with_canonical,
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
    let colors: Vec<[u8; 3]> = bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    if colors.len() < 2 || accent_idx >= colors.len() {
        return Err(ED_ERR_BAD_PALETTE);
    }
    Ok(Some(Palette::new(colors, accent_idx)))
}

/// Dither a flat RGB image to palette indices.
///
/// # Parameters
/// - `pixels` / `pixels_len`: flat sRGB bytes, `len = width * height * 3`.
/// - `width`: image width in pixels (`> 0`).
/// - `matching_palette` / `matching_len` / `matching_accent`: the palette matched against,
///   in the caller's index order (`len` a multiple of 3, ≥ 2 colors). Required.
/// - `canonical_palette` / `canonical_len` / `canonical_accent`: optional ideal-color
///   palette for exact-pixel passthrough. Pass `canonical_len == 0` to disable (plain
///   matching against `matching_palette`).
/// - `mode_id`: [`DitherMode`] discriminant (0 = None … 8 = JarvisJudiceNinke).
/// - `serpentine`: serpentine scanning for error-diffusion modes.
/// - `out` / `out_len`: caller-allocated output, `out_len` must equal `width * height`.
///
/// # Returns
/// [`ED_OK`] on success (and `out` is filled), or a negative `ED_ERR_*` code on failure
/// (and `out` is left untouched).
///
/// # Safety
/// All pointers must either be null (where documented as optional) or valid for their
/// stated lengths for the duration of the call. `out` must be writable for `out_len` bytes.
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
    mode_id: u8,
    serpentine: bool,
    out: *mut u8,
    out_len: usize,
) -> i32 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if pixels.is_null() || out.is_null() {
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

        let mode = match DitherMode::try_from(mode_id) {
            Ok(m) => m,
            Err(_) => return ED_ERR_BAD_MODE,
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

        // Pre-processing intentionally left at defaults (off) — tone/gamut stay in the app.
        let config = DitherConfig { mode, serpentine, ..Default::default() };

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

/// ABI version of this wrapper. Bump when the exported signature changes so the Swift side
/// can assert compatibility at load time.
#[unsafe(no_mangle)]
pub extern "C" fn ed_abi_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Solid-red 2×2 against a black/white/red palette (app order) must map every pixel to
    /// index 2 (red), exercising the exact-color bypass through the FFI.
    #[test]
    fn solid_red_maps_to_red_index() {
        let pixels: Vec<u8> = std::iter::repeat_n([255u8, 0, 0], 4).flatten().collect();
        let palette: [u8; 9] = [0, 0, 0, 255, 255, 255, 255, 0, 0]; // black, white, red
        let mut out = [0u8; 4];
        let code = unsafe {
            ed_dither(
                pixels.as_ptr(),
                pixels.len(),
                2,
                palette.as_ptr(),
                palette.len(),
                2,
                std::ptr::null(),
                0,
                0,
                DitherMode::Burkes as u8,
                true,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert_eq!(code, ED_OK);
        assert_eq!(out, [2, 2, 2, 2]);
    }

    /// The FFI must produce byte-identical output to calling `core::dither` directly —
    /// this is the whole point of the wrapper (no logic of its own).
    #[test]
    fn ffi_matches_core_dither() {
        // A small gradient so error diffusion actually does something.
        let width = 4usize;
        let height = 4usize;
        let mut pixels = Vec::with_capacity(width * height * 3);
        for i in 0..(width * height) {
            let v = (i * 16) as u8;
            pixels.extend_from_slice(&[v, v, v]);
        }
        let palette_bytes: [u8; 6] = [0, 0, 0, 255, 255, 255]; // black, white
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]], 1);

        let img = ImageBuffer::new(&pixels, width);
        let expected = dither(
            &img,
            &palette,
            DitherConfig { mode: DitherMode::FloydSteinberg, serpentine: true, ..Default::default() },
        );

        let mut out = vec![0u8; width * height];
        let code = unsafe {
            ed_dither(
                pixels.as_ptr(),
                pixels.len(),
                width,
                palette_bytes.as_ptr(),
                palette_bytes.len(),
                1,
                std::ptr::null(),
                0,
                0,
                DitherMode::FloydSteinberg as u8,
                true,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert_eq!(code, ED_OK);
        assert_eq!(out, expected);
    }

    #[test]
    fn rejects_bad_output_len() {
        let pixels = [0u8; 12]; // 4 px
        let palette: [u8; 6] = [0, 0, 0, 255, 255, 255];
        let mut out = [0u8; 3]; // wrong: should be 4
        let code = unsafe {
            ed_dither(
                pixels.as_ptr(),
                pixels.len(),
                2,
                palette.as_ptr(),
                palette.len(),
                1,
                std::ptr::null(),
                0,
                0,
                DitherMode::Burkes as u8,
                true,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert_eq!(code, ED_ERR_BAD_OUTPUT_LEN);
    }

    #[test]
    fn rejects_null_pixels() {
        let mut out = [0u8; 4];
        let palette: [u8; 6] = [0, 0, 0, 255, 255, 255];
        let code = unsafe {
            ed_dither(
                std::ptr::null(),
                12,
                2,
                palette.as_ptr(),
                palette.len(),
                1,
                std::ptr::null(),
                0,
                0,
                DitherMode::Burkes as u8,
                true,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert_eq!(code, ED_ERR_NULL_POINTER);
    }

    #[test]
    fn rejects_unknown_mode() {
        let pixels = [0u8; 12];
        let palette: [u8; 6] = [0, 0, 0, 255, 255, 255];
        let mut out = [0u8; 4];
        let code = unsafe {
            ed_dither(
                pixels.as_ptr(),
                pixels.len(),
                2,
                palette.as_ptr(),
                palette.len(),
                1,
                std::ptr::null(),
                0,
                0,
                99,
                true,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert_eq!(code, ED_ERR_BAD_MODE);
    }

    #[test]
    fn abi_version_is_one() {
        assert_eq!(ed_abi_version(), 1);
    }
}

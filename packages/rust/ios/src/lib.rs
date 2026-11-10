//! C-ABI FFI wrapper around `epaper-dithering-core`, intended for the iOS/Swift app.
//!
//! Scope is deliberately minimal — this exposes **only** the dithering step (anchoring and
//! ink snapping, palette matching in OKLab, error diffusion, optional DBS). The
//! tone/gamut/exposure/saturation pre-processing pipeline is intentionally *not* surfaced:
//! the OpenDisplay iOS app keeps its own tone-compression pass in Swift for now.
//!
//! ## Contract
//!
//! - **Palettes are handles.** Create one from an `epaper-palette` JSON document with
//!   [`ed_palette_from_json`] and release it with [`ed_palette_free`] exactly once. A handle
//!   is immutable and may be shared across threads.
//! - **Output is caller-allocated.** `ed_dither` writes exactly `width * height` palette
//!   indices, in the palette's own entry order.
//! - **Panics never cross the boundary.** Every body runs inside `catch_unwind`; a panic is
//!   converted to [`ED_ERR_PANIC`].
//! - **Options mirror the core's `DitherOptions`.** Start from
//!   [`ed_dither_options_default`] and change only what you need.
//! - **Errors are status codes**, never exceptions. `0` is success; negative values are the
//!   `ED_ERR_*` constants below.

use std::ffi::{c_char, CStr};
use std::panic::{self, AssertUnwindSafe};

use epaper_dithering_core::{
    DitherOptions, dither,
    dbs::DbsParams,
    enums::DitherMode,
    palette::Palette,
    types::ImageBuffer,
};

/// Success.
pub const ED_OK: i32 = 0;
/// A required pointer argument was null.
pub const ED_ERR_NULL_POINTER: i32 = -1;
/// `width` was zero.
pub const ED_ERR_BAD_WIDTH: i32 = -2;
/// `pixels_len` is not a whole number of `width`-pixel RGB rows.
pub const ED_ERR_BAD_PIXELS: i32 = -3;
/// The palette JSON was not valid UTF-8, not an `epaper-palette` document, or failed
/// validation (entry count, duplicate inks, un-normalised white, …).
pub const ED_ERR_BAD_PALETTE: i32 = -4;
/// An output buffer has the wrong length.
pub const ED_ERR_BAD_OUTPUT_LEN: i32 = -5;
/// `mode` was null, not UTF-8, or not a known mode name.
pub const ED_ERR_BAD_MODE: i32 = -6;
/// A panic was caught inside the FFI body.
pub const ED_ERR_PANIC: i32 = -7;
/// `opts` asked for DBS (`dbs_max_passes > 0`) with `dbs_viewing_distance_cm` or `dbs_ppi`
/// not finite and greater than 0.
pub const ED_ERR_BAD_DBS: i32 = -8;

/// Opaque palette handle (`ed_palette` in C).
pub struct EdPalette {
    inner: Palette,
}

/// Dithering options, the C mirror of the parts of `DitherOptions` this wrapper exposes.
///
/// Get one from [`ed_dither_options_default`] and change fields from there. Adding a field is
/// an ABI change: bump [`ed_abi_version`] and the header together.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdDitherOptions {
    /// NUL-terminated mode name: "none", "burkes", "ordered", "floyd_steinberg", "atkinson",
    /// "stucki", "sierra", "sierra_lite", "jarvis_judice_ninke" or "dizzy".
    pub mode: *const c_char,
    /// Serpentine scanning for error-diffusion modes.
    pub serpentine: bool,
    /// Map each named ink's nominal colour onto its measured appearance before matching.
    pub anchor: bool,
    /// A pixel exactly equal to a named ink's nominal takes that ink, undithered.
    pub snap: bool,
    /// Direct Binary Search refinement: upper bound on full-image passes. `0` disables DBS.
    pub dbs_max_passes: u32,
    /// DBS eye model: viewer-to-panel distance in centimetres. Ignored when DBS is off.
    pub dbs_viewing_distance_cm: f64,
    /// DBS eye model: panel pixel density in pixels per inch. Ignored when DBS is off.
    pub dbs_ppi: f64,
}

/// Static NUL-terminated names, so `ed_dither_options_default` can hand out a pointer that
/// stays valid forever.
const MODE_NAMES: [(DitherMode, &CStr); 10] = [
    (DitherMode::None, c"none"),
    (DitherMode::Burkes, c"burkes"),
    (DitherMode::Ordered, c"ordered"),
    (DitherMode::FloydSteinberg, c"floyd_steinberg"),
    (DitherMode::Atkinson, c"atkinson"),
    (DitherMode::Stucki, c"stucki"),
    (DitherMode::Sierra, c"sierra"),
    (DitherMode::SierraLite, c"sierra_lite"),
    (DitherMode::JarvisJudiceNinke, c"jarvis_judice_ninke"),
    (DitherMode::Dizzy, c"dizzy"),
];

impl EdDitherOptions {
    /// Translate into the core's options. `Err` carries the `ED_ERR_*` code.
    ///
    /// # Safety
    /// `self.mode` must be null or point to a NUL-terminated string.
    unsafe fn to_options(self) -> Result<DitherOptions, i32> {
        if self.mode.is_null() {
            return Err(ED_ERR_BAD_MODE);
        }
        // SAFETY: non-null, and the caller guarantees NUL termination.
        let name = unsafe { CStr::from_ptr(self.mode) }.to_str().map_err(|_| ED_ERR_BAD_MODE)?;
        let mode = name.parse::<DitherMode>().map_err(|_| ED_ERR_BAD_MODE)?;
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
        Ok(DitherOptions { mode, serpentine: self.serpentine, anchor: self.anchor, snap: self.snap, dbs, ..Default::default() })
    }
}

/// The core's defaults: mode, serpentine, anchoring and snapping from
/// `DitherOptions::default()`, DBS off, and `DbsParams::default()`'s viewing geometry.
#[unsafe(no_mangle)]
pub extern "C" fn ed_dither_options_default() -> EdDitherOptions {
    let core = DitherOptions::default();
    let dbs = DbsParams::default();
    let mode = MODE_NAMES.iter().find(|(m, _)| *m == core.mode).map_or(c"burkes", |(_, n)| n);
    EdDitherOptions {
        mode: mode.as_ptr(),
        serpentine: core.serpentine,
        anchor: core.anchor,
        snap: core.snap,
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

/// Parse an `epaper-palette` JSON document (`json`, `len` bytes, UTF-8, no NUL needed) into a
/// new palette handle written to `*out`. On failure `*out` is set to null.
///
/// # Safety
/// `json` must be valid for `len` bytes; `out` must be a valid pointer to write to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ed_palette_from_json(json: *const c_char, len: usize, out: *mut *mut EdPalette) -> i32 {
    panic::catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return ED_ERR_NULL_POINTER;
        }
        // SAFETY: checked non-null.
        unsafe { *out = std::ptr::null_mut() };
        if json.is_null() {
            return ED_ERR_NULL_POINTER;
        }
        // SAFETY: caller guarantees `json` is valid for `len` bytes.
        let bytes = unsafe { std::slice::from_raw_parts(json.cast::<u8>(), len) };
        let Ok(text) = std::str::from_utf8(bytes) else { return ED_ERR_BAD_PALETTE };
        match Palette::from_json(text) {
            Ok(inner) => {
                // SAFETY: checked non-null.
                unsafe { *out = Box::into_raw(Box::new(EdPalette { inner })) };
                ED_OK
            }
            Err(_) => ED_ERR_BAD_PALETTE,
        }
    }))
    .unwrap_or(ED_ERR_PANIC)
}

/// Release a palette handle. Null is a no-op. Freeing the same handle twice is undefined
/// behaviour.
///
/// # Safety
/// `palette` must be null or a handle from [`ed_palette_from_json`] not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ed_palette_free(palette: *mut EdPalette) {
    if !palette.is_null() {
        // SAFETY: the caller guarantees this came from `Box::into_raw` and is freed once.
        drop(unsafe { Box::from_raw(palette) });
    }
}

/// Number of entries in the palette (0 for a null handle).
///
/// # Safety
/// `palette` must be null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ed_palette_len(palette: *const EdPalette) -> usize {
    // SAFETY: null or a live handle, per the contract.
    unsafe { palette.as_ref() }.map_or(0, |p| p.inner.len())
}

/// Write each entry's sRGB preview colour (3 bytes per entry, entry order) into `out`.
/// `out_len` must equal `3 * ed_palette_len(palette)`.
///
/// # Safety
/// `palette` must be a live handle; `out` must be writable for `out_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ed_palette_preview_rgb(palette: *const EdPalette, out: *mut u8, out_len: usize) -> i32 {
    // SAFETY: null or a live handle, per the contract.
    let Some(p) = (unsafe { palette.as_ref() }) else { return ED_ERR_NULL_POINTER };
    if out.is_null() {
        return ED_ERR_NULL_POINTER;
    }
    let rgb: Vec<u8> = p.inner.preview_srgb().iter().flatten().copied().collect();
    if out_len != rgb.len() {
        return ED_ERR_BAD_OUTPUT_LEN;
    }
    // SAFETY: `out` is non-null and writable for `out_len == rgb.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(rgb.as_ptr(), out, rgb.len()) };
    ED_OK
}

/// Dither a flat RGB image to palette indices, optionally refined with DBS.
///
/// # Parameters
/// - `pixels` / `pixels_len`: flat sRGB bytes, `len = width * height * 3`.
/// - `width`: image width in pixels (`> 0`).
/// - `palette`: a handle from [`ed_palette_from_json`]. Required.
/// - `opts`: algorithm, scan order, inks and DBS settings; see [`EdDitherOptions`]. Required.
/// - `out` / `out_len`: caller-allocated output, `out_len` must equal `width * height`.
///
/// # Returns
/// [`ED_OK`] on success (and `out` is filled), or a negative `ED_ERR_*` code on failure
/// (and `out` is left untouched).
///
/// # Safety
/// All pointers must be valid for their stated lengths for the duration of the call;
/// `opts->mode` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ed_dither(
    pixels: *const u8,
    pixels_len: usize,
    width: usize,
    palette: *const EdPalette,
    opts: *const EdDitherOptions,
    out: *mut u8,
    out_len: usize,
) -> i32 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if pixels.is_null() || out.is_null() || opts.is_null() || palette.is_null() {
            return ED_ERR_NULL_POINTER;
        }
        if width == 0 {
            return ED_ERR_BAD_WIDTH;
        }
        // SAFETY: validated non-null; the caller guarantees `pixels_len` bytes.
        let pixel_bytes = unsafe { std::slice::from_raw_parts(pixels, pixels_len) };
        let Ok(img) = ImageBuffer::new(pixel_bytes, width) else { return ED_ERR_BAD_PIXELS };
        if out_len != img.width * img.height {
            return ED_ERR_BAD_OUTPUT_LEN;
        }
        // SAFETY: validated non-null; the caller guarantees a valid struct and mode string.
        let options = match unsafe { (*opts).to_options() } {
            Ok(o) => o,
            Err(code) => return code,
        };
        // SAFETY: validated non-null; the caller guarantees a live handle.
        let palette = unsafe { &(*palette).inner };
        let indices = dither(&img, palette, &options);
        // SAFETY: `out` validated non-null and `out_len == indices.len()`.
        unsafe { std::ptr::copy_nonoverlapping(indices.as_ptr(), out, indices.len()) };
        ED_OK
    }));
    result.unwrap_or(ED_ERR_PANIC)
}

/// ABI version of this wrapper. Bump when the exported surface changes so the Swift side
/// can assert compatibility at load time.
///
/// - 1: `ed_dither(…, mode_id, serpentine, out, out_len)`.
/// - 2: options moved into [`EdDitherOptions`] (adds DBS); `ed_dither_options_default*` added.
/// - 3: palettes are handles from `epaper-palette` JSON (`ed_palette_*`); modes are names;
///   `anchor` and `snap` options; the canonical palette arguments are gone.
#[unsafe(no_mangle)]
pub extern "C" fn ed_abi_version() -> u32 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    const BWR_JSON: &str = r##"{"format":"epaper-palette","version":1,"entries":[
        {"appearance":[0.0,0.0,0.0],"name":"black","nominal":"#000000"},
        {"appearance":[100.0,0.0,0.0],"name":"white","nominal":"#ffffff"},
        {"appearance":[53.24,80.09,67.2],"name":"red","nominal":"#ff0000"}]}"##;

    fn palette(json: &str) -> *mut EdPalette {
        let mut handle = std::ptr::null_mut();
        let code = unsafe { ed_palette_from_json(json.as_ptr().cast(), json.len(), &mut handle) };
        assert_eq!(code, ED_OK);
        handle
    }

    fn opts(mode: &'static CStr) -> EdDitherOptions {
        EdDitherOptions { mode: mode.as_ptr(), ..ed_dither_options_default() }
    }

    /// Call `ed_dither`; returns (status, output).
    fn run(pixels: &[u8], width: usize, p: *const EdPalette, o: &EdDitherOptions) -> (i32, Vec<u8>) {
        let mut out = vec![0u8; pixels.len() / 3];
        let code = unsafe { ed_dither(pixels.as_ptr(), pixels.len(), width, p, o, out.as_mut_ptr(), out.len()) };
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

    #[test]
    fn defaults_mirror_the_core() {
        let o = ed_dither_options_default();
        let core = DitherOptions::default();
        let dbs = DbsParams::default();
        let mode = unsafe { CStr::from_ptr(o.mode) }.to_str().unwrap();
        assert_eq!(mode, core.mode.to_string());
        assert_eq!(o.serpentine, core.serpentine);
        assert_eq!((o.anchor, o.snap), (core.anchor, core.snap));
        assert_eq!(o.dbs_max_passes, 0, "DBS is opt-in");
        assert_eq!(o.dbs_viewing_distance_cm, dbs.viewing_distance_cm);
        assert_eq!(o.dbs_ppi, dbs.ppi);
        assert_eq!(ed_dither_options_default_dbs_passes(), dbs.max_passes);
    }

    #[test]
    fn solid_red_maps_to_red_index() {
        let p = palette(BWR_JSON);
        let pixels: Vec<u8> = std::iter::repeat_n([255u8, 0, 0], 4).flatten().collect();
        let (code, out) = run(&pixels, 2, p, &opts(c"burkes"));
        unsafe { ed_palette_free(p) };
        assert_eq!(code, ED_OK);
        assert_eq!(out, [2, 2, 2, 2]);
    }

    #[test]
    fn ffi_matches_core_dither() {
        let (width, height) = (16, 12);
        let pixels = gradient(width, height);
        let core_palette = Palette::from_json(BWR_JSON).unwrap();
        let img = ImageBuffer::new(&pixels, width).unwrap();
        let expected = dither(&img, &core_palette, &DitherOptions { mode: DitherMode::FloydSteinberg, ..Default::default() });
        let p = palette(BWR_JSON);
        let (code, out) = run(&pixels, width, p, &opts(c"floyd_steinberg"));
        unsafe { ed_palette_free(p) };
        assert_eq!(code, ED_OK);
        assert_eq!(out, expected);
    }

    #[test]
    fn ffi_matches_core_dither_with_dbs() {
        let (width, height) = (24, 16);
        let pixels = gradient(width, height);
        let params = DbsParams { viewing_distance_cm: 30.0, ppi: 100.0, max_passes: 4 };
        let img = ImageBuffer::new(&pixels, width).unwrap();
        let expected = dither(&img, &Palette::from_json(BWR_JSON).unwrap(), &DitherOptions { dbs: Some(params), ..Default::default() });
        let p = palette(BWR_JSON);
        let o = EdDitherOptions { dbs_max_passes: 4, dbs_viewing_distance_cm: 30.0, dbs_ppi: 100.0, ..opts(c"burkes") };
        let (code, out) = run(&pixels, width, p, &o);
        let (_, plain) = run(&pixels, width, p, &opts(c"burkes"));
        unsafe { ed_palette_free(p) };
        assert_eq!(code, ED_OK);
        assert_eq!(out, expected);
        assert_ne!(out, plain, "DBS should change at least one pixel of a gradient");
    }

    #[test]
    fn anchor_and_snap_switches_reach_the_core() {
        let pixels: Vec<u8> = std::iter::repeat_n([255u8, 0, 0], 4).flatten().collect();
        let img = ImageBuffer::new(&pixels, 2).unwrap();
        let core_palette = Palette::from_json(BWR_JSON).unwrap();
        let expected = dither(&img, &core_palette, &DitherOptions { anchor: false, snap: false, mode: DitherMode::None, ..Default::default() });
        let p = palette(BWR_JSON);
        let o = EdDitherOptions { anchor: false, snap: false, ..opts(c"none") };
        let (code, out) = run(&pixels, 2, p, &o);
        unsafe { ed_palette_free(p) };
        assert_eq!(code, ED_OK);
        assert_eq!(out, expected);
    }

    #[test]
    fn dbs_geometry_validated_only_when_enabled() {
        let pixels = gradient(8, 8);
        let p = palette(BWR_JSON);
        let off = EdDitherOptions { dbs_ppi: 0.0, dbs_viewing_distance_cm: f64::NAN, ..opts(c"burkes") };
        assert_eq!(run(&pixels, 8, p, &off).0, ED_OK);
        for (distance, ppi) in [(0.0, 127.0), (40.0, -1.0), (f64::INFINITY, 127.0), (40.0, f64::NAN)] {
            let on = EdDitherOptions { dbs_max_passes: 1, dbs_viewing_distance_cm: distance, dbs_ppi: ppi, ..opts(c"burkes") };
            assert_eq!(run(&pixels, 8, p, &on).0, ED_ERR_BAD_DBS, "{distance} cm, {ppi} ppi");
        }
        unsafe { ed_palette_free(p) };
    }

    #[test]
    fn rejects_bad_output_len() {
        let p = palette(BWR_JSON);
        let pixels = [0u8; 12];
        let mut out = [0u8; 3];
        let code = unsafe { ed_dither(pixels.as_ptr(), pixels.len(), 2, p, &opts(c"burkes"), out.as_mut_ptr(), out.len()) };
        unsafe { ed_palette_free(p) };
        assert_eq!(code, ED_ERR_BAD_OUTPUT_LEN);
    }

    #[test]
    fn rejects_bad_pixels() {
        let p = palette(BWR_JSON);
        let (code, _) = run(&[0u8; 7], 2, p, &opts(c"burkes"));
        unsafe { ed_palette_free(p) };
        assert_eq!(code, ED_ERR_BAD_PIXELS);
    }

    #[test]
    fn rejects_null_pixels_palette_and_options() {
        let p = palette(BWR_JSON);
        let mut out = [0u8; 4];
        let pixels = [0u8; 12];
        let o = opts(c"burkes");
        let codes = unsafe {
            [
                ed_dither(std::ptr::null(), 12, 2, p, &o, out.as_mut_ptr(), out.len()),
                ed_dither(pixels.as_ptr(), 12, 2, std::ptr::null(), &o, out.as_mut_ptr(), out.len()),
                ed_dither(pixels.as_ptr(), 12, 2, p, std::ptr::null(), out.as_mut_ptr(), out.len()),
            ]
        };
        unsafe { ed_palette_free(p) };
        assert_eq!(codes, [ED_ERR_NULL_POINTER; 3]);
    }

    #[test]
    fn rejects_unknown_or_null_mode() {
        let p = palette(BWR_JSON);
        let pixels = [0u8; 12];
        assert_eq!(run(&pixels, 2, p, &opts(c"bayer")).0, ED_ERR_BAD_MODE);
        let null_mode = EdDitherOptions { mode: std::ptr::null(), ..ed_dither_options_default() };
        assert_eq!(run(&pixels, 2, p, &null_mode).0, ED_ERR_BAD_MODE);
        unsafe { ed_palette_free(p) };
    }

    #[test]
    fn rejects_invalid_palette_json() {
        let mut handle = std::ptr::null_mut();
        let bad = "{\"format\":\"epaper-palette\",\"version\":1,\"entries\":[]}";
        let code = unsafe { ed_palette_from_json(bad.as_ptr().cast(), bad.len(), &mut handle) };
        assert_eq!(code, ED_ERR_BAD_PALETTE);
        assert!(handle.is_null(), "no handle on failure");
        let code = unsafe { ed_palette_from_json(std::ptr::null(), 3, &mut handle) };
        assert_eq!(code, ED_ERR_NULL_POINTER);
    }

    #[test]
    fn palette_reports_length_and_preview() {
        let p = palette(BWR_JSON);
        let mut rgb = [0u8; 9];
        let (len, code) = unsafe { (ed_palette_len(p), ed_palette_preview_rgb(p, rgb.as_mut_ptr(), rgb.len())) };
        let short = unsafe { ed_palette_preview_rgb(p, rgb.as_mut_ptr(), 8) };
        unsafe { ed_palette_free(p) };
        assert_eq!(len, 3);
        assert_eq!(code, ED_OK);
        assert_eq!(&rgb[..6], &[0, 0, 0, 255, 255, 255]);
        assert_eq!(short, ED_ERR_BAD_OUTPUT_LEN);
    }

    /// Double free is undefined behaviour and documented as such; freeing NULL is a no-op.
    #[test]
    fn freeing_null_is_a_no_op() {
        unsafe { ed_palette_free(std::ptr::null_mut()) };
    }

    #[test]
    fn abi_version_is_three() {
        assert_eq!(ed_abi_version(), 3);
    }
}

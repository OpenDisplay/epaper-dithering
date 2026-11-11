/*
 * epaper_dithering.h — C ABI for the epaper-dithering iOS wrapper.
 *
 * Hand-written (the surface is small). Keep in sync with `src/lib.rs`.
 * Bump ED_ABI_VERSION / ed_abi_version() together when the signature changes.
 */
#ifndef EPAPER_DITHERING_H
#define EPAPER_DITHERING_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes. */
#define ED_OK                 0
#define ED_ERR_NULL_POINTER  -1
#define ED_ERR_BAD_WIDTH     -2
#define ED_ERR_BAD_PIXELS    -3   /* pixels_len is not whole width-pixel RGB rows */
#define ED_ERR_BAD_PALETTE   -4   /* palette JSON not UTF-8, not epaper-palette, or invalid */
#define ED_ERR_BAD_OUTPUT_LEN -5
#define ED_ERR_BAD_MODE      -6   /* mode NULL, not UTF-8, or not a known name */
#define ED_ERR_PANIC         -7
#define ED_ERR_BAD_DBS       -8   /* DBS on with viewing distance or ppi not finite and > 0 */

/* Opaque, immutable palette handle. Thread-safe to share. */
typedef struct ed_palette ed_palette;

/*
 * Parse an epaper-palette JSON document (json, len bytes of UTF-8; no NUL needed) into a new
 * handle written to *out. On failure *out is NULL and a negative ED_ERR_* code is returned.
 */
int32_t ed_palette_from_json(const char *json, size_t len, ed_palette **out);

/* Release a handle. NULL is a no-op. Freeing the same handle twice is undefined behaviour. */
void ed_palette_free(ed_palette *palette);

/* Number of entries (0 for NULL). */
size_t ed_palette_len(const ed_palette *palette);

/* Each entry's sRGB preview colour, 3 bytes per entry; out_len MUST equal 3 * ed_palette_len. */
int32_t ed_palette_preview_rgb(const ed_palette *palette, uint8_t *out, size_t out_len);

/*
 * Dithering options (mirror of the exposed part of the core's DitherOptions).
 * Start from ed_dither_options_default() and change only what you need.
 *
 *   mode                     NUL-terminated name: "none", "burkes", "ordered",
 *                            "floyd_steinberg", "atkinson", "stucki", "sierra",
 *                            "sierra_lite", "jarvis_judice_ninke", "dizzy"
 *   serpentine               serpentine scan for error-diffusion modes
 *   anchor                   map each named ink's nominal onto its measured appearance first
 *   snap                     a pixel exactly equal to a named ink's nominal takes that ink
 *   dbs_max_passes           Direct Binary Search refinement: max full-image passes. 0 = off.
 *   dbs_viewing_distance_cm  DBS eye model: viewer distance in cm (ignored when off)
 *   dbs_ppi                  DBS eye model: panel pixels per inch (ignored when off)
 *
 * Adding a field is an ABI change: bump ed_abi_version() with it.
 */
typedef struct {
    const char *mode;
    bool     serpentine;
    bool     anchor;
    bool     snap;
    uint32_t dbs_max_passes;
    double   dbs_viewing_distance_cm;
    double   dbs_ppi;
} ed_dither_options;

/* The core's defaults (mode "burkes", serpentine, anchor and snap on), DBS off. */
ed_dither_options ed_dither_options_default(void);

/* The core's default DBS pass budget, for turning DBS on without hard-coding it. */
uint32_t ed_dither_options_default_dbs_passes(void);

/*
 * Dither a flat sRGB image to palette indices: anchoring and ink snapping, matching + error
 * diffusion, then optional DBS refinement (opts->dbs_max_passes > 0).
 *
 *   pixels/pixels_len   flat RGB bytes, len = width*height*3
 *   width               image width in pixels (> 0)
 *   palette             handle from ed_palette_from_json (required)
 *   opts                options (required, non-NULL)
 *   out/out_len         caller-allocated output; out_len MUST equal width*height.
 *                       One u8 palette index per pixel, in palette entry order.
 *
 * Returns ED_OK (0) on success (out filled), or a negative ED_ERR_* code (out untouched).
 * Never panics across the boundary.
 */
int32_t ed_dither(const uint8_t *pixels, size_t pixels_len, size_t width,
                  const ed_palette *palette,
                  const ed_dither_options *opts,
                  uint8_t *out, size_t out_len);

/* ABI version of this wrapper (currently 3: palette handles, named modes, anchor/snap). */
uint32_t ed_abi_version(void);

#ifdef __cplusplus
}
#endif

#endif /* EPAPER_DITHERING_H */

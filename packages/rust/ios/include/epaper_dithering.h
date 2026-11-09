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

/* Status codes returned by ed_dither. */
#define ED_OK                 0
#define ED_ERR_NULL_POINTER  -1
#define ED_ERR_BAD_WIDTH     -2
#define ED_ERR_BAD_PIXELS    -3
#define ED_ERR_BAD_PALETTE   -4
#define ED_ERR_BAD_OUTPUT_LEN -5
#define ED_ERR_BAD_MODE      -6
#define ED_ERR_PANIC         -7
#define ED_ERR_BAD_DBS       -8   /* DBS on with viewing distance or ppi not finite and > 0 */

/*
 * Dithering options (mirror of the exposed part of the core's DitherConfig).
 * Start from ed_dither_options_default() and change only what you need.
 *
 *   mode_id                  DitherMode discriminant: 0=None,1=Burkes,2=Ordered,
 *                            3=FloydSteinberg,4=Atkinson,5=Stucki,6=Sierra,7=SierraLite,
 *                            8=JarvisJudiceNinke,9=Dizzy
 *   serpentine               serpentine scan for error-diffusion modes
 *   dbs_max_passes           Direct Binary Search refinement: max full-image passes
 *                            (stops early once a pass changes nothing). 0 = DBS off.
 *                            Opt-in and much slower than diffusion alone.
 *   dbs_viewing_distance_cm  DBS eye model: viewer distance in cm (ignored when off)
 *   dbs_ppi                  DBS eye model: panel pixels per inch (ignored when off)
 *
 * Adding a field is an ABI change: bump ed_abi_version() with it.
 */
typedef struct {
    uint8_t  mode_id;
    bool     serpentine;
    uint32_t dbs_max_passes;
    double   dbs_viewing_distance_cm;
    double   dbs_ppi;
} ed_dither_options;

/* The core's defaults: default mode/serpentine, DBS off, DbsParams::default() geometry. */
ed_dither_options ed_dither_options_default(void);

/* The core's default DBS pass budget, for turning DBS on without hard-coding it. */
uint32_t ed_dither_options_default_dbs_passes(void);

/*
 * Dither a flat sRGB image to palette indices: matching + error diffusion, then optional
 * DBS refinement (opts->dbs_max_passes > 0).
 *
 *   pixels/pixels_len   flat RGB bytes, len = width*height*3
 *   width               image width in pixels (> 0)
 *   matching_palette/…  palette matched against, caller index order; len multiple of 3,
 *                       >= 2 colors; matching_accent < color count. Required.
 *   canonical_palette/… optional ideal-color palette for exact-pixel passthrough;
 *                       pass canonical_len == 0 (and NULL) to disable.
 *   opts                algorithm, scan order and DBS settings (required, non-NULL)
 *   out/out_len         caller-allocated output; out_len MUST equal width*height.
 *                       One u8 palette index per pixel, in matching-palette order.
 *
 * Returns ED_OK (0) on success (out filled), or a negative ED_ERR_* code (out untouched).
 * Never panics across the boundary; never allocates memory the caller must free.
 */
int32_t ed_dither(const uint8_t *pixels, size_t pixels_len, size_t width,
                  const uint8_t *matching_palette, size_t matching_len, size_t matching_accent,
                  const uint8_t *canonical_palette, size_t canonical_len, size_t canonical_accent,
                  const ed_dither_options *opts,
                  uint8_t *out, size_t out_len);

/* ABI version of this wrapper (currently 2: options struct + DBS). */
uint32_t ed_abi_version(void);

#ifdef __cplusplus
}
#endif

#endif /* EPAPER_DITHERING_H */

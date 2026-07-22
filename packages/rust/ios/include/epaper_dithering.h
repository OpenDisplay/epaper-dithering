/*
 * epaper_dithering.h — C ABI for the epaper-dithering iOS wrapper.
 *
 * Hand-written (the surface is two functions). Keep in sync with `src/lib.rs`.
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

/*
 * Dither a flat sRGB image to palette indices (matching + error diffusion only).
 *
 *   pixels/pixels_len   flat RGB bytes, len = width*height*3
 *   width               image width in pixels (> 0)
 *   matching_palette/…  palette matched against, caller index order; len multiple of 3,
 *                       >= 2 colors; matching_accent < color count. Required.
 *   canonical_palette/… optional ideal-color palette for exact-pixel passthrough;
 *                       pass canonical_len == 0 (and NULL) to disable.
 *   mode_id             DitherMode discriminant: 0=None,1=Burkes,2=Ordered,
 *                       3=FloydSteinberg,4=Atkinson,5=Stucki,6=Sierra,7=SierraLite,
 *                       8=JarvisJudiceNinke
 *   serpentine          serpentine scan for error-diffusion modes
 *   out/out_len         caller-allocated output; out_len MUST equal width*height.
 *                       One u8 palette index per pixel, in matching-palette order.
 *
 * Returns ED_OK (0) on success (out filled), or a negative ED_ERR_* code (out untouched).
 * Never panics across the boundary; never allocates memory the caller must free.
 */
int32_t ed_dither(const uint8_t *pixels, size_t pixels_len, size_t width,
                  const uint8_t *matching_palette, size_t matching_len, size_t matching_accent,
                  const uint8_t *canonical_palette, size_t canonical_len, size_t canonical_accent,
                  uint8_t mode_id, bool serpentine,
                  uint8_t *out, size_t out_len);

/* ABI version of this wrapper (currently 1). */
uint32_t ed_abi_version(void);

#ifdef __cplusplus
}
#endif

#endif /* EPAPER_DITHERING_H */

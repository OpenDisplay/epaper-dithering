/** Dithering algorithms. Values are the names the core accepts. */
export enum DitherMode {
  /**
   * Direct nearest-color mapping with no error diffusion. Intended for
   * already-quantized graphics only; use an error-diffusion mode for photos.
   */
  NONE = 'none',
  BURKES = 'burkes',
  ORDERED = 'ordered',
  FLOYD_STEINBERG = 'floyd_steinberg',
  ATKINSON = 'atkinson',
  STUCKI = 'stucki',
  SIERRA = 'sierra',
  SIERRA_LITE = 'sierra_lite',
  JARVIS_JUDICE_NINKE = 'jarvis_judice_ninke',
  /**
   * Error diffusion with pseudo-random traversal instead of a raster scan,
   * diffusing error only to not-yet-quantized neighbours. Produces no
   * directional structure. `serpentine` is ignored: there is no scan direction.
   */
  DIZZY = 'dizzy',
}

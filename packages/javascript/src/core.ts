import type { ImageBuffer, PaletteImageBuffer, ColorPalette } from './types';
import { DitherMode } from './enums';
import { ColorScheme } from './palettes';
import * as algorithms from './algorithms';

/**
 * Apply dithering algorithm to image for e-paper display.
 *
 * @param image - Input image buffer (RGBA format). Alpha is composited on white.
 * @param colorScheme - Target color scheme (ColorScheme enum) or measured palette (ColorPalette)
 * @param mode - Dithering algorithm (default: BURKES)
 * @param serpentine - Alternate row direction to reduce artifacts (default: true)
 * @returns Palette-indexed image buffer
 *
 * @example
 * ```typescript
 * // Standard color scheme
 * const dithered = ditherImage(imageBuffer, ColorScheme.BWR, DitherMode.FLOYD_STEINBERG);
 *
 * // Measured palette for accurate color matching
 * const dithered = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR);
 * ```
 */
export function ditherImage(
  image: ImageBuffer,
  colorScheme: ColorScheme | ColorPalette,
  mode: DitherMode = DitherMode.BURKES,
  serpentine: boolean = true,
): PaletteImageBuffer {
  switch (mode) {
    case DitherMode.NONE:
      return algorithms.directPaletteMap(image, colorScheme);
    case DitherMode.ORDERED:
      return algorithms.orderedDither(image, colorScheme);
    case DitherMode.FLOYD_STEINBERG:
      return algorithms.floydSteinbergDither(image, colorScheme, serpentine);
    case DitherMode.ATKINSON:
      return algorithms.atkinsonDither(image, colorScheme, serpentine);
    case DitherMode.STUCKI:
      return algorithms.stuckiDither(image, colorScheme, serpentine);
    case DitherMode.SIERRA:
      return algorithms.sierraDither(image, colorScheme, serpentine);
    case DitherMode.SIERRA_LITE:
      return algorithms.sierraLiteDither(image, colorScheme, serpentine);
    case DitherMode.JARVIS_JUDICE_NINKE:
      return algorithms.jarvisJudiceNinkeDither(image, colorScheme, serpentine);
    case DitherMode.BURKES:
    default:
      return algorithms.burkesDither(image, colorScheme, serpentine);
  }
}

export { ditherImage } from './core';
export { DitherMode } from './enums';
export {
  ColorScheme,
  getPalette,
  getColorCount,
  fromValue,
  // Measured palettes
  SPECTRA_7_3_6COLOR,
  MONO_4_26,
  BWRY_4_2,
  SOLUM_BWR,
  HANSHOW_BWR,
  HANSHOW_BWY,
  BWRY_3_97,
} from './palettes';
export type { RGB, ImageBuffer, PaletteImageBuffer, ColorPalette } from './types';

export const VERSION = '0.1.0';

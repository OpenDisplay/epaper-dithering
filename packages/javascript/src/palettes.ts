import type { ColorPalette } from './types';

/**
 * E-paper display color schemes
 * Values match firmware conventions (0-7)
 */
export enum ColorScheme {
  MONO        = 0,
  BWR         = 1,
  BWY         = 2,
  BWRY        = 3,
  BWGBRY      = 4,
  GRAYSCALE_4 = 5,
  GRAYSCALE_8 = 6,
  GRAYSCALE_16 = 7,
}

const PALETTES: Record<ColorScheme, ColorPalette> = {
  [ColorScheme.MONO]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
  [ColorScheme.BWR]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      red: { r: 255, g: 0, b: 0 },
    },
    accent: 'red',
  },
  [ColorScheme.BWY]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0 },
    },
    accent: 'yellow',
  },
  [ColorScheme.BWRY]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0 },
      red: { r: 255, g: 0, b: 0 },
    },
    accent: 'red',
  },
  [ColorScheme.BWGBRY]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      white: { r: 255, g: 255, b: 255 },
      yellow: { r: 255, g: 255, b: 0 },
      red: { r: 255, g: 0, b: 0 },
      blue: { r: 0, g: 0, b: 255 },
      green: { r: 0, g: 255, b: 0 },
    },
    accent: 'red',
  },
  [ColorScheme.GRAYSCALE_4]: {
    colors: {
      black: { r: 0, g: 0, b: 0 },
      gray1: { r: 85, g: 85, b: 85 },
      gray2: { r: 170, g: 170, b: 170 },
      white: { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
  [ColorScheme.GRAYSCALE_8]: {
    colors: {
      black: { r: 0,   g: 0,   b: 0   },
      gray1: { r: 36,  g: 36,  b: 36  },
      gray2: { r: 73,  g: 73,  b: 73  },
      gray3: { r: 109, g: 109, b: 109 },
      gray4: { r: 146, g: 146, b: 146 },
      gray5: { r: 182, g: 182, b: 182 },
      gray6: { r: 219, g: 219, b: 219 },
      white: { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
  [ColorScheme.GRAYSCALE_16]: {
    colors: {
      black:  { r: 0,   g: 0,   b: 0   },
      gray1:  { r: 17,  g: 17,  b: 17  },
      gray2:  { r: 34,  g: 34,  b: 34  },
      gray3:  { r: 51,  g: 51,  b: 51  },
      gray4:  { r: 68,  g: 68,  b: 68  },
      gray5:  { r: 85,  g: 85,  b: 85  },
      gray6:  { r: 102, g: 102, b: 102 },
      gray7:  { r: 119, g: 119, b: 119 },
      gray8:  { r: 136, g: 136, b: 136 },
      gray9:  { r: 153, g: 153, b: 153 },
      gray10: { r: 170, g: 170, b: 170 },
      gray11: { r: 187, g: 187, b: 187 },
      gray12: { r: 204, g: 204, b: 204 },
      gray13: { r: 221, g: 221, b: 221 },
      gray14: { r: 238, g: 238, b: 238 },
      white:  { r: 255, g: 255, b: 255 },
    },
    accent: 'black',
  },
};

/** Get color palette for a color scheme */
export function getPalette(scheme: ColorScheme): ColorPalette {
  return PALETTES[scheme];
}

/** Get number of colors in a color scheme */
export function getColorCount(scheme: ColorScheme): number {
  return Object.keys(PALETTES[scheme].colors).length;
}

/** Create ColorScheme from firmware integer value */
export function fromValue(value: number): ColorScheme {
  if (value < 0 || value > 7) {
    throw new Error(`Invalid color scheme value: ${value}`);
  }
  return value as ColorScheme;
}

// =============================================================================
// Measured Palettes for Specific E-Paper Displays
// =============================================================================
//
// These constants provide measured RGB values from real e-paper displays.
// Pass them directly to ditherImage() instead of a ColorScheme enum.
//
// Usage:
//   import { ditherImage, SPECTRA_7_3_6COLOR } from '@opendisplay/epaper-dithering';
//   const result = ditherImage(imageBuffer, SPECTRA_7_3_6COLOR);
//
// Color order MUST match the corresponding ColorScheme palette order.
// =============================================================================

// 7.3" Spectra™ 6-color (BWGBRY scheme)
// Measured: 2026-02-03, iPhone 15 Pro Max RAW + Hue Play bars @ 6500K
// Paper reference RGB(215,217,218); normalization: value × (255/paper_channel)
export const SPECTRA_7_3_6COLOR: ColorPalette = {
  colors: {
    black:  { r: 26,  g: 13,  b: 35  },
    white:  { r: 185, g: 202, b: 205 },
    yellow: { r: 202, g: 184, b: 0   },
    red:    { r: 121, g: 9,   b: 0   },
    blue:   { r: 0,   g: 69,  b: 139 },
    green:  { r: 40,  g: 82,  b: 57  },
  },
  accent: 'red',
};

// 4.26" Monochrome (MONO scheme)
export const MONO_4_26: ColorPalette = {
  colors: {
    black: { r: 5,   g: 5,   b: 5   },
    white: { r: 220, g: 220, b: 220 },
  },
  accent: 'black',
};

// 4.2" BWRY (BWRY scheme)
export const BWRY_4_2: ColorPalette = {
  colors: {
    black:  { r: 5,   g: 5,   b: 5   },
    white:  { r: 200, g: 200, b: 200 },
    yellow: { r: 200, g: 180, b: 0   },
    red:    { r: 120, g: 15,  b: 5   },
  },
  accent: 'red',
};

// Solum BWR (harvested display, BWR scheme)
export const SOLUM_BWR: ColorPalette = {
  colors: {
    black: { r: 5,   g: 5,   b: 5   },
    white: { r: 200, g: 200, b: 200 },
    red:   { r: 120, g: 15,  b: 5   },
  },
  accent: 'red',
};

// Hanshow BWR (harvested display, BWR scheme)
export const HANSHOW_BWR: ColorPalette = {
  colors: {
    black: { r: 5,   g: 5,   b: 5   },
    white: { r: 200, g: 200, b: 200 },
    red:   { r: 120, g: 15,  b: 5   },
  },
  accent: 'red',
};

// Hanshow BWY (harvested display, BWY scheme)
export const HANSHOW_BWY: ColorPalette = {
  colors: {
    black:  { r: 5,   g: 5,   b: 5   },
    white:  { r: 200, g: 200, b: 200 },
    yellow: { r: 200, g: 180, b: 0   },
  },
  accent: 'yellow',
};

// 3.97" BWRY — EP397YR_800x480 (BWRY scheme)
// Measured: 2026-03-06, iPhone RAW
// Paper reference RGB(205,205,205); normalization: value × (255/205)
export const BWRY_3_97: ColorPalette = {
  colors: {
    black:  { r: 10,  g: 7,   b: 14  },
    white:  { r: 173, g: 178, b: 174 },
    yellow: { r: 172, g: 128, b: 0   },
    red:    { r: 85,  g: 24,  b: 14  },
  },
  accent: 'red',
};

/**
 * CIELAB color space conversion and LCH-weighted color matching.
 *
 * Why LCH weighting for dithering:
 * Error diffusion compensates for lightness errors spatially (mixing pixels),
 * but CANNOT compensate for hue errors. So hue must be matched accurately on
 * the first try. Weights: WL=0.5 (de-emphasized), WC=1.0, WH=2.0 (emphasized).
 *
 * Uses the CIE identity: da² + db² = dC² + dH²
 * This decomposes a·b plane distance into chroma+hue without atan2/angle wrapping.
 */

// sRGB → XYZ matrix (D65 illuminant) — Brucelindbloom
// Stored as scalar constants to avoid array indexing overhead in hot path
const M00 = 0.4124564, M01 = 0.3575761, M02 = 0.1804375;
const M10 = 0.2126729, M11 = 0.7151522, M12 = 0.0721750;
const M20 = 0.0193339, M21 = 0.1191920, M22 = 0.9503041;

// D65 white point
const XN = 0.95047, YN = 1.00000, ZN = 1.08883;

// CIE LAB transfer function constants
const EPSILON = 216.0 / 24389.0; // 0.008856...
const KAPPA = 24389.0 / 27.0;    // 903.296...

// LCH distance weights — pre-squared to eliminate squaring in inner loop
// WL=0.5 → WL²=0.25, WC=1.0 → WC²=1.0, WH=2.0 → WH²=4.0
const WL2 = 0.25;
const WC2 = 1.0;
const WH2 = 4.0;

function labF(t: number): number {
  return t > EPSILON ? Math.cbrt(t) : (KAPPA * t + 16.0) / 116.0;
}

/** Convert linear RGB [0,1] to CIELAB. Inlined XYZ multiply for speed. */
function rgbToLabScalar(r: number, g: number, b: number): [number, number, number] {
  const x = M00 * r + M01 * g + M02 * b;
  const y = M10 * r + M11 * g + M12 * b;
  const z = M20 * r + M21 * g + M22 * b;

  const fx = labF(x / XN);
  const fy = labF(y / YN);
  const fz = labF(z / ZN);

  return [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)];
}

/** Pre-computed palette LAB components for the error diffusion inner loop. */
export interface PaletteLabArrays {
  L: Float64Array;
  a: Float64Array;
  b: Float64Array;
  C: Float64Array; // chroma = sqrt(a² + b²)
}

/**
 * Pre-compute palette LAB components before the dithering loop.
 * Call once per ditherImage call, then pass results to matchPixelLch.
 */
export function precomputePaletteLab(
  paletteLinear: Array<[number, number, number]>,
): PaletteLabArrays {
  const n = paletteLinear.length;
  const L = new Float64Array(n);
  const a = new Float64Array(n);
  const b = new Float64Array(n);
  const C = new Float64Array(n);

  for (let i = 0; i < n; i++) {
    const [pl, pa, pb] = rgbToLabScalar(paletteLinear[i][0], paletteLinear[i][1], paletteLinear[i][2]);
    L[i] = pl;
    a[i] = pa;
    b[i] = pb;
    C[i] = Math.sqrt(pa * pa + pb * pb);
  }

  return { L, a, b, C };
}

/**
 * Find the closest palette color for a single pixel using LCH-weighted LAB distance.
 *
 * Pure scalar arithmetic — no object allocation, no array creation.
 * This is the innermost hot loop; called once per pixel.
 */
export function matchPixelLch(
  r: number,
  g: number,
  b: number,
  paletteL: Float64Array,
  paletteA: Float64Array,
  paletteB: Float64Array,
  paletteC: Float64Array,
): number {
  const [pL, pa, pb] = rgbToLabScalar(r, g, b);
  const pC = Math.sqrt(pa * pa + pb * pb);

  let bestIdx = 0;
  let bestDist = Infinity;
  const n = paletteL.length;

  for (let i = 0; i < n; i++) {
    const dL = pL - paletteL[i];
    const da = pa - paletteA[i];
    const db = pb - paletteB[i];
    const dC = pC - paletteC[i];
    let dHsq = da * da + db * db - dC * dC;
    if (dHsq < 0.0) dHsq = 0.0;

    const dist = WL2 * dL * dL + WC2 * dC * dC + WH2 * dHsq;
    if (dist < bestDist) {
      bestDist = dist;
      bestIdx = i;
    }
  }

  return bestIdx;
}

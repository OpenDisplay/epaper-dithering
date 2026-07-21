use epaper_dithering_core::{
    color_space_lab::rgb_to_oklab,
    composite::composite_rgba_on_white,
    dither, dither_with_canonical, DitherConfig,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::CATALOG,
    palettes::{ColorScheme, Palette},
    tone_map,
    types::ImageBuffer,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::types::PyBytes;

fn parse_mode(v: u8) -> PyResult<DitherMode> {
    DitherMode::try_from(v).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Validate image dimensions before constructing an `ImageBuffer` (which panics on mismatch).
fn validate_image(pixels: &[u8], width: usize) -> PyResult<()> {
    if width == 0 {
        return Err(PyValueError::new_err("width must be greater than 0"));
    }
    if !pixels.len().is_multiple_of(3) {
        return Err(PyValueError::new_err("pixel buffer length must be a multiple of 3"));
    }
    if !(pixels.len() / 3).is_multiple_of(width) {
        return Err(PyValueError::new_err(
            "pixel count is not a whole number of rows for the given width",
        ));
    }
    Ok(())
}

fn parse_tone(v: Option<f64>) -> ToneCompression {
    match v {
        None    => ToneCompression::Auto,
        Some(s) => ToneCompression::Fixed(s),
    }
}

fn parse_gamut(v: Option<f64>) -> GamutCompression {
    match v {
        None              => GamutCompression::Auto,
        Some(s) if s > 0.0 => GamutCompression::Fixed(s),
        _                 => GamutCompression::None,
    }
}

/// Dither a flat RGB image.
///
/// Pass either `scheme_id` (idealized color scheme) or `palette_bytes` + `accent_idx`
/// (measured palette). If both are given, `palette_bytes` wins.
#[pyfunction]
#[pyo3(signature = (
    pixels, width, height, *,
    scheme_id=None, palette_bytes=None, accent_idx=0,
    mode_id=1, serpentine=true,
    exposure=1.0, saturation=1.0, shadows=0.0, highlights=0.0,
    tone=0.0, gamut=0.0,
))]
#[allow(clippy::too_many_arguments)]
fn dither_image(
    py: Python<'_>,
    pixels: PyBackedBytes,
    width: usize,
    height: usize,
    scheme_id: Option<u8>,
    palette_bytes: Option<PyBackedBytes>,
    accent_idx: usize,
    mode_id: u8,
    serpentine: bool,
    exposure: f64,
    saturation: f64,
    shadows: f64,
    highlights: f64,
    tone: Option<f64>,
    gamut: Option<f64>,
) -> PyResult<Vec<u8>> {
    validate_image(&pixels, width)?;
    // Additionally validate against the caller's height instead of silently
    // deriving it and truncating trailing pixels on a width/length mismatch.
    // Layout is flat RGB: len = width × height × 3.
    let expected = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(3))
        .ok_or_else(|| PyValueError::new_err("width × height × 3 overflows usize"))?;
    if pixels.len() != expected {
        return Err(PyValueError::new_err(format!(
            "pixel buffer length ({}) does not match width × height × 3 ({} × {} × 3 = {})",
            pixels.len(),
            width,
            height,
            expected,
        )));
    }
    let img = ImageBuffer::new(&pixels, width);
    let config = DitherConfig {
        mode: parse_mode(mode_id)?,
        serpentine,
        exposure,
        saturation,
        shadows,
        highlights,
        tone:  parse_tone(tone),
        gamut: parse_gamut(gamut),
    };

    // Validation and `Palette` construction stay under the GIL; only the
    // rayon-parallel dither call itself releases it, so a caller running us
    // from an executor thread (e.g. Home Assistant's `drawcustom` pipeline)
    // doesn't stall its event loop for the whole dither.
    match (palette_bytes, scheme_id) {
        (Some(bytes), scheme_id) => {
            if !bytes.len().is_multiple_of(3) {
                return Err(PyValueError::new_err("palette_bytes length must be a multiple of 3"));
            }
            let colors: Vec<[u8; 3]> = bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
            if colors.len() < 2 {
                return Err(PyValueError::new_err("palette must have at least 2 colors"));
            }
            if accent_idx >= colors.len() {
                return Err(PyValueError::new_err(format!(
                    "accent_idx {accent_idx} out of range for {}-color palette",
                    colors.len()
                )));
            }
            let palette = Palette::new(colors, accent_idx);
            if let Some(id) = scheme_id {
                let scheme = ColorScheme::try_from(id)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                Ok(py.detach(|| dither_with_canonical(&img, &palette, scheme.palette(), config)))
            } else {
                Ok(py.detach(|| dither(&img, palette, config)))
            }
        }
        (None, Some(id)) => {
            let scheme = ColorScheme::try_from(id)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(py.detach(|| dither(&img, scheme.palette(), config)))
        }
        (None, None) => Err(PyValueError::new_err("must provide either scheme_id or palette_bytes")),
    }
}

/// Composite a flat RGBA buffer onto white, returning flat RGB bytes.
///
/// Shares its implementation with the WASM/JavaScript binding so both languages produce
/// byte-identical output for semi-transparent pixels.
///
/// Raises `ValueError` if the buffer length is not a multiple of 4.
#[pyfunction]
fn composite_rgba<'py>(py: Python<'py>, rgba: PyBackedBytes) -> PyResult<Bound<'py, PyBytes>> {
    let rgb = py
        .detach(|| composite_rgba_on_white(&rgba))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyBytes::new(py, &rgb))
}

// ── Preprocessing steps (linear-RGB buffers) for tooling/visualization ──────────
//
// These expose the pre-dither pipeline stages so scripts can inspect intermediate
// buffers. Input/output are flat linear-RGB f64 (length a multiple of 3).

fn to_linear_pixels(pixels: &[f64]) -> PyResult<Vec<[f64; 3]>> {
    if !pixels.len().is_multiple_of(3) {
        return Err(PyValueError::new_err("pixel buffer length must be a multiple of 3"));
    }
    Ok(pixels.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

fn palette_from_bytes(palette_bytes: &[u8]) -> PyResult<Palette> {
    if !palette_bytes.len().is_multiple_of(3) {
        return Err(PyValueError::new_err("palette_bytes length must be a multiple of 3"));
    }
    let colors: Vec<[u8; 3]> = palette_bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    if colors.len() < 2 {
        return Err(PyValueError::new_err("palette must have at least 2 colors"));
    }
    // accent_idx is irrelevant for tone/gamut ops; use 0.
    Ok(Palette::new(colors, 0))
}

fn flatten(pixels: Vec<[f64; 3]>) -> Vec<f64> {
    pixels.into_iter().flatten().collect()
}

/// Dynamic-range compression on a linear-RGB buffer. `strength=None` → auto (histogram-based).
#[pyfunction]
#[pyo3(signature = (pixels, palette_bytes, strength=None))]
fn tone_compress(py: Python<'_>, pixels: Vec<f64>, palette_bytes: &[u8], strength: Option<f64>) -> PyResult<Vec<f64>> {
    let mut px = to_linear_pixels(&pixels)?;
    let palette = palette_from_bytes(palette_bytes)?;
    py.detach(|| match strength {
        None => tone_map::auto_compress_dynamic_range(&mut px, &palette),
        Some(s) if s > 0.0 => tone_map::compress_dynamic_range(&mut px, &palette, s),
        _ => {}
    });
    Ok(flatten(px))
}

/// Gamut compression on a linear-RGB buffer. `strength=None` → full strength (1.0).
#[pyfunction]
#[pyo3(signature = (pixels, palette_bytes, strength=None))]
fn gamut_compress(py: Python<'_>, pixels: Vec<f64>, palette_bytes: &[u8], strength: Option<f64>) -> PyResult<Vec<f64>> {
    let mut px = to_linear_pixels(&pixels)?;
    let palette = palette_from_bytes(palette_bytes)?;
    let s = strength.unwrap_or(1.0);
    if s > 0.0 {
        py.detach(|| tone_map::gamut_compress(&mut px, &palette, s));
    }
    Ok(flatten(px))
}

/// Convert a flat linear-RGB f64 buffer to flat OKLab f64 (L, a, b per pixel).
#[pyfunction]
fn rgb_to_oklab_buffer(pixels: Vec<f64>) -> PyResult<Vec<f64>> {
    let px = to_linear_pixels(&pixels)?;
    Ok(px
        .into_iter()
        .flat_map(|[r, g, b]| {
            let lab = rgb_to_oklab(r, g, b);
            [lab.l, lab.a, lab.b]
        })
        .collect())
}

/// Returns all measured palettes from the Rust catalog.
///
/// Each entry is `(id, rgb_bytes, color_names, accent_idx, scheme_id)`.
#[pyfunction]
fn measured_palettes() -> Vec<(String, Vec<u8>, Vec<String>, usize, u8)> {
    CATALOG
        .iter()
        .map(|e| {
            (
                e.id.to_string(),
                e.palette.colors.iter().flatten().copied().collect(),
                e.color_names.iter().map(|&s| s.to_string()).collect(),
                e.palette.accent_idx,
                u8::from(e.scheme),
            )
        })
        .collect()
}

#[pymodule]
fn _rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(dither_image, m)?)?;
    m.add_function(wrap_pyfunction!(composite_rgba, m)?)?;
    m.add_function(wrap_pyfunction!(measured_palettes, m)?)?;
    m.add_function(wrap_pyfunction!(tone_compress, m)?)?;
    m.add_function(wrap_pyfunction!(gamut_compress, m)?)?;
    m.add_function(wrap_pyfunction!(rgb_to_oklab_buffer, m)?)?;
    Ok(())
}

use epaper_dithering_core::{
    color_space_lab::rgb_to_oklab,
    composite::composite_rgba_on_white,
    dbs::DbsParams,
    dither,
    enums::DitherMode,
    palette::{Ink, Lab, Palette, PaletteEntry},
    srgb_palette::SrgbPalette,
    tone_map,
    types::ImageBuffer,
    DitherOptions,
};
use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::types::PyBytes;

create_exception!(_rs, PaletteError, PyValueError, "A palette was rejected (invalid entries, file or normalisation).");

/// An entry as it crosses the boundary: (appearance L*a*b*, ink name, ink nominal sRGB).
type EntryTuple = ((f64, f64, f64), Option<String>, Option<(u8, u8, u8)>);

fn palette_err(e: impl ToString) -> PyErr {
    PaletteError::new_err(e.to_string())
}

fn entry_from_tuple(i: usize, ((l, a, b), name, nominal): EntryTuple) -> PyResult<PaletteEntry> {
    let ink = match (name, nominal) {
        (Some(name), Some((r, g, bl))) => Some(Ink { name, nominal: [r, g, bl] }),
        (None, None) => None,
        _ => return Err(palette_err(format!("entry {i}: an ink needs both a name and a nominal colour"))),
    };
    Ok(PaletteEntry { appearance: Lab { l, a, b }, ink })
}

fn entries_from_tuples(entries: Vec<EntryTuple>) -> PyResult<Vec<PaletteEntry>> {
    entries.into_iter().enumerate().map(|(i, e)| entry_from_tuple(i, e)).collect()
}

/// A validated palette. Construct through the `epaper_dithering.Palette` wrapper.
#[pyclass(name = "Palette", module = "epaper_dithering._rs", frozen)]
struct PyPalette {
    inner: Palette,
}

#[pymethods]
impl PyPalette {
    #[new]
    #[pyo3(signature = (entries, measured=false))]
    fn new(entries: Vec<EntryTuple>, measured: bool) -> PyResult<Self> {
        let inner = Palette::new(entries_from_tuples(entries)?, measured).map_err(palette_err)?;
        Ok(Self { inner })
    }

    #[staticmethod]
    fn from_json(s: &str) -> PyResult<Self> {
        Palette::from_json(s).map(|inner| Self { inner }).map_err(palette_err)
    }

    #[staticmethod]
    #[pyo3(signature = (entries, white_index, measured=false))]
    fn from_absolute_lab(entries: Vec<EntryTuple>, white_index: usize, measured: bool) -> PyResult<Self> {
        let inner = Palette::from_absolute_lab(entries_from_tuples(entries)?, white_index, measured).map_err(palette_err)?;
        Ok(Self { inner })
    }

    #[staticmethod]
    #[pyo3(signature = (colors, measured=false))]
    fn from_srgb(colors: Vec<((u8, u8, u8), Option<String>, Option<(u8, u8, u8)>)>, measured: bool) -> PyResult<Self> {
        let colors = colors
            .into_iter()
            .enumerate()
            .map(|(i, ((r, g, b), name, nominal))| {
                let ink = match (name, nominal) {
                    (Some(name), Some((nr, ng, nb))) => Some(Ink { name, nominal: [nr, ng, nb] }),
                    (None, None) => None,
                    _ => return Err(palette_err(format!("colour {i}: an ink needs both a name and a nominal colour"))),
                };
                Ok(([r, g, b], ink))
            })
            .collect::<PyResult<Vec<_>>>()?;
        Palette::from_srgb(colors, measured).map(|inner| Self { inner }).map_err(palette_err)
    }

    fn with_source(&self, source: String) -> Self {
        Self { inner: self.inner.clone().with_source(source) }
    }

    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    fn entries(&self) -> Vec<EntryTuple> {
        self.inner
            .entries()
            .iter()
            .map(|e| {
                let Lab { l, a, b } = e.appearance;
                let name = e.ink.as_ref().map(|i| i.name.clone());
                let nominal = e.ink.as_ref().map(|i| (i.nominal[0], i.nominal[1], i.nominal[2]));
                ((l, a, b), name, nominal)
            })
            .collect()
    }

    #[getter]
    fn measured(&self) -> bool {
        self.inner.measured()
    }

    #[getter]
    fn source(&self) -> Option<String> {
        self.inner.source().map(str::to_owned)
    }

    fn index_of(&self, name: &str) -> Option<u8> {
        self.inner.index_of(name)
    }

    fn preview_rgb(&self) -> Vec<(u8, u8, u8)> {
        self.inner.preview_srgb().iter().map(|&[r, g, b]| (r, g, b)).collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

/// `dbs_passes = None` disables DBS refinement; otherwise build and validate its parameters.
fn parse_dbs(passes: Option<u32>, viewing_distance_cm: f64, ppi: f64) -> PyResult<Option<DbsParams>> {
    passes
        .map(|max_passes| {
            let params = DbsParams { viewing_distance_cm, ppi, max_passes };
            params.validate().map(|()| params).map_err(|e| PyValueError::new_err(e.to_string()))
        })
        .transpose()
}

/// Dither a flat RGB image to `palette`. `mode` is a snake_case mode name; `tone` and
/// `gamut` are strengths from 0.0 (off) to 1.0. Returns one palette index per pixel.
#[pyfunction]
#[pyo3(signature = (
    pixels, width, height, palette, *,
    mode="burkes", serpentine=true,
    exposure=1.0, saturation=1.0, shadows=0.0, highlights=0.0,
    tone=0.0, gamut=0.0,
    dbs_passes=None, dbs_viewing_distance_cm=40.0, dbs_ppi=127.0,
    anchor=true, snap=true,
))]
#[allow(clippy::too_many_arguments)]
fn dither_image(
    py: Python<'_>,
    pixels: PyBackedBytes,
    width: usize,
    height: usize,
    palette: &PyPalette,
    mode: &str,
    serpentine: bool,
    exposure: f64,
    saturation: f64,
    shadows: f64,
    highlights: f64,
    tone: f64,
    gamut: f64,
    dbs_passes: Option<u32>,
    dbs_viewing_distance_cm: f64,
    dbs_ppi: f64,
    anchor: bool,
    snap: bool,
) -> PyResult<Vec<u8>> {
    let img = ImageBuffer::new(&pixels, width).map_err(|e| PyValueError::new_err(e.to_string()))?;
    if img.height != height {
        return Err(PyValueError::new_err(format!(
            "pixel buffer length ({}) does not match width × height × 3 ({width} × {height} × 3)",
            pixels.len()
        )));
    }
    let options = DitherOptions {
        mode: mode.parse::<DitherMode>().map_err(|e| PyValueError::new_err(e.to_string()))?,
        serpentine,
        exposure,
        saturation,
        shadows,
        highlights,
        tone,
        gamut,
        dbs: parse_dbs(dbs_passes, dbs_viewing_distance_cm, dbs_ppi)?,
        anchor,
        snap,
    };
    // Validation stays under the GIL; only the rayon-parallel dither releases it, so a
    // caller running us from an executor thread doesn't stall its event loop.
    let palette = &palette.inner;
    Ok(py.detach(|| dither(&img, palette, &options)))
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

// ── Pipeline stages for developer tooling (tools/pipeline.py) ────────────────────
//
// Not part of the public Python API. Input/output are flat linear-RGB f64.

fn to_linear_pixels(pixels: &[f64]) -> PyResult<Vec<[f64; 3]>> {
    if !pixels.len().is_multiple_of(3) {
        return Err(PyValueError::new_err("pixel buffer length must be a multiple of 3"));
    }
    Ok(pixels.as_chunks::<3>().0.to_vec())
}

fn flatten(pixels: Vec<[f64; 3]>) -> Vec<f64> {
    pixels.into_iter().flatten().collect()
}

/// Gamut compression on a linear-RGB buffer. `strength=None` → full strength (1.0).
#[pyfunction]
#[pyo3(signature = (pixels, palette, strength=None))]
fn gamut_compress(py: Python<'_>, pixels: Vec<f64>, palette: &PyPalette, strength: Option<f64>) -> PyResult<Vec<f64>> {
    let mut px = to_linear_pixels(&pixels)?;
    let srgb = SrgbPalette::from(&palette.inner);
    let s = strength.unwrap_or(1.0);
    if s > 0.0 {
        py.detach(|| tone_map::gamut_compress(&mut px, &srgb, s));
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

#[pymodule]
fn _rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPalette>()?;
    m.add("PaletteError", m.py().get_type::<PaletteError>())?;
    m.add_function(wrap_pyfunction!(dither_image, m)?)?;
    m.add_function(wrap_pyfunction!(composite_rgba, m)?)?;
    m.add_function(wrap_pyfunction!(gamut_compress, m)?)?;
    m.add_function(wrap_pyfunction!(rgb_to_oklab_buffer, m)?)?;
    Ok(())
}

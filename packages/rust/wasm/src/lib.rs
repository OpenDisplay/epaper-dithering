use epaper_dithering_core::{
    composite::composite_rgba_on_white,
    dbs::DbsParams,
    dither,
    enums::{DitherMode, GamutCompression},
    palette::{parse_nominal, Ink, Lab, Palette, PaletteEntry},
    types::ImageBuffer,
    DitherOptions,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct EntryIn {
    appearance: [f64; 3],
    name: Option<String>,
    nominal: Option<String>,
}

#[derive(Deserialize)]
struct SrgbIn {
    rgb: [u8; 3],
    name: Option<String>,
    nominal: Option<String>,
}

fn ink(i: usize, name: Option<String>, nominal: Option<String>) -> Result<Option<Ink>, JsValue> {
    match (name, nominal) {
        (Some(name), Some(nominal)) => Ok(Some(Ink { name, nominal: parse_nominal(&nominal).map_err(js_err)? })),
        (None, None) => Ok(None),
        _ => Err(js_err(format!("entry {i}: an ink needs both a name and a nominal colour"))),
    }
}

fn js_err(e: impl ToString) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// A validated palette. Build it from an `epaper-palette` JSON document; the TypeScript
/// `Palette` class wraps this.
#[wasm_bindgen(js_name = WasmPalette)]
pub struct WasmPalette {
    inner: Palette,
}

#[wasm_bindgen(js_class = WasmPalette)]
impl WasmPalette {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmPalette, JsValue> {
        Palette::from_json(json).map(|inner| WasmPalette { inner }).map_err(js_err)
    }

    /// Build from absolute measurements (`[{appearance, name?, nominal?}]` as JSON),
    /// normalising so `entries[white_index]` becomes L* 100, a* = b* = 0.
    #[wasm_bindgen(js_name = fromAbsoluteLab)]
    pub fn from_absolute_lab(entries_json: &str, white_index: usize, measured: bool) -> Result<WasmPalette, JsValue> {
        let raw: Vec<EntryIn> = serde_json::from_str(entries_json).map_err(js_err)?;
        let entries = raw
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                let [l, a, b] = e.appearance;
                Ok(PaletteEntry { appearance: Lab { l, a, b }, ink: ink(i, e.name, e.nominal)? })
            })
            .collect::<Result<Vec<_>, JsValue>>()?;
        Palette::from_absolute_lab(entries, white_index, measured).map(|inner| WasmPalette { inner }).map_err(js_err)
    }

    /// Build from sRGB colours (`[{rgb: [r, g, b], name?, nominal?}]` as JSON); normalises to
    /// the `white` ink if there is one.
    #[wasm_bindgen(js_name = fromSrgb)]
    pub fn from_srgb(colors_json: &str, measured: bool) -> Result<WasmPalette, JsValue> {
        let raw: Vec<SrgbIn> = serde_json::from_str(colors_json).map_err(js_err)?;
        let colors = raw
            .into_iter()
            .enumerate()
            .map(|(i, c)| Ok((c.rgb, ink(i, c.name, c.nominal)?)))
            .collect::<Result<Vec<_>, JsValue>>()?;
        Palette::from_srgb(colors, measured).map(|inner| WasmPalette { inner }).map_err(js_err)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        self.inner.to_json()
    }

    #[wasm_bindgen(js_name = indexOf)]
    pub fn index_of(&self, name: &str) -> Option<u8> {
        self.inner.index_of(name)
    }

    /// Flat `[r, g, b, r, g, b, …]` preview colours, one triple per entry.
    #[wasm_bindgen(js_name = previewRgb)]
    pub fn preview_rgb(&self) -> Vec<u8> {
        self.inner.preview_srgb().iter().flatten().copied().collect()
    }

    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.inner.len()
    }
}

fn parse_gamut(v: Option<f64>) -> GamutCompression {
    match v {
        None               => GamutCompression::Auto,
        Some(s) if s > 0.0 => GamutCompression::Fixed(s),
        _                  => GamutCompression::None,
    }
}

/// `dbs_passes = None` disables DBS refinement; otherwise build and validate its parameters.
fn parse_dbs(passes: Option<u32>, viewing_distance_cm: f64, ppi: f64) -> Result<Option<DbsParams>, JsValue> {
    passes
        .map(|max_passes| {
            let params = DbsParams { viewing_distance_cm, ppi, max_passes };
            params.validate().map(|()| params).map_err(js_err)
        })
        .transpose()
}

/// Dither a flat RGB image to `palette`. `mode` is a snake_case mode name.
///
/// Returns a `Uint8Array` of palette indices (one per pixel, length = width × height).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn dither_image(
    pixels: &[u8],
    width: usize,
    palette: &WasmPalette,
    mode: &str,
    serpentine: bool,
    exposure: f64,
    saturation: f64,
    shadows: f64,
    highlights: f64,
    gamut: Option<f64>,
    dbs_passes: Option<u32>,
    dbs_viewing_distance_cm: f64,
    dbs_ppi: f64,
    anchor: bool,
    snap: bool,
) -> Result<Vec<u8>, JsValue> {
    let img = ImageBuffer::new(pixels, width).map_err(js_err)?;
    let options = DitherOptions {
        mode: mode.parse::<DitherMode>().map_err(js_err)?,
        serpentine,
        exposure,
        saturation,
        shadows,
        highlights,
        gamut: parse_gamut(gamut),
        dbs: parse_dbs(dbs_passes, dbs_viewing_distance_cm, dbs_ppi)?,
        anchor,
        snap,
    };
    Ok(dither(&img, &palette.inner, &options))
}

/// Composite an RGBA buffer onto white, returning flat RGB bytes (sRGB).
///
/// Thin wrapper over `epaper_dithering_core::composite::composite_rgba_on_white` so that
/// JavaScript, Python and Rust all flatten alpha identically. Throws if the buffer length
/// is not a multiple of 4.
#[wasm_bindgen]
pub fn composite_rgba(rgba: &[u8]) -> Result<Vec<u8>, JsValue> {
    composite_rgba_on_white(rgba).map_err(js_err)
}

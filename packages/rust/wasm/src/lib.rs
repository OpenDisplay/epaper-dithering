use epaper_dithering_core::{
    dither, dither_with_canonical, DitherConfig,
    enums::{DitherMode, GamutCompression, ToneCompression},
    measured_palettes::CATALOG,
    palettes::{ColorScheme, Palette},
    types::ImageBuffer,
};
use wasm_bindgen::prelude::*;

fn parse_mode(v: u8) -> Result<DitherMode, JsValue> {
    DitherMode::try_from(v).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// `None` ⇒ Auto; otherwise Fixed(value). Negative or zero acts as "off" via Fixed semantics.
fn parse_tone(v: Option<f64>) -> ToneCompression {
    match v {
        None    => ToneCompression::Auto,
        Some(s) => ToneCompression::Fixed(s),
    }
}

fn parse_gamut(v: Option<f64>) -> GamutCompression {
    match v {
        None             => GamutCompression::Auto,
        Some(s) if s > 0.0 => GamutCompression::Fixed(s),
        _                => GamutCompression::None,
    }
}

/// Dither a flat RGB image. Accepts either an idealized `scheme_id` or a measured
/// `palette_bytes`/`accent_idx` pair; `palette_bytes` empty ⇒ use `scheme_id`.
///
/// Returns a `Uint8Array` of palette indices (one per pixel, length = width × height).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn dither_image(
    pixels: &[u8],
    width: usize,
    scheme_id: Option<u8>,
    palette_bytes: &[u8],
    accent_idx: usize,
    mode_id: u8,
    serpentine: bool,
    exposure: f64,
    saturation: f64,
    shadows: f64,
    highlights: f64,
    tone: Option<f64>,
    gamut: Option<f64>,
) -> Result<Vec<u8>, JsValue> {
    if width == 0 {
        return Err(JsValue::from_str("width must be greater than 0"));
    }
    if pixels.len() % 3 != 0 {
        return Err(JsValue::from_str("pixel buffer length must be a multiple of 3"));
    }
    if (pixels.len() / 3) % width != 0 {
        return Err(JsValue::from_str(
            "pixel count is not a whole number of rows for the given width",
        ));
    }
    let img = ImageBuffer::new(pixels, width);
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

    if palette_bytes.is_empty() {
        let id = scheme_id.ok_or_else(|| {
            JsValue::from_str("scheme_id is required when palette_bytes is empty")
        })?;
        let scheme = ColorScheme::try_from(id)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(dither(&img, scheme.palette(), config))
    } else {
        if !palette_bytes.len().is_multiple_of(3) {
            return Err(JsValue::from_str("palette_bytes length must be a multiple of 3"));
        }
        let colors: Vec<[u8; 3]> = palette_bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
        if colors.len() < 2 {
            return Err(JsValue::from_str("palette must have at least 2 colors"));
        }
        if accent_idx >= colors.len() {
            return Err(JsValue::from_str(&format!(
                "accent_idx {accent_idx} out of range for {}-color palette",
                colors.len()
            )));
        }
        let palette = Palette::new(colors, accent_idx);
        match scheme_id {
            Some(id) => {
                let scheme = ColorScheme::try_from(id)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                Ok(dither_with_canonical(&img, &palette, scheme.palette(), config))
            }
            None => Ok(dither(&img, palette, config)),
        }
    }
}

/// Composite an RGBA buffer onto white, returning flat RGB bytes (sRGB).
#[wasm_bindgen]
pub fn composite_rgba(rgba: &[u8]) -> Vec<u8> {
    let n = rgba.len() / 4;
    let mut rgb = vec![0u8; n * 3];
    for i in 0..n {
        let s = i * 4;
        let a = rgba[s + 3] as f64 / 255.0;
        let inv = 1.0 - a;
        rgb[i * 3]     = (rgba[s]     as f64 * a + 255.0 * inv).round() as u8;
        rgb[i * 3 + 1] = (rgba[s + 1] as f64 * a + 255.0 * inv).round() as u8;
        rgb[i * 3 + 2] = (rgba[s + 2] as f64 * a + 255.0 * inv).round() as u8;
    }
    rgb
}

/// Returns all measured palettes as a JSON string.
///
/// Format: `[{"id": "SPECTRA_7_3_6COLOR", "colors": [[r,g,b], ...], "color_names": [...], "accent_idx": 3, "scheme_id": 4}, ...]`
#[wasm_bindgen]
pub fn measured_palettes() -> String {
    let entries: Vec<String> = CATALOG
        .iter()
        .map(|e| {
            let colors: Vec<String> = e.palette.colors.iter()
                .map(|c| format!("[{},{},{}]", c[0], c[1], c[2]))
                .collect();
            let names: Vec<String> = e.color_names.iter()
                .map(|s| format!("\"{}\"", s))
                .collect();
            format!(
                "{{\"id\":\"{}\",\"colors\":[{}],\"color_names\":[{}],\"accent_idx\":{},\"scheme_id\":{}}}",
                e.id,
                colors.join(","),
                names.join(","),
                e.palette.accent_idx,
                u8::from(e.scheme),
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use vision_squeezer as vs;

fn parse_model(s: Option<&str>) -> Option<vs::VisionModel> {
    s.and_then(|m| match m.to_ascii_lowercase().as_str() {
        "claude" => Some(vs::VisionModel::Claude),
        "gpt4o" | "gpt-4o" => Some(vs::VisionModel::Gpt4o),
        "gpt5" | "gpt-5" | "gpt5.5" => Some(vs::VisionModel::Gpt5),
        "gemini" => Some(vs::VisionModel::Gemini15),
        _ => None,
    })
}

fn parse_format(s: Option<&str>) -> vs::OutputFormat {
    match s.unwrap_or("jpeg").to_ascii_lowercase().as_str() {
        "webp" => vs::OutputFormat::WebP,
        "avif" => vs::OutputFormat::Avif,
        _ => vs::OutputFormat::Jpeg,
    }
}

fn parse_mode(s: Option<&str>) -> vs::ProcessMode {
    match s.unwrap_or("auto").to_ascii_lowercase().as_str() {
        "ocr" => vs::ProcessMode::Ocr,
        "standard" => vs::ProcessMode::Standard,
        _ => vs::ProcessMode::Auto,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_cfg(
    quality: u8,
    tile_size: u32,
    crop: bool,
    bg_tolerance: u8,
    fmt: vs::OutputFormat,
    model: Option<vs::VisionModel>,
    max_tiles: Option<u32>,
    smart_crop: bool,
) -> vs::ProcessConfig {
    let mut b = vs::ProcessConfig::builder()
        .quality(quality)
        .tile_size(tile_size)
        .crop(crop)
        .bg_tolerance(bg_tolerance)
        .smart_crop(smart_crop)
        .output_format(fmt);
    if let Some(m) = model {
        b = b.target_model(m);
    }
    if let Some(t) = max_tiles {
        b = b.max_tiles(t);
    }
    b.build()
}

fn load_input(input: &Bound<'_, PyAny>) -> PyResult<(image::DynamicImage, u64)> {
    // Accept str/path (file path) or bytes/bytearray (raw image bytes).
    if let Ok(s) = input.extract::<String>() {
        let p = Path::new(&s);
        let sz = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        let img =
            image::open(p).map_err(|e| PyIOError::new_err(format!("open failed: {e}")))?;
        return Ok((img, sz));
    }
    if let Ok(b) = input.downcast::<PyBytes>() {
        let bytes = b.as_bytes();
        let img = image::load_from_memory(bytes)
            .map_err(|e| PyValueError::new_err(format!("decode failed: {e}")))?;
        return Ok((img, bytes.len() as u64));
    }
    Err(PyValueError::new_err(
        "input must be a path (str) or raw image bytes",
    ))
}

#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (input, model=None, mode="auto", format="jpeg", quality=75, tile_size=512, crop=true, bg_tolerance=15, max_tiles=None, output_path=None, smart_crop=false, auto_quality=None))]
fn optimize_image<'py>(
    py: Python<'py>,
    input: &Bound<'py, PyAny>,
    model: Option<&str>,
    mode: &str,
    format: &str,
    quality: u8,
    tile_size: u32,
    crop: bool,
    bg_tolerance: u8,
    max_tiles: Option<u32>,
    output_path: Option<&str>,
    smart_crop: bool,
    auto_quality: Option<f64>,
) -> PyResult<Bound<'py, PyDict>> {
    let (img, input_bytes) = load_input(input)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    let model_enum = parse_model(model);
    let fmt = parse_format(Some(format));
    let cfg = build_cfg(
        quality,
        tile_size,
        crop,
        bg_tolerance,
        fmt,
        model_enum,
        max_tiles,
        smart_crop,
    );
    let pmode = parse_mode(Some(mode));

    let mut result = vs::process(img, pmode, input_bytes, &cfg);
    let (bytes, used_quality) = if let Some(target) = auto_quality {
        vs::encode_with_auto_quality(&result.image, &cfg, target, 40, 95)
            .map_err(|e| PyValueError::new_err(format!("auto-quality failed: {e}")))?
    } else {
        let b = vs::encode_to_bytes(&result.image, &cfg)
            .map_err(|e| PyValueError::new_err(format!("encode failed: {e}")))?;
        (b, cfg.quality)
    };
    let output_bytes = bytes.len() as u64;
    result.report.bytes_after = Some(output_bytes);

    if let Some(p) = output_path {
        std::fs::write(p, &bytes)
            .map_err(|e| PyIOError::new_err(format!("write {p} failed: {e}")))?;
    }

    let m_for_tokens = cfg.target_model.unwrap_or(vs::VisionModel::Claude);
    let orig_tokens = vs::estimate_tokens(orig_w, orig_h, m_for_tokens).tokens;
    let opt_tokens = vs::estimate_tokens(result.width, result.height, m_for_tokens).tokens;

    let out = PyDict::new_bound(py);
    out.set_item("bytes", PyBytes::new_bound(py, &bytes))?;
    out.set_item("base64", B64.encode(&bytes))?;
    out.set_item("input_width", orig_w)?;
    out.set_item("input_height", orig_h)?;
    out.set_item("output_width", result.width)?;
    out.set_item("output_height", result.height)?;
    out.set_item("input_bytes", input_bytes)?;
    out.set_item("output_bytes", output_bytes)?;
    out.set_item("format", format!("{:?}", cfg.output_format).to_lowercase())?;
    out.set_item("quality", used_quality)?;
    out.set_item("auto_quality_target", auto_quality)?;
    out.set_item("smart_crop", cfg.smart_crop)?;
    out.set_item("tiles_before", result.report.tiles_before)?;
    out.set_item("tiles_after", result.report.tiles_after)?;
    out.set_item("tiles_saved", result.report.tiles_saved)?;
    out.set_item("tokens_before", orig_tokens)?;
    out.set_item("tokens_after", opt_tokens)?;
    out.set_item("tokens_saved", orig_tokens.saturating_sub(opt_tokens))?;
    out.set_item(
        "size_reduction_pct",
        result.report.size_reduction_pct().unwrap_or(0.0),
    )?;
    Ok(out)
}

#[pyfunction]
#[pyo3(signature = (width, height, model="claude"))]
fn estimate_tokens<'py>(
    py: Python<'py>,
    width: u32,
    height: u32,
    model: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let m = parse_model(Some(model))
        .ok_or_else(|| PyValueError::new_err(format!("unknown model: {model}")))?;
    let est = vs::estimate_tokens(width, height, m);
    let out = PyDict::new_bound(py);
    out.set_item("model", model)?;
    out.set_item("tokens", est.tokens)?;
    out.set_item("tiles", est.tiles)?;
    Ok(out)
}

#[pyfunction]
#[pyo3(signature = (width, height, model="claude"))]
fn optimal_dimensions<'py>(
    py: Python<'py>,
    width: u32,
    height: u32,
    model: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let m = parse_model(Some(model))
        .ok_or_else(|| PyValueError::new_err(format!("unknown model: {model}")))?;
    let (w, h) = vs::optimal_send_dimensions(width, height, m);
    let out = PyDict::new_bound(py);
    out.set_item("width", w)?;
    out.set_item("height", h)?;
    Ok(out)
}

#[pymodule]
#[pyo3(name = "vision_squeezer")]
fn vs_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(optimize_image, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(optimal_dimensions, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::Utc;
use image::{DynamicImage, ImageBuffer, Luma, imageops::FilterType};
use rusqlite::{Connection, params};
use std::path::PathBuf;
// ── Config ────────────────────────────────────────────────────────────────────

/// Output encoding format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// JPEG at configured quality (default).
    #[default]
    Jpeg,
    /// WebP at configured quality — typically 30-50% smaller than JPEG at equal quality.
    WebP,
}

/// All tuneable knobs for the pipeline.
#[derive(Clone, Debug)]
pub struct ProcessConfig {
    /// Output quality 1–100 (default 75). Applies to both JPEG and WebP.
    pub quality: u8,
    /// LLM patch size in pixels. Overridden when `target_model` is set.
    pub tile_size: u32,
    /// Remove solid-color padding borders before resizing (default true).
    pub crop: bool,
    /// Max channel delta to treat a pixel as background (default 15).
    pub bg_tolerance: u8,
    /// Output encoding format (default: JPEG).
    pub output_format: OutputFormat,
    /// When set, resizing is model-aware (accounts for pre-scaling behavior).
    pub target_model: Option<VisionModel>,
    /// Limit the maximum number of tiles the output image can consume.
    pub max_tiles: Option<u32>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            quality: 75,
            tile_size: 512,
            crop: true,
            bg_tolerance: 15,
            output_format: OutputFormat::Jpeg,
            target_model: None,
            max_tiles: None,
        }
    }
}

impl ProcessConfig {
    pub fn builder() -> ProcessConfigBuilder {
        ProcessConfigBuilder(Self::default())
    }
}

pub struct ProcessConfigBuilder(ProcessConfig);

impl ProcessConfigBuilder {
    pub fn quality(mut self, q: u8) -> Self {
        self.0.quality = q.clamp(1, 100);
        self
    }
    pub fn tile_size(mut self, t: u32) -> Self {
        self.0.tile_size = t.max(1);
        self
    }
    pub fn crop(mut self, c: bool) -> Self {
        self.0.crop = c;
        self
    }
    pub fn bg_tolerance(mut self, t: u8) -> Self {
        self.0.bg_tolerance = t;
        self
    }
    pub fn output_format(mut self, f: OutputFormat) -> Self {
        self.0.output_format = f;
        self
    }
    pub fn target_model(mut self, m: VisionModel) -> Self {
        self.0.target_model = Some(m);
        self
    }
    pub fn max_tiles(mut self, m: u32) -> Self {
        self.0.max_tiles = Some(m);
        self
    }
    pub fn build(self) -> ProcessConfig {
        self.0
    }
}

// ── Token Estimation ──────────────────────────────────────────────────────────

/// Supported vision model families with their patch pricing.
#[derive(Clone, Copy, Debug)]
pub enum VisionModel {
    /// Claude 3.5/4.5/4.6/4.7: Area-based calculation (Tokens ≈ width × height / 750).
    Claude,
    /// GPT-4o / GPT-4.5 high detail: fits in 2048x2048, scales short side to 768, then 512x512 tiles.
    Gpt4o,
    /// GPT-5/5.5: 6000px max dim, 10.24M max pixels, 512×512 tiles, 1536 token cap.
    Gpt5,
    /// Gemini 2.0/3.0: flat 258 tokens if ≤ 384x384, else 258 per 768x768 tile.
    Gemini15,
}

#[derive(Debug)]
pub struct TokenEstimate {
    pub model: VisionModel,
    pub tokens: u32,
    pub tiles: u32,
}

/// Estimate LLM vision tokens for an image of given dimensions.
pub fn estimate_tokens(width: u32, height: u32, model: VisionModel) -> TokenEstimate {
    match model {
        VisionModel::Claude => {
            // 2026 area-based pricing for Claude
            let tokens = ((width as u64 * height as u64) / 750) as u32;
            TokenEstimate {
                model,
                tiles: 1,
                tokens: tokens.max(85),
            }
        }
        VisionModel::Gpt4o => {
            // GPT-4o / 4.5: fit within 2048x2048, then short side scaled to 768px, then 512x512 tiles.
            let (mut w, mut h) = fit_within(width, height, 2048);
            let short_side = w.min(h);
            if short_side > 768 {
                let scale = 768.0 / short_side as f64;
                w = (w as f64 * scale).round() as u32;
                h = (h as f64 * scale).round() as u32;
            }
            let tiles = tile_count(w, 512) * tile_count(h, 512);
            TokenEstimate {
                model,
                tiles,
                tokens: 85 + tiles * 170,
            }
        }
        VisionModel::Gpt5 => {
            let (w, h) = fit_within_pixels(width, height, 6000, 10_240_000);
            let tiles = tile_count(w, 512) * tile_count(h, 512);
            TokenEstimate {
                model,
                tiles,
                tokens: (85 + tiles * 170).min(1536),
            }
        }
        VisionModel::Gemini15 => {
            // Gemini 2026: flat 258 if <= 384x384, else 768x768 tiles.
            if width <= 384 && height <= 384 {
                TokenEstimate {
                    model,
                    tiles: 1,
                    tokens: 258,
                }
            } else {
                let tiles = tile_count(width, 768) * tile_count(height, 768);
                TokenEstimate {
                    model,
                    tiles,
                    tokens: tiles * 258,
                }
            }
        }
    }
}

/// Scale dimensions to fit within `max_side` while preserving aspect ratio.
pub fn fit_within(width: u32, height: u32, max_side: u32) -> (u32, u32) {
    if width <= max_side && height <= max_side {
        return (width, height);
    }
    let scale = max_side as f64 / width.max(height) as f64;
    (
        (width as f64 * scale) as u32,
        (height as f64 * scale) as u32,
    )
}

/// Scale dimensions to fit within both a max-side limit and a total-pixel limit.
pub fn fit_within_pixels(width: u32, height: u32, max_side: u32, max_pixels: u64) -> (u32, u32) {
    let (mut w, mut h) = fit_within(width, height, max_side);
    let total = w as u64 * h as u64;
    if total > max_pixels {
        let scale = (max_pixels as f64 / total as f64).sqrt();
        w = (w as f64 * scale) as u32;
        h = (h as f64 * scale) as u32;
    }
    (w.max(1), h.max(1))
}

/// Compute the optimal dimensions to *send* to a given model to minimize tiles.
///
/// For models that pre-scale images (GPT-4o, Gemini), we simulate their scaling,
/// snap the scaled result to tile boundaries, then invert back to input space.
/// For Claude (no pre-scaling), we snap the input directly.
pub fn optimal_send_dimensions(width: u32, height: u32, model: VisionModel) -> (u32, u32) {
    match model {
        VisionModel::Claude => {
            // Claude is now area-based, so tiling doesn't dictate a specific rigid boundary.
            // But we still snap to 256 or 512 so dimensions aren't completely arbitrary.
            (
                snap_to_tile_boundary(width, 256),
                snap_to_tile_boundary(height, 256),
            )
        }
        VisionModel::Gpt4o => optimal_for_prescaling_model(width, height, 2048, 512),
        VisionModel::Gpt5 => {
            let (fw, fh) = fit_within_pixels(width, height, 6000, 10_240_000);
            (
                snap_to_tile_boundary(fw, 512).max(512),
                snap_to_tile_boundary(fh, 512).max(512),
            )
        }
        VisionModel::Gemini15 => {
            // Gemini uses 768x768 tiles if > 384x384
            if width <= 384 && height <= 384 {
                (width, height)
            } else {
                optimal_for_prescaling_model(width, height, 4096, 768)
            }
        }
    }
}

/// For models that pre-scale (GPT-4o, Gemini), find the smallest input dimensions
/// that, after the model's internal fit-within + tiling, produce the fewest tiles.
///
/// Strategy: enumerate candidate tile-grid dimensions (tw*tile, th*tile) that fit
/// within max_side, compute the input size that would map to each, and pick the
/// candidate that uses the fewest tiles while preserving the original aspect ratio
/// as closely as possible.
fn optimal_for_prescaling_model(width: u32, height: u32, max_side: u32, tile: u32) -> (u32, u32) {
    let (fw, fh) = fit_within(width, height, max_side);

    // Simply snap the fitted dimensions to the nearest tile boundary
    let target_w = snap_to_tile_boundary(fw, tile).max(tile);
    let target_h = snap_to_tile_boundary(fh, tile).max(tile);

    // If image was larger than max_side, scale back to input space
    if width > max_side || height > max_side {
        let scale = width.max(height) as f64 / max_side as f64;
        let opt_w = (target_w as f64 * scale).round() as u32;
        let opt_h = (target_h as f64 * scale).round() as u32;
        return (opt_w.max(1), opt_h.max(1));
    }

    (target_w, target_h)
}

/// Full token savings report for a before/after dimension pair across all models.
pub struct TokenSavingsTable {
    pub claude_before: TokenEstimate,
    pub claude_after: TokenEstimate,
    pub gpt4o_before: TokenEstimate,
    pub gpt4o_after: TokenEstimate,
    pub gpt5_before: TokenEstimate,
    pub gpt5_after: TokenEstimate,
    pub gemini_before: TokenEstimate,
    pub gemini_after: TokenEstimate,
}

pub fn token_savings_table(orig_w: u32, orig_h: u32, opt_w: u32, opt_h: u32) -> TokenSavingsTable {
    TokenSavingsTable {
        claude_before: estimate_tokens(orig_w, orig_h, VisionModel::Claude),
        claude_after: estimate_tokens(opt_w, opt_h, VisionModel::Claude),
        gpt4o_before: estimate_tokens(orig_w, orig_h, VisionModel::Gpt4o),
        gpt4o_after: estimate_tokens(opt_w, opt_h, VisionModel::Gpt4o),
        gpt5_before: estimate_tokens(orig_w, orig_h, VisionModel::Gpt5),
        gpt5_after: estimate_tokens(opt_w, opt_h, VisionModel::Gpt5),
        gemini_before: estimate_tokens(orig_w, orig_h, VisionModel::Gemini15),
        gemini_after: estimate_tokens(opt_w, opt_h, VisionModel::Gemini15),
    }
}

impl TokenSavingsTable {
    pub fn print(&self) {
        println!(
            "{:<12} {:>8} {:>8} {:>10}",
            "Model", "Before", "After", "Saved"
        );
        println!("{}", "-".repeat(42));
        self.print_row("Claude", &self.claude_before, &self.claude_after);
        self.print_row("GPT-4o", &self.gpt4o_before, &self.gpt4o_after);
        self.print_row("GPT-5", &self.gpt5_before, &self.gpt5_after);
        self.print_row("Gemini", &self.gemini_before, &self.gemini_after);
    }

    fn print_row(&self, name: &str, before: &TokenEstimate, after: &TokenEstimate) {
        let saved = before.tokens.saturating_sub(after.tokens);
        let pct = if before.tokens > 0 {
            saved as f64 / before.tokens as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "{:<12} {:>8} {:>8} {:>8} ({:.1}%)",
            name, before.tokens, after.tokens, saved, pct
        );
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

pub struct DimensionResult {
    pub width: u32,
    pub height: u32,
    pub tiles_before: u32,
    pub tiles_after: u32,
}

impl DimensionResult {
    pub fn tokens_saved(&self) -> u32 {
        self.tiles_before.saturating_sub(self.tiles_after)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProcessMode {
    /// General LLM vision — JPEG output at configured quality.
    Standard,
    /// Text extraction — high-contrast grayscale binarization (Otsu threshold).
    Ocr,
    /// Auto-detects if the image is mostly text (monochrome/grayscale).
    #[default]
    Auto,
}

pub fn detect_ocr_mode(img: &DynamicImage) -> bool {
    let rgb = img.to_rgb8();
    let mut colorful_count = 0;
    let mut total_count = 0;
    // Sample every 4th pixel for speed
    for (x, y, p) in rgb.enumerate_pixels() {
        if x % 4 == 0 && y % 4 == 0 {
            total_count += 1;
            let min = p[0].min(p[1]).min(p[2]);
            let max = p[0].max(p[1]).max(p[2]);
            if max.saturating_sub(min) > 25 {
                colorful_count += 1;
            }
        }
    }
    let colorful_ratio = colorful_count as f64 / total_count.max(1) as f64;
    colorful_ratio < 0.1 // if less than 10% of pixels are colorful, assume OCR
}

pub struct SavingsReport {
    pub tiles_before: u32,
    pub tiles_after: u32,
    pub tiles_saved: u32,
    pub bytes_before: Option<u64>,
    pub bytes_after: Option<u64>,
}

impl SavingsReport {
    pub fn size_reduction_pct(&self) -> Option<f64> {
        match (self.bytes_before, self.bytes_after) {
            (Some(b), Some(a)) if b > 0 => Some((1.0 - a as f64 / b as f64) * 100.0),
            _ => None,
        }
    }

    pub fn token_reduction_pct(&self) -> f64 {
        if self.tiles_before == 0 {
            return 0.0;
        }
        self.tiles_saved as f64 / self.tiles_before as f64 * 100.0
    }
}

pub struct ProcessResult {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
    pub report: SavingsReport,
}

impl ProcessResult {
    pub fn tokens_saved(&self) -> u32 {
        self.report.tiles_saved
    }
}

// ── Pipeline ──────────────────────────────────────────────────────────────────

/// Full pipeline: [crop] → tile-snap resize → [OCR binarize].
/// Pass `input_bytes = 0` if unknown (omits file-size from report).
pub fn process(
    img: DynamicImage,
    mode: ProcessMode,
    input_bytes: u64,
    cfg: &ProcessConfig,
) -> ProcessResult {
    let (orig_w, orig_h) = (img.width(), img.height());
    let tiles_before = match cfg.target_model {
        Some(model) => estimate_tokens(orig_w, orig_h, model).tiles,
        None => tile_count(orig_w, cfg.tile_size) * tile_count(orig_h, cfg.tile_size),
    };

    let after_crop = if cfg.crop {
        crop_padding(img, cfg.bg_tolerance)
    } else {
        img
    };
    let (mut opt_w, mut opt_h) = match cfg.target_model {
        Some(model) => optimal_send_dimensions(after_crop.width(), after_crop.height(), model),
        None => {
            let d = calculate_optimal_dimensions_with(
                after_crop.width(),
                after_crop.height(),
                cfg.tile_size,
            );
            (d.width, d.height)
        }
    };

    if let Some(max_t) = cfg.max_tiles {
        let (nw, nh) = enforce_max_tiles(opt_w, opt_h, max_t, cfg.tile_size, cfg.target_model);
        opt_w = nw;
        opt_h = nh;
    }

    let tiles_after = match cfg.target_model {
        Some(model) => {
            let est = estimate_tokens(opt_w, opt_h, model);
            est.tiles
        }
        None => tile_count(opt_w, cfg.tile_size) * tile_count(opt_h, cfg.tile_size),
    };
    let resized = after_crop.resize_exact(opt_w, opt_h, FilterType::Lanczos3);

    let actual_mode = match mode {
        ProcessMode::Auto => {
            if detect_ocr_mode(&after_crop) {
                ProcessMode::Ocr
            } else {
                ProcessMode::Standard
            }
        }
        m => m,
    };

    let final_image = match actual_mode {
        ProcessMode::Standard | ProcessMode::Auto => resized,
        ProcessMode::Ocr => binarize(resized),
    };

    ProcessResult {
        width: final_image.width(),
        height: final_image.height(),
        image: final_image,
        report: SavingsReport {
            tiles_before,
            tiles_after,
            tiles_saved: tiles_before.saturating_sub(tiles_after),
            bytes_before: if input_bytes > 0 {
                Some(input_bytes)
            } else {
                None
            },
            bytes_after: None,
        },
    }
}

fn enforce_max_tiles(
    mut width: u32,
    mut height: u32,
    max_tiles: u32,
    default_tile_size: u32,
    model: Option<VisionModel>,
) -> (u32, u32) {
    if max_tiles == 0 {
        return (width, height);
    }

    let mut scale = 1.0;
    let orig_w = width;
    let orig_h = height;

    loop {
        let (snapped_w, snapped_h) = match model {
            Some(m) => optimal_send_dimensions(width, height, m),
            None => {
                let d = calculate_optimal_dimensions_with(width, height, default_tile_size);
                (d.width, d.height)
            }
        };

        let tiles = match model {
            Some(m) => estimate_tokens(snapped_w, snapped_h, m).tiles,
            None => {
                tile_count(snapped_w, default_tile_size) * tile_count(snapped_h, default_tile_size)
            }
        };

        if tiles <= max_tiles || scale < 0.1 {
            return (snapped_w, snapped_h);
        }

        scale *= 0.95;
        width = (orig_w as f64 * scale) as u32;
        height = (orig_h as f64 * scale) as u32;
        width = width.max(1);
        height = height.max(1);
    }
}

// ── Step 1: Tile-Aware Dimension Calculation ───────────────────────────────────

/// Snap W×H to tile boundaries using default tile size (512).
pub fn calculate_optimal_dimensions(width: u32, height: u32) -> DimensionResult {
    calculate_optimal_dimensions_with(width, height, 512)
}

/// Snap W×H to tile boundaries using a custom tile size.
pub fn calculate_optimal_dimensions_with(
    width: u32,
    height: u32,
    tile_size: u32,
) -> DimensionResult {
    let opt_w = snap_to_tile_boundary(width, tile_size);
    let opt_h = snap_to_tile_boundary(height, tile_size);

    DimensionResult {
        width: opt_w,
        height: opt_h,
        tiles_before: tile_count(width, tile_size) * tile_count(height, tile_size),
        tiles_after: tile_count(opt_w, tile_size) * tile_count(opt_h, tile_size),
    }
}

fn tile_count(dim: u32, tile_size: u32) -> u32 {
    dim.div_ceil(tile_size)
}

fn snap_to_tile_boundary(dim: u32, tile_size: u32) -> u32 {
    if dim.is_multiple_of(tile_size) {
        return dim;
    }
    ((dim / tile_size) * tile_size).max(tile_size)
}

// ── Step 2: Semantic Crop (padding removal) ────────────────────────────────────

/// Remove solid-color borders using corner sampling + configurable tolerance.
pub fn crop_padding(img: DynamicImage, bg_tolerance: u8) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let corners = [
        *rgba.get_pixel(0, 0),
        *rgba.get_pixel(w - 1, 0),
        *rgba.get_pixel(0, h - 1),
        *rgba.get_pixel(w - 1, h - 1),
    ];
    let bg = corners[0]; // first corner as background reference

    let top = first_non_bg_row(&rgba, bg, bg_tolerance, true);
    let bottom = first_non_bg_row(&rgba, bg, bg_tolerance, false);
    let left = first_non_bg_col(&rgba, bg, bg_tolerance, true);
    let right = first_non_bg_col(&rgba, bg, bg_tolerance, false);

    if top >= bottom || left >= right {
        return DynamicImage::ImageRgba8(rgba);
    }

    DynamicImage::ImageRgba8(
        image::imageops::crop_imm(&rgba, left, top, right - left, bottom - top).to_image(),
    )
}

fn is_bg(pixel: image::Rgba<u8>, bg: image::Rgba<u8>, tolerance: u8) -> bool {
    pixel.0[3] < 10
        || pixel.0[..3]
            .iter()
            .zip(bg.0[..3].iter())
            .all(|(&a, &b)| a.abs_diff(b) <= tolerance)
}

fn first_non_bg_row(img: &image::RgbaImage, bg: image::Rgba<u8>, tol: u8, from_top: bool) -> u32 {
    let (w, h) = img.dimensions();
    let rows: Box<dyn Iterator<Item = u32>> = if from_top {
        Box::new(0..h)
    } else {
        Box::new((0..h).rev())
    };
    for y in rows {
        if (0..w).any(|x| !is_bg(*img.get_pixel(x, y), bg, tol)) {
            return y;
        }
    }
    0
}

fn first_non_bg_col(img: &image::RgbaImage, bg: image::Rgba<u8>, tol: u8, from_left: bool) -> u32 {
    let (w, h) = img.dimensions();
    let cols: Box<dyn Iterator<Item = u32>> = if from_left {
        Box::new(0..w)
    } else {
        Box::new((0..w).rev())
    };
    for x in cols {
        if (0..h).any(|y| !is_bg(*img.get_pixel(x, y), bg, tol)) {
            return x;
        }
    }
    0
}

// ── Step 3: OCR Binarization ───────────────────────────────────────────────────

pub fn binarize(img: DynamicImage) -> DynamicImage {
    let gray = img.to_luma8();
    let (w, h) = gray.dimensions();
    let threshold = otsu_threshold(&gray);
    let binary: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, y| {
        let p = gray.get_pixel(x, y).0[0];
        Luma([if p < threshold { 0u8 } else { 255u8 }])
    });
    DynamicImage::ImageLuma8(binary)
}

fn otsu_threshold(img: &image::GrayImage) -> u8 {
    let mut histogram = [0u32; 256];
    for p in img.pixels() {
        histogram[p.0[0] as usize] += 1;
    }
    let total = img.width() * img.height();
    let (mut sum, mut sum_bg, mut weight_bg) = (0f64, 0f64, 0f64);
    for (i, &h) in histogram.iter().enumerate() {
        sum += i as f64 * h as f64;
    }
    let (mut best_thresh, mut best_var) = (0u8, 0f64);
    for (t, &h) in histogram.iter().enumerate() {
        weight_bg += h as f64;
        if weight_bg == 0.0 {
            continue;
        }
        let weight_fg = total as f64 - weight_bg;
        if weight_fg == 0.0 {
            break;
        }
        sum_bg += t as f64 * h as f64;
        let mean_bg = sum_bg / weight_bg;
        let mean_fg = (sum - sum_bg) / weight_fg;
        let var = weight_bg * weight_fg * (mean_bg - mean_fg).powi(2);
        if var > best_var {
            best_var = var;
            best_thresh = t as u8;
        }
    }
    best_thresh
}

// ── Base64 I/O ────────────────────────────────────────────────────────────────

pub fn decode_base64_image(input: &str) -> Result<DynamicImage, String> {
    let data = if let Some(c) = input.find(',') {
        &input[c + 1..]
    } else {
        input
    };
    let bytes = B64.decode(data.trim()).map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

pub fn encode_image_base64(img: &DynamicImage, cfg: &ProcessConfig) -> Result<String, String> {
    let bytes = encode_to_bytes(img, cfg)?;
    Ok(B64.encode(bytes))
}

/// Encode image to raw bytes using the configured output format.
pub fn encode_to_bytes(img: &DynamicImage, cfg: &ProcessConfig) -> Result<Vec<u8>, String> {
    match cfg.output_format {
        OutputFormat::Jpeg => {
            use image::codecs::jpeg::JpegEncoder;
            let mut buf = Cursor::new(Vec::new());
            let rgb = img.to_rgb8();
            JpegEncoder::new_with_quality(&mut buf, cfg.quality)
                .encode_image(&DynamicImage::ImageRgb8(rgb))
                .map_err(|e| e.to_string())?;
            Ok(buf.into_inner())
        }
        OutputFormat::WebP => {
            let rgb = img.to_rgb8();
            let enc = webp::Encoder::from_rgb(rgb.as_raw(), rgb.width(), rgb.height());
            let mem = enc.encode(cfg.quality as f32);
            Ok(mem.to_vec())
        }
    }
}

// ── MCP Tool: optimize_image ──────────────────────────────────────────────────

pub struct OptimizeResult {
    pub optimized_base64: String,
    pub report: SavingsReport,
    pub original_width: u32,
    pub original_height: u32,
    pub width: u32,
    pub height: u32,
    pub optimized_bytes: usize,
}

/// MCP entry point: base64 in → base64 JPEG out + savings report.
pub fn optimize_image(
    input_base64: &str,
    mode: ProcessMode,
    cfg: &ProcessConfig,
) -> Result<OptimizeResult, String> {
    let img = decode_base64_image(input_base64)?;
    let (orig_w, orig_h) = (img.width(), img.height());
    let input_bytes = {
        let data = if let Some(c) = input_base64.find(',') {
            &input_base64[c + 1..]
        } else {
            input_base64
        };
        B64.decode(data.trim()).map_err(|e| e.to_string())?.len() as u64
    };

    let mut result = process(img, mode, input_bytes, cfg);
    let bytes = encode_to_bytes(&result.image, cfg)?;
    let encoded = B64.encode(&bytes);
    result.report.bytes_after = Some(bytes.len() as u64);

    Ok(OptimizeResult {
        optimized_base64: encoded,
        report: result.report,
        original_width: orig_w,
        original_height: orig_h,
        width: result.width,
        height: result.height,
        optimized_bytes: bytes.len(),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

// ── Step 4: Sandbox (Think in Code) ──────────────────────────────────────────

/// Atomic image operations for the Sandbox mode.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase", tag = "op")]
pub enum ImageOp {
    /// Crop a specific region: { x, y, width, height }
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    /// Convert to grayscale.
    Grayscale,
    /// Binarize using Otsu's threshold (if threshold is None).
    Binarize { threshold: Option<u8> },
    /// Resize to exact dimensions.
    Resize { width: u32, height: u32 },
    /// Adjust contrast (e.g., 2.0 for double contrast).
    Contrast { amount: f32 },
    /// Adjust brightness (e.g., -20 to darken).
    Brightness { amount: f32 },
}

/// Execute a sequence of operations on an image.
pub fn process_with_operations(mut img: DynamicImage, ops: Vec<ImageOp>) -> DynamicImage {
    for op in ops {
        img = match op {
            ImageOp::Crop {
                x,
                y,
                width,
                height,
            } => img.crop_imm(x, y, width, height),
            ImageOp::Grayscale => DynamicImage::ImageLuma8(img.to_luma8()),
            ImageOp::Binarize { threshold } => {
                let gray = img.to_luma8();
                let thr = threshold.unwrap_or(128);
                let mut binarized = ImageBuffer::new(gray.width(), gray.height());
                for (x, y, p) in gray.enumerate_pixels() {
                    let val = if p[0] > thr { 255 } else { 0 };
                    binarized.put_pixel(x, y, Luma([val]));
                }
                DynamicImage::ImageLuma8(binarized)
            }
            ImageOp::Resize { width, height } => {
                img.resize_exact(width, height, FilterType::Lanczos3)
            }
            ImageOp::Contrast { amount } => img.adjust_contrast(amount),
            ImageOp::Brightness { amount } => img.brighten(amount as i32),
        };
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ProcessConfig {
        ProcessConfig::default()
    }

    #[test]
    fn exact_boundary_unchanged() {
        let r = calculate_optimal_dimensions(1024, 512);
        assert_eq!((r.width, r.height), (1024, 512));
        assert_eq!(r.tokens_saved(), 0);
    }

    #[test]
    fn one_pixel_over_saves_full_tile_row() {
        let r = calculate_optimal_dimensions(1025, 1025);
        assert_eq!((r.width, r.height), (1024, 1024));
        assert_eq!(r.tiles_before, 9);
        assert_eq!(r.tiles_after, 4);
        assert_eq!(r.tokens_saved(), 5);
    }

    #[test]
    fn small_image_never_below_one_tile() {
        let r = calculate_optimal_dimensions(100, 200);
        assert_eq!((r.width, r.height), (512, 512));
    }

    #[test]
    fn mid_boundary_snaps_down() {
        let r = calculate_optimal_dimensions(768, 512);
        assert_eq!(r.width, 512);
        assert_eq!(r.tiles_after, 1);
    }

    #[test]
    fn custom_tile_size_256() {
        let r = calculate_optimal_dimensions_with(257, 512, 256);
        assert_eq!(r.width, 256); // 257 → snaps down to 256
        assert_eq!(r.tiles_before, 2 * 2); // ceil(257/256)*ceil(512/256) = 2*2
        assert_eq!(r.tiles_after, 1 * 2); // 256/256 * 512/256 = 1*2
    }

    #[test]
    fn full_pipeline_reduces_tiles() {
        use image::{DynamicImage, Rgba, RgbaImage};
        let mut img = RgbaImage::from_pixel(1025, 1025, Rgba([255, 255, 255, 255]));
        for x in 400..600 {
            for y in 400..600 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let result = process(
            DynamicImage::ImageRgba8(img),
            ProcessMode::Standard,
            0,
            &cfg(),
        );
        assert!(result.report.tiles_after < result.report.tiles_before);
    }

    #[test]
    fn crop_disabled_preserves_size() {
        use image::{DynamicImage, Rgba, RgbaImage};
        let img = RgbaImage::from_pixel(1024, 1024, Rgba([255, 255, 255, 255]));
        let no_crop = ProcessConfig::builder().crop(false).build();
        let result = process(
            DynamicImage::ImageRgba8(img),
            ProcessMode::Standard,
            0,
            &no_crop,
        );
        assert_eq!(result.width, 1024);
    }

    #[test]
    fn crop_removes_white_border() {
        use image::{Rgba, RgbaImage};
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
        for x in 45..55 {
            for y in 45..55 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let cropped = crop_padding(DynamicImage::ImageRgba8(img), 15);
        assert!(cropped.width() < 100 && cropped.height() < 100);
    }

    #[test]
    fn binarize_produces_only_black_white() {
        use image::{DynamicImage, GrayImage, Luma};
        let img = GrayImage::from_fn(64, 64, |x, _| Luma([if x < 32 { 50u8 } else { 200u8 }]));
        let result = binarize(DynamicImage::ImageLuma8(img)).to_luma8();
        for p in result.pixels() {
            assert!(p.0[0] == 0 || p.0[0] == 255);
        }
    }

    #[test]
    fn high_bg_tolerance_crops_more() {
        use image::{DynamicImage, Rgba, RgbaImage};
        // Corners: pure white [255,255,255]. Border: off-white [240,240,240]. Center: black.
        // diff = 15. strict(5): 15 > 5 → border NOT bg → no crop.
        // loose(20): 15 ≤ 20 → border IS bg → crops.
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([240, 240, 240, 255]));
        for corner in [(0u32, 0u32), (99, 0), (0, 99), (99, 99)] {
            img.put_pixel(corner.0, corner.1, Rgba([255, 255, 255, 255]));
        }
        for x in 45..55 {
            for y in 45..55 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let strict = crop_padding(DynamicImage::ImageRgba8(img.clone()), 5);
        let loose = crop_padding(DynamicImage::ImageRgba8(img), 20);
        assert!(loose.width() < strict.width());
    }
}
// ── Persistence & Analytics ───────────────────────────────────────────────────

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OptimizationReport {
    pub timestamp: String,
    pub model: String,
    pub original_tokens: u32,
    pub optimized_tokens: u32,
    pub original_bytes: u64,
    pub optimized_bytes: u64,
    pub mode: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SqueezerStats {
    pub total_optimizations: u64,
    pub total_original_tokens: u64,
    pub total_optimized_tokens: u64,
    pub total_original_bytes: u64,
    pub total_optimized_bytes: u64,
    pub history: Vec<OptimizationReport>,
}

impl SqueezerStats {
    pub fn total_token_savings(&self) -> u64 {
        self.total_original_tokens
            .saturating_sub(self.total_optimized_tokens)
    }

    pub fn total_byte_savings(&self) -> u64 {
        self.total_original_bytes
            .saturating_sub(self.total_optimized_bytes)
    }

    pub fn estimated_usd_saved(&self) -> f64 {
        // Blended average: $2.50 per 1M tokens (Claude/GPT-4o blend)
        (self.total_token_savings() as f64 / 1_000_000.0) * 2.50
    }
}

pub struct Persistence;

impl Persistence {
    fn get_db_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".vision-squeezer");
        let _ = std::fs::create_dir_all(&path);
        path.push("stats.db");
        path
    }

    pub fn init_db() -> Result<(), String> {
        let conn = Connection::open(Self::get_db_path()).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS optimizations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                model TEXT NOT NULL,
                original_tokens INTEGER NOT NULL,
                optimized_tokens INTEGER NOT NULL,
                original_bytes INTEGER NOT NULL,
                optimized_bytes INTEGER NOT NULL,
                mode TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn log_optimization(
        model: &str,
        orig_tokens: u32,
        opt_tokens: u32,
        orig_bytes: u64,
        opt_bytes: u64,
        mode: &str,
    ) -> Result<(), String> {
        let conn = Connection::open(Self::get_db_path()).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO optimizations (timestamp, model, original_tokens, optimized_tokens, original_bytes, optimized_bytes, mode)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                Utc::now().to_rfc3339(),
                model,
                orig_tokens,
                opt_tokens,
                orig_bytes as i64,
                opt_bytes as i64,
                mode,
            ],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_stats() -> Result<SqueezerStats, String> {
        let conn = Connection::open(Self::get_db_path()).map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT 
                COUNT(*), 
                SUM(original_tokens), 
                SUM(optimized_tokens), 
                SUM(original_bytes), 
                SUM(optimized_bytes) 
             FROM optimizations",
            )
            .map_err(|e| e.to_string())?;

        let (count, orig_t, opt_t, orig_b, opt_b) = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?.unwrap_or(0) as u64,
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0) as u64,
                    row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
                    row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
                    row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare(
            "SELECT timestamp, model, original_tokens, optimized_tokens, original_bytes, optimized_bytes, mode 
             FROM optimizations ORDER BY timestamp DESC LIMIT 50"
        ).map_err(|e| e.to_string())?;

        let history = stmt
            .query_map([], |row| {
                Ok(OptimizationReport {
                    timestamp: row.get(0)?,
                    model: row.get(1)?,
                    original_tokens: row.get(2)?,
                    optimized_tokens: row.get(3)?,
                    original_bytes: row.get::<_, i64>(4)? as u64,
                    optimized_bytes: row.get::<_, i64>(5)? as u64,
                    mode: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(SqueezerStats {
            total_optimizations: count,
            total_original_tokens: orig_t,
            total_optimized_tokens: opt_t,
            total_original_bytes: orig_b,
            total_optimized_bytes: opt_b,
            history,
        })
    }
}

// tools_list() is one large json! literal; the default macro recursion limit (128) is too small for it.
#![recursion_limit = "256"]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use vision_squeezer::{OutputFormat, ProcessConfig, ProcessMode, VisionModel, optimize_image};

/// Token budget applied when the caller does not pass `max_tokens` (≈ Claude's former 1568-token tier).
const DEFAULT_MAX_TOKENS: u32 = 1600;

// ── JSON-RPC types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Serialize)]
struct Response {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl Response {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }
    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ── Tool definitions ──────────────────────────────────────────────────────────

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "optimize_image",
                "description": "Resize and optimize an image for LLM vision APIs, fitting it to a token budget (default 1600). Removes padding, snaps dimensions to the model grid and re-encodes. Returns the optimized image plus a short JSON report; with image_path the copy is also written to output_path. To save tokens, look at the returned image instead of the original.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "image_path": {
                            "type": "string",
                            "description": "Path to a local image file (JPEG/PNG/WebP/GIF). Preferred: the file is read locally and the optimized copy is returned as an image plus written to a temp file (see output_path). Use this or image_base64."
                        },
                        "image_base64": {
                            "type": "string",
                            "description": "Base64-encoded image (JPEG/PNG/WebP). Data-URL prefix accepted. Use this or image_path."
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["standard", "ocr", "auto"],
                            "default": "auto",
                            "description": "auto = standard (colour preserved); standard = general vision; ocr = Otsu-threshold black and white, text only."
                        },
                        "output_format": {
                            "type": "string",
                            "enum": ["jpeg", "webp", "avif"],
                            "default": "jpeg",
                            "description": "Output encoding. WebP is typically 30-50% smaller than JPEG at equal quality; AVIF is typically another 20-50% smaller than WebP."
                        },
                        "quality": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 75,
                            "description": "JPEG output quality (1-100)."
                        },
                        "tile_size": {
                            "type": "integer",
                            "default": 512,
                            "description": "Custom patch size in pixels; ignored when target_model is set."
                        },
                        "crop": {
                            "type": "boolean",
                            "default": true,
                            "description": "Remove solid-color padding borders before resizing."
                        },
                        "bg_tolerance": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 255,
                            "default": 15,
                            "description": "Channel delta threshold for background detection (0 = exact match, 255 = everything)."
                        },
                        "max_tiles": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Hard cap on maximum tile count. Image will be progressively downscaled until it fits within this budget."
                        },
                        "max_tokens": {
                            "type": "integer",
                            "minimum": 0,
                            "default": 1600,
                            "description": "Token budget for the output image, measured with target_model (Claude when unset). The image is downscaled until it fits. 0 disables the cap."
                        },
                        "target_model": {
                            "type": "string",
                            "enum": ["claude", "claude-standard", "gpt6", "gpt4o", "gpt5", "gemini", "llama", "qwen", "deepseek", "deepseek-local", "kimi", "glm", "pixtral", "gemma", "internvl", "minicpm", "molmo", "aya", "phi4", "granite", "llava", "falcon", "minimax", "step", "ling", "voyage"],
                            "description": "Target model family for specialized dimension snapping."
                        }
                    }
                }
            },
            {
                "name": "optimize_image_batch",
                "description": "Optimize multiple images in one call (max 64). Each entry takes the same arguments as optimize_image. Returns a per-image results array; a failing image yields an error entry without failing the whole batch.",
                "inputSchema": {
                    "type": "object",
                    "required": ["images"],
                    "properties": {
                        "images": {
                            "type": "array",
                            "maxItems": 64,
                            "description": "Array of optimize_image argument objects.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "image_path": { "type": "string", "description": "Path to a local image file." },
                                    "image_base64": { "type": "string", "description": "Base64-encoded image (JPEG/PNG/WebP). Data-URL prefix accepted." },
                                    "mode": { "type": "string", "enum": ["standard", "ocr", "auto"], "default": "auto" },
                                    "output_format": { "type": "string", "enum": ["jpeg", "webp", "avif"], "default": "jpeg" },
                                    "quality": { "type": "integer", "minimum": 1, "maximum": 100, "default": 75 },
                                    "tile_size": { "type": "integer", "default": 512 },
                                    "crop": { "type": "boolean", "default": true },
                                    "bg_tolerance": { "type": "integer", "minimum": 0, "maximum": 255, "default": 15 },
                                    "max_tiles": { "type": "integer", "minimum": 1 },
                                    "max_tokens": { "type": "integer", "minimum": 0, "default": 1600 },
                                    "target_model": { "type": "string", "enum": ["claude", "claude-standard", "gpt6", "gpt4o", "gpt5", "gemini", "llama", "qwen", "deepseek", "deepseek-local", "kimi", "glm", "pixtral", "gemma", "internvl", "minicpm", "molmo", "aya", "phi4", "granite", "llava", "falcon", "minimax", "step", "ling", "voyage"] }
                                }
                            }
                        }
                    }
                }
            },
            {
                "name": "get_savings_stats",
                "description": "Retrieve cumulative token and bandwidth savings achieved through VisionSqueezer optimizations.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "sandbox_execute",
                "description": "Think in Code: Execute a sequence of atomic operations (crop, grayscale, binarize, resize, contrast, brightness) on an image to extract only necessary context and slash tokens.",
                "inputSchema": {
                    "type": "object",
                    "required": ["image_base64", "operations"],
                    "properties": {
                        "image_base64": {
                            "type": "string",
                            "description": "Base64-encoded image."
                        },
                        "operations": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["op"],
                                "properties": {
                                    "op": { "type": "string", "enum": ["crop", "grayscale", "binarize", "resize", "contrast", "brightness"] },
                                    "x": { "type": "integer" },
                                    "y": { "type": "integer" },
                                    "width": { "type": "integer" },
                                    "height": { "type": "integer" },
                                    "threshold": { "type": "integer" },
                                    "amount": { "type": "number" }
                                }
                            }
                        }
                    }
                }
            }
        ]
    })
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

fn handle_sandbox_execute(id: Value, args: Value) -> Response {
    use vision_squeezer::{
        ImageOp, decode_base64_image, encode_image_base64, process_with_operations,
    };

    let b64 = match args.get("image_base64").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Response::err(id, -32602, "missing image_base64"),
    };

    let ops: Vec<ImageOp> =
        match serde_json::from_value(args.get("operations").cloned().unwrap_or(json!([]))) {
            Ok(o) => o,
            Err(e) => return Response::err(id, -32602, format!("invalid operations: {}", e)),
        };

    let img = match decode_base64_image(b64) {
        Ok(i) => i,
        Err(e) => return Response::err(id, -32000, e),
    };

    let processed = process_with_operations(img, ops);

    // Use standard config for final output encoding
    let cfg = ProcessConfig::default();

    match encode_image_base64(&processed, &cfg) {
        Ok(encoded) => Response::ok(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": json!({
                        "optimized_base64": encoded,
                        "width": processed.width(),
                        "height": processed.height(),
                        "info": "Sandbox execution complete. Snap-to-tile will be applied if you use optimize_image next, or send as-is for minimal footprint."
                    }).to_string()
                }]
            }),
        ),
        Err(e) => Response::err(id, -32000, e),
    }
}

fn stats_text() -> Result<String, String> {
    let stats =
        vision_squeezer::Persistence::get_stats().map_err(|e| format!("Database error: {e}"))?;
    Ok(format!(
        "VisionSqueezer Analytics Report:\n\
        - Total Optimizations: {}\n\
        - Total Tokens Saved:  {}\n\
        - Total Bytes Saved:   {:.2} MB\n\
        - Estimated USD Saved: ${:.2}",
        stats.total_optimizations,
        stats.total_token_savings(),
        stats.total_byte_savings() as f64 / 1_048_576.0,
        stats.estimated_usd_saved()
    ))
}

fn handle_get_stats(id: Value) -> Response {
    match stats_text() {
        Ok(text) => Response::ok(id, json!({ "content": [{ "type": "text", "text": text }] })),
        Err(e) => Response::err(id, -32000, e),
    }
}

fn image_block(o: &Optimized) -> Value {
    json!({ "type": "image", "data": o.base64, "mimeType": o.mime })
}

fn handle_optimize_image(id: Value, args: Value) -> Response {
    match optimize_one(&args) {
        Ok(o) => Response::ok(
            id,
            json!({ "content": [
                image_block(&o),
                { "type": "text", "text": serde_json::to_string(&o.report).unwrap() }
            ] }),
        ),
        Err(e) => Response::err(id, -32000, e),
    }
}

fn handle_optimize_image_batch(id: Value, args: Value) -> Response {
    let images = match args.get("images").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Response::err(id, -32602, "missing images array"),
    };
    const MAX_BATCH: usize = 64; // ponytail: hard cap, raise if real workloads need it
    if images.len() > MAX_BATCH {
        return Response::err(id, -32602, format!("batch exceeds {MAX_BATCH} images"));
    }
    // One bad image returns an error entry; the batch as a whole still succeeds.
    // Images follow the text block in input order, skipping failed entries.
    let mut blocks: Vec<Value> = Vec::new();
    let results: Vec<Value> = images
        .iter()
        .enumerate()
        .map(|(idx, item)| match optimize_one(item) {
            Ok(o) => {
                blocks.push(image_block(&o));
                json!({ "index": idx, "ok": true, "result": o.report })
            }
            Err(e) => json!({ "index": idx, "ok": false, "error": e }),
        })
        .collect();
    let mut content = vec![json!({
        "type": "text",
        "text": serde_json::to_string(&json!({ "results": results })).unwrap()
    })];
    content.extend(blocks);
    Response::ok(id, json!({ "content": content }))
}

/// Input files larger than this are refused before being read into memory.
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

/// One optimized image: the report to show the model, plus the bytes to hand
/// back as an MCP `image` block. The base64 must never go into a text block:
/// the model would read it as hundreds of thousands of text tokens.
struct Optimized {
    report: Value,
    base64: String,
    mime: &'static str,
}

/// Where an optimized copy of `input` is written: a stable name under the OS
/// temp dir, so re-optimizing the same image reuses one file.
fn output_path_for(input: &Path, optimized_b64: &str, ext: &str) -> PathBuf {
    let mut h = DefaultHasher::new();
    optimized_b64.hash(&mut h);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    std::env::temp_dir()
        .join("vision-squeezer")
        .join(format!("{stem}-{:08x}.{ext}", h.finish() as u32))
}

/// Optimize a single image from an arguments object (same shape as the
/// `optimize_image` tool input): `image_path` (a local file) or `image_base64`.
// ponytail: batch loops this sequentially — parallelize only if latency is measured as a problem.
fn optimize_one(args: &Value) -> Result<Optimized, String> {
    let input_path = args
        .get("image_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    let owned_b64;
    let b64 = if let Some(path) = &input_path {
        let len = std::fs::metadata(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?
            .len();
        if len > MAX_INPUT_BYTES {
            return Err(format!(
                "{} is larger than {} MB",
                path.display(),
                MAX_INPUT_BYTES >> 20
            ));
        }
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        owned_b64 = B64.encode(bytes);
        owned_b64.as_str()
    } else {
        match args.get("image_base64").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return Err("missing image_path or image_base64".to_string()),
        }
    };
    let mode = match args.get("mode").and_then(|v| v.as_str()).unwrap_or("auto") {
        "ocr" => ProcessMode::Ocr,
        "standard" => ProcessMode::Standard,
        _ => ProcessMode::Auto,
    };
    let out_fmt = match args
        .get("output_format")
        .and_then(|v| v.as_str())
        .unwrap_or("jpeg")
    {
        "webp" => OutputFormat::WebP,
        "avif" => OutputFormat::Avif,
        _ => OutputFormat::Jpeg,
    };
    let mut cfg_builder = ProcessConfig::builder()
        .quality(
            args.get("quality")
                .and_then(|v| v.as_u64())
                .map(|q| q as u8)
                .unwrap_or(75),
        )
        .tile_size(
            args.get("tile_size")
                .and_then(|v| v.as_u64())
                .map(|t| t as u32)
                .unwrap_or(512),
        )
        .crop(args.get("crop").and_then(|v| v.as_bool()).unwrap_or(true))
        .bg_tolerance(
            args.get("bg_tolerance")
                .and_then(|v| v.as_u64())
                .map(|t| t as u8)
                .unwrap_or(15),
        )
        .output_format(out_fmt);

    if let Some(max_t) = args.get("max_tiles").and_then(|v| v.as_u64()) {
        cfg_builder = cfg_builder.max_tiles(max_t as u32);
    }
    cfg_builder = cfg_builder.max_tokens(
        args.get("max_tokens")
            .and_then(|v| v.as_u64())
            .map_or(DEFAULT_MAX_TOKENS, |t| t as u32),
    );
    if let Some(model_str) = args.get("target_model").and_then(|v| v.as_str())
        && let Some(model) = VisionModel::parse(model_str)
    {
        cfg_builder = cfg_builder.target_model(model);
    }
    let cfg = cfg_builder.build();

    match optimize_image(b64, mode, &cfg) {
        Ok(r) => {
            // Log to DB for Analytics
            let model_name = cfg
                .target_model
                .map_or("Agnostic", VisionModel::display_name);

            let m_enum = cfg.target_model.unwrap_or(VisionModel::Claude);
            let orig_tokens =
                vision_squeezer::estimate_tokens(r.original_width, r.original_height, m_enum)
                    .tokens;
            let opt_tokens = vision_squeezer::estimate_tokens(r.width, r.height, m_enum).tokens;

            let _ = vision_squeezer::Persistence::log_optimization(
                model_name,
                orig_tokens,
                opt_tokens,
                r.report.bytes_before.unwrap_or(0),
                r.optimized_bytes as u64,
                &format!("{:?}", mode),
            );

            let (ext, mime) = match out_fmt {
                OutputFormat::WebP => ("webp", "image/webp"),
                OutputFormat::Avif => ("avif", "image/avif"),
                _ => ("jpg", "image/jpeg"),
            };
            let output_path = match &input_path {
                Some(input) => {
                    let out = output_path_for(input, &r.optimized_base64, ext);
                    let bytes = B64.decode(&r.optimized_base64).map_err(|e| e.to_string())?;
                    if let Some(dir) = out.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    }
                    std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
                    Some(out.display().to_string())
                }
                None => None,
            };

            Ok(Optimized {
                base64: r.optimized_base64,
                mime,
                report: json!({
                "width": r.width,
                "height": r.height,
                "output_path": output_path,
                "tokens_before": orig_tokens,
                "tokens_after": opt_tokens,
                "savings_report": {
                    "tiles_before": r.report.tiles_before,
                    "tiles_after": r.report.tiles_after,
                    "tiles_saved": r.report.tiles_saved,
                    "token_reduction_pct": format!(
                        "{:.1}",
                        r.report.tiles_saved as f64 / r.report.tiles_before as f64 * 100.0
                    ),
                    "size_reduction_pct": r.report.size_reduction_pct()
                        .map(|p| format!("{:.1}", p))
                }
                }),
            })
        }
        Err(e) => Err(e),
    }
}

fn handle(req: Request) -> Option<Response> {
    // JSON-RPC notifications (no `id` field) must not receive a response.
    let id = req.id?;

    let resp = match req.method.as_str() {
        "initialize" => Response::ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "vision-squeezer", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),

        "tools/list" => Response::ok(id, tools_list()),

        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
            match tool_name {
                "optimize_image" => handle_optimize_image(id, args),
                "optimize_image_batch" => handle_optimize_image_batch(id, args),
                "get_savings_stats" => handle_get_stats(id),
                "sandbox_execute" => handle_sandbox_execute(id, args),
                _ => Response::err(id, -32601, format!("Tool not found: {}", tool_name)),
            }
        }

        _ => Response::err(id, -32601, format!("method not found: {}", req.method)),
    };
    Some(resp)
}

// ── Main loop (stdio JSON-RPC) ────────────────────────────────────────────────

fn print_setup() {
    let bin = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/path/to/vision-squeezer-mcp".to_string());

    println!("# VisionSqueezer MCP Setup");
    println!("# Binary: {bin}");
    println!();
    println!("## Claude Desktop  (~/.config/claude/claude_desktop_config.json)");
    println!();
    println!("{{");
    println!("  \"mcpServers\": {{");
    println!("    \"vision-squeezer\": {{");
    println!("      \"command\": \"{bin}\"");
    println!("    }}");
    println!("  }}");
    println!("}}");
    println!();
    println!("## Cursor / VS Code  (.cursor/mcp.json or .vscode/mcp.json)");
    println!();
    println!("{{");
    println!("  \"servers\": {{");
    println!("    \"vision-squeezer\": {{");
    println!("      \"type\": \"stdio\",");
    println!("      \"command\": \"{bin}\"");
    println!("    }}");
    println!("  }}");
    println!("}}");
    println!();
    println!("## Windsurf  (~/.codeium/windsurf/mcp_config.json)");
    println!();
    println!("{{");
    println!("  \"mcpServers\": {{");
    println!("    \"vision-squeezer\": {{");
    println!("      \"command\": \"{bin}\"");
    println!("    }}");
    println!("  }}");
    println!("}}");
}

/// `vision-squeezer-mcp optimize <image> [--max-tokens N] [--model M]`: one-shot
/// mode for hooks and scripts. Prints the JSON report (with output_path) to stdout.
fn run_optimize_cli(args: &[String]) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("usage: vision-squeezer-mcp optimize <image> [--max-tokens N] [--model M]");
        return 2;
    };
    let mut req = json!({ "image_path": path });
    let mut it = args[1..].iter();
    while let Some(flag) = it.next() {
        match (flag.as_str(), it.next()) {
            ("--max-tokens", Some(v)) if v.parse::<u64>().is_ok() => {
                req["max_tokens"] = json!(v.parse::<u64>().unwrap())
            }
            ("--model", Some(v)) => req["target_model"] = json!(v),
            _ => {
                eprintln!("unexpected argument: {flag}");
                return 2;
            }
        }
    }
    match optimize_one(&req) {
        Ok(o) => {
            println!("{}", o.report);
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if matches!(args.get(1).map(String::as_str), Some("--version" | "-V")) {
        println!("vision-squeezer-mcp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let _ = vision_squeezer::Persistence::init_db();
    if args.get(1).map(String::as_str) == Some("optimize") {
        std::process::exit(run_optimize_cli(&args[2..]));
    }
    // `npx vision-squeezer stats` for people without the Cargo CLI (used by /vision-stats).
    if args.get(1).map(String::as_str) == Some("stats") {
        match stats_text() {
            Ok(t) => println!("{t}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--setup" || a == "--help") {
        print_setup();
        return;
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(_) => break,
        };

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(req) => handle(req),
            Err(e) => {
                // If the message can't even be parsed as a Request, fall back to
                // inspecting it as raw JSON: notifications (no `id`) get no reply.
                match serde_json::from_str::<Value>(&line) {
                    Ok(v) if v.get("id").is_none() => None,
                    _ => Some(Response::err(
                        json!(null),
                        -32700,
                        format!("parse error: {e}"),
                    )),
                }
            }
        };

        if let Some(response) = response
            && let Ok(json) = serde_json::to_string(&response)
        {
            writeln!(out, "{json}").ok();
            out.flush().ok();
        }
    }
}

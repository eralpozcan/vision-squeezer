use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use vision_squeezer::{OutputFormat, ProcessConfig, ProcessMode, VisionModel, optimize_image};

// ── JSON-RPC types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Request {
    id: Value,
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
                "description": "Resize and optimize an image for LLM vision APIs. Snaps dimensions to tile boundaries, removes padding, and re-encodes to minimize token consumption.",
                "inputSchema": {
                    "type": "object",
                    "required": ["image_base64"],
                    "properties": {
                        "image_base64": {
                            "type": "string",
                            "description": "Base64-encoded image (JPEG/PNG/WebP). Data-URL prefix accepted."
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["standard", "ocr", "auto"],
                            "default": "auto",
                            "description": "auto = detect from color variance; standard = general vision; ocr = Otsu-threshold grayscale for text."
                        },
                        "output_format": {
                            "type": "string",
                            "enum": ["jpeg", "webp"],
                            "default": "jpeg",
                            "description": "Output encoding. WebP is typically 30-50% smaller than JPEG at equal quality."
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
                            "description": "Model patch size in pixels. 512 for Claude/GPT, 256 for Gemini."
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
                        "target_model": {
                            "type": "string",
                            "enum": ["claude", "gpt4o", "gpt5", "gemini"],
                            "description": "Target model family for specialized dimension snapping."
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

fn handle_get_stats(id: Value) -> Response {
    match vision_squeezer::Persistence::get_stats() {
        Ok(stats) => {
            let result = json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "VisionSqueezer Analytics Report:\n\
                        - Total Optimizations: {}\n\
                        - Total Tokens Saved:  {}\n\
                        - Total Bytes Saved:   {:.2} MB\n\
                        - Estimated USD Saved: ${:.2}",
                        stats.total_optimizations,
                        stats.total_token_savings(),
                        stats.total_byte_savings() as f64 / 1_048_576.0,
                        stats.estimated_usd_saved()
                    )
                }]
            });
            Response::ok(id, result)
        }
        Err(e) => Response::err(id, -32000, format!("Database error: {}", e)),
    }
}

fn handle_optimize_image(id: Value, args: Value) -> Response {
    let b64 = match args.get("image_base64").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Response::err(id, -32602, "missing image_base64"),
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
    if let Some(model_str) = args.get("target_model").and_then(|v| v.as_str()) {
        let model = match model_str {
            "gpt4o" | "gpt-4o" => VisionModel::Gpt4o,
            "gpt5" | "gpt-5" => VisionModel::Gpt5,
            "gemini" => VisionModel::Gemini15,
            _ => VisionModel::Claude,
        };
        cfg_builder = cfg_builder.target_model(model);
    }
    let cfg = cfg_builder.build();

    match optimize_image(b64, mode, &cfg) {
        Ok(r) => {
            // Log to DB for Analytics
            let model_name = match cfg.target_model {
                Some(VisionModel::Claude) => "Claude",
                Some(VisionModel::Gpt4o) => "GPT-4o",
                Some(VisionModel::Gpt5) => "GPT-5",
                Some(VisionModel::Gemini15) => "Gemini",
                None => "Agnostic",
            };

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

            Response::ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&json!({
                            "optimized_base64": r.optimized_base64,
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
                        })).unwrap()
                    }]
                }),
            )
        }
        Err(e) => Response::err(id, -32000, e),
    }
}

fn handle(req: Request) -> Response {
    match req.method.as_str() {
        "initialize" => Response::ok(
            req.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "vision-squeezer", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),

        "notifications/initialized" => Response::ok(req.id, json!({})),

        "tools/list" => Response::ok(req.id, tools_list()),

        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
            match tool_name {
                "optimize_image" => handle_optimize_image(req.id, args),
                "get_savings_stats" => handle_get_stats(req.id),
                "sandbox_execute" => handle_sandbox_execute(req.id, args),
                _ => Response::err(req.id, -32601, format!("Tool not found: {}", tool_name)),
            }
        }

        _ => Response::err(req.id, -32601, format!("method not found: {}", req.method)),
    }
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

fn main() {
    let _ = vision_squeezer::Persistence::init_db();

    let args: Vec<String> = std::env::args().collect();
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
            Err(e) => Response::err(json!(null), -32700, format!("parse error: {e}")),
        };

        if let Ok(json) = serde_json::to_string(&response) {
            writeln!(out, "{json}").ok();
            out.flush().ok();
        }
    }
}

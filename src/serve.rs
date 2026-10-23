//! `vision-squeezer serve`: a small HTTP API for workflow tools (n8n, Make, Zapier).
//!
//! POST /optimize with the raw image as the body. Options go in the query string:
//! model, max_tokens (default 1600, 0 disables), format, quality, mode, crop, response.
//! The response is the optimized image with the report in `X-*` headers, or the
//! report plus base64 image as JSON with `response=json`. GET /health returns "ok".

use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use vision_squeezer::{
    OutputFormat, ProcessConfig, ProcessMode, VisionModel, encode_to_bytes, estimate_tokens,
    process,
};

/// Same default budget as the MCP server.
const DEFAULT_MAX_TOKENS: u32 = 1600;
/// Request bodies larger than this are rejected before decoding.
const MAX_BODY_BYTES: u64 = 25 * 1024 * 1024;

pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

fn text(status: u16, msg: &str) -> Reply {
    Reply {
        status,
        content_type: "text/plain; charset=utf-8",
        headers: Vec::new(),
        body: format!("{msg}\n").into_bytes(),
    }
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
}

/// Handles one request. Kept free of HTTP-library types so it can be unit tested.
pub fn handle(method: &str, url: &str, body: &[u8]) -> Reply {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    match (method, path) {
        ("GET", "/health") => text(200, "ok"),
        ("POST", "/optimize") => optimize(query, body),
        (_, "/optimize") => text(405, "use POST /optimize with the image as the body"),
        _ => text(404, "not found: try POST /optimize or GET /health"),
    }
}

fn optimize(query: &str, body: &[u8]) -> Reply {
    let q = |k| query_param(query, k);

    let model = match q("model") {
        Some(m) => match VisionModel::parse(m) {
            Some(model) => Some(model),
            None => return text(400, &format!("unknown model: {m}")),
        },
        None => None,
    };
    let max_tokens = match q("max_tokens").map(str::parse::<u32>) {
        Some(Ok(t)) => t,
        Some(Err(_)) => return text(400, "max_tokens must be a non-negative integer"),
        None => DEFAULT_MAX_TOKENS,
    };
    let (fmt, ext, mime) = match q("format").unwrap_or("jpeg") {
        "jpeg" | "jpg" => (OutputFormat::Jpeg, "jpg", "image/jpeg"),
        "webp" => (OutputFormat::WebP, "webp", "image/webp"),
        "avif" => (OutputFormat::Avif, "avif", "image/avif"),
        f => return text(400, &format!("unknown format: {f} (jpeg, webp, avif)")),
    };
    let mode = match q("mode").unwrap_or("auto") {
        "auto" => ProcessMode::Auto,
        "ocr" => ProcessMode::Ocr,
        "standard" => ProcessMode::Standard,
        m => return text(400, &format!("unknown mode: {m} (auto, ocr, standard)")),
    };
    let json = match q("response").unwrap_or("image") {
        "image" => false,
        "json" => true,
        r => return text(400, &format!("unknown response: {r} (image, json)")),
    };

    let mut cfg = ProcessConfig::builder()
        .output_format(fmt)
        .max_tokens(max_tokens)
        .crop(q("crop") != Some("false"));
    if let Some(quality) = q("quality") {
        match quality.parse() {
            Ok(v) => cfg = cfg.quality(v),
            Err(_) => return text(400, "quality must be an integer 1-100"),
        }
    }
    if let Some(m) = model {
        cfg = cfg.target_model(m);
    }
    let cfg = cfg.build();

    if body.is_empty() {
        return text(400, "empty body: send the image bytes as the request body");
    }
    let img = match image::load_from_memory(body) {
        Ok(img) => img,
        Err(e) => return text(400, &format!("could not decode image: {e}")),
    };
    let (in_w, in_h) = (img.width(), img.height());

    let result = process(img, mode, body.len() as u64, &cfg);
    let bytes = match encode_to_bytes(&result.image, &cfg) {
        Ok(b) => b,
        Err(e) => return text(500, &format!("encode failed: {e}")),
    };

    let token_model = cfg.target_model.unwrap_or(VisionModel::Claude);
    let tokens_before = estimate_tokens(in_w, in_h, token_model).tokens;
    let tokens_after = estimate_tokens(result.width, result.height, token_model).tokens;

    if json {
        let report = serde_json::json!({
            "model": token_model.display_name(),
            "format": ext,
            "mime_type": mime,
            "input_width": in_w,
            "input_height": in_h,
            "output_width": result.width,
            "output_height": result.height,
            "input_bytes": body.len(),
            "output_bytes": bytes.len(),
            "tokens_before": tokens_before,
            "tokens_after": tokens_after,
            "tokens_saved": tokens_before.saturating_sub(tokens_after),
            "base64": B64.encode(&bytes),
        });
        return Reply {
            status: 200,
            content_type: "application/json",
            headers: Vec::new(),
            body: report.to_string().into_bytes(),
        };
    }

    Reply {
        status: 200,
        content_type: mime,
        headers: vec![
            ("X-Tokens-Before", tokens_before.to_string()),
            ("X-Tokens-After", tokens_after.to_string()),
            ("X-Input-Size", format!("{in_w}x{in_h}")),
            (
                "X-Output-Size",
                format!("{}x{}", result.width, result.height),
            ),
            ("X-Model", token_model.display_name().to_string()),
            (
                "Content-Disposition",
                format!("inline; filename=\"optimized.{ext}\""),
            ),
        ],
        body: bytes,
    }
}

/// Runs the server until the process is killed.
pub fn run(args: &[String]) {
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 8080;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                i += 1;
                if let Some(h) = args.get(i) {
                    host = h.clone();
                }
            }
            "--port" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse().ok()) {
                    Some(p) => port = p,
                    None => {
                        eprintln!("--port needs a number");
                        std::process::exit(2);
                    }
                }
            }
            other => {
                eprintln!("unknown serve option: {other} (use --host, --port)");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let addr = format!("{host}:{port}");
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not listen on {addr}: {e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "vision-squeezer {} listening on http://{addr}",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("  POST /optimize?model=claude&max_tokens=1600   (body: image bytes)");

    for mut req in server.incoming_requests() {
        let reply = if req.body_length().is_some_and(|n| n as u64 > MAX_BODY_BYTES) {
            text(413, "image too large (limit 25 MB)")
        } else {
            let mut body = Vec::new();
            match req
                .as_reader()
                .take(MAX_BODY_BYTES + 1)
                .read_to_end(&mut body)
            {
                Ok(_) if body.len() as u64 > MAX_BODY_BYTES => {
                    text(413, "image too large (limit 25 MB)")
                }
                Ok(_) => handle(req.method().as_str(), req.url(), &body),
                Err(e) => text(400, &format!("could not read body: {e}")),
            }
        };

        let mut resp = tiny_http::Response::from_data(reply.body)
            .with_status_code(reply.status)
            .with_header(
                tiny_http::Header::from_bytes("Content-Type", reply.content_type).unwrap(),
            );
        for (k, v) in reply.headers {
            if let Ok(h) = tiny_http::Header::from_bytes(k, v) {
                resp = resp.with_header(h);
            }
        }
        let _ = req.respond(resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::io::Cursor;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbImage::from_fn(w, h, |x, y| Rgb([(x % 251) as u8, (y % 241) as u8, 90]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn header<'a>(r: &'a Reply, name: &str) -> &'a str {
        r.headers
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.as_str())
            .unwrap()
    }

    #[test]
    fn health_is_ok() {
        assert_eq!(handle("GET", "/health", &[]).status, 200);
    }

    #[test]
    fn unknown_path_and_method() {
        assert_eq!(handle("GET", "/nope", &[]).status, 404);
        assert_eq!(handle("GET", "/optimize", &[]).status, 405);
    }

    #[test]
    fn returns_image_with_default_budget() {
        let r = handle("POST", "/optimize", &png(2400, 1670));
        assert_eq!(r.status, 200);
        assert_eq!(r.content_type, "image/jpeg");
        assert!(image::load_from_memory(&r.body).is_ok());
        let after: u32 = header(&r, "X-Tokens-After").parse().unwrap();
        let before: u32 = header(&r, "X-Tokens-Before").parse().unwrap();
        assert!(after <= DEFAULT_MAX_TOKENS, "{after} over budget");
        assert!(after < before);
    }

    #[test]
    fn json_response_carries_report_and_image() {
        let r = handle(
            "POST",
            "/optimize?max_tokens=600&format=webp&response=json",
            &png(1600, 1200),
        );
        assert_eq!(r.status, 200);
        let v: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert!(v["tokens_after"].as_u64().unwrap() <= 600);
        assert_eq!(v["mime_type"], "image/webp");
        let img = B64.decode(v["base64"].as_str().unwrap()).unwrap();
        assert_eq!(img.len() as u64, v["output_bytes"].as_u64().unwrap());
    }

    #[test]
    fn zero_budget_disables_cap() {
        let capped = handle("POST", "/optimize", &png(2400, 1670));
        let open = handle("POST", "/optimize?max_tokens=0", &png(2400, 1670));
        let a: u32 = header(&capped, "X-Tokens-After").parse().unwrap();
        let b: u32 = header(&open, "X-Tokens-After").parse().unwrap();
        assert!(b > a);
    }

    #[test]
    fn bad_input_is_400() {
        assert_eq!(handle("POST", "/optimize", &[]).status, 400);
        assert_eq!(handle("POST", "/optimize", b"not an image").status, 400);
        assert_eq!(
            handle("POST", "/optimize?model=nope", &png(10, 10)).status,
            400
        );
        assert_eq!(
            handle("POST", "/optimize?format=gif", &png(10, 10)).status,
            400
        );
        assert_eq!(
            handle("POST", "/optimize?max_tokens=-1", &png(10, 10)).status,
            400
        );
    }
}

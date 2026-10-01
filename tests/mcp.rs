use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use assert_cmd::cargo::CommandCargoExt;
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use image::{Rgba, RgbaImage};

fn make_image_base64(w: u32, h: u32) -> String {
    let img = RgbaImage::from_pixel(w, h, Rgba([200, 200, 200, 255]));
    let mut buf: Vec<u8> = Vec::new();
    let dyn_img = image::DynamicImage::ImageRgba8(img);
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    B64.encode(&buf)
}

#[test]
fn mcp_tools_list_includes_optimize_image() {
    let mut child = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list"}}"#).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    let v: serde_json::Value = serde_json::from_str(&line).expect("json parse");
    assert_eq!(v["id"], serde_json::json!(1));
    let tools = v["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"optimize_image"));
    assert!(names.contains(&"optimize_image_batch"));
    assert!(names.contains(&"get_savings_stats"));
    assert!(names.contains(&"sandbox_execute"));

    let _ = child.kill();
}

#[test]
fn mcp_optimize_image_returns_image_block_and_report() {
    let mut child = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");

    let stdin = child.stdin.as_mut().unwrap();
    let b64 = make_image_base64(1025, 1025);

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "optimize_image",
            "arguments": {
                "image_base64": b64,
                "target_model": "claude"
            }
        }
    });
    writeln!(stdin, "{}", req).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    let v: serde_json::Value = serde_json::from_str(&line).expect("json parse");
    assert_eq!(v["id"], serde_json::json!(42));
    // The image goes back as an MCP image block; the text block is a small report
    // and must not carry the base64 (the model would read it as text tokens).
    let image = &v["result"]["content"][0];
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/jpeg");
    assert!(image["data"].as_str().unwrap().len() > 0);
    let text = v["result"]["content"][1]["text"].as_str().unwrap();
    assert!(!text.contains("base64"));
    let inner: serde_json::Value = serde_json::from_str(text).expect("inner json");
    assert!(inner["savings_report"]["tiles_before"].is_u64());
    assert!(inner["savings_report"]["tiles_after"].is_u64());

    let _ = child.kill();
}

#[test]
fn mcp_optimize_image_batch_processes_each_entry() {
    let mut child = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");

    let stdin = child.stdin.as_mut().unwrap();
    let good = make_image_base64(1025, 1025);

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "optimize_image_batch",
            "arguments": {
                "images": [
                    { "image_base64": good, "target_model": "claude" },
                    { "image_base64": "!!!not base64!!!" }
                ]
            }
        }
    });
    writeln!(stdin, "{}", req).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    let v: serde_json::Value = serde_json::from_str(&line).expect("json parse");
    assert_eq!(v["id"], serde_json::json!(7));
    // The batch itself succeeds even though one image is invalid.
    assert!(v["error"].is_null());
    let content = &v["result"]["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content.as_str().unwrap()).expect("inner json");
    let results = inner["results"].as_array().expect("results array");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["ok"], serde_json::json!(true));
    assert!(results[0]["result"]["savings_report"].is_object());
    assert_eq!(results[1]["ok"], serde_json::json!(false));
    assert!(results[1]["error"].is_string());
    // One image block for the one successful entry, after the text block.
    let content = v["result"]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[1]["type"], "image");

    let _ = child.kill();
}

#[test]
fn mcp_optimize_image_accepts_a_file_path_and_writes_a_copy() {
    use base64::Engine;
    let dir = std::env::temp_dir().join(format!("vs-mcp-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.png");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(make_image_base64(1025, 1025))
        .unwrap();
    std::fs::write(&input, bytes).unwrap();

    let mut child = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 9, "method": "tools/call",
        "params": { "name": "optimize_image", "arguments": { "image_path": input } }
    });
    writeln!(child.stdin.as_mut().unwrap(), "{}", req).unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&line).expect("json parse");
    assert_eq!(v["result"]["content"][0]["type"], "image");
    let report: serde_json::Value =
        serde_json::from_str(v["result"]["content"][1]["text"].as_str().unwrap()).unwrap();
    let out = report["output_path"].as_str().expect("output_path");
    assert!(std::path::Path::new(out).exists(), "{out} was not written");
    assert!(report["tokens_after"].as_u64() <= report["tokens_before"].as_u64());

    let _ = child.kill();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mcp_optimize_image_rejects_a_missing_file() {
    let mut child = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 10, "method": "tools/call",
        "params": { "name": "optimize_image", "arguments": { "image_path": "/definitely/not/here.png" } }
    });
    writeln!(child.stdin.as_mut().unwrap(), "{}", req).unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&line).expect("json parse");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cannot read")
    );
    let _ = child.kill();
}

#[test]
fn stats_subcommand_prints_the_report_without_the_cargo_cli() {
    let home = std::env::temp_dir().join(format!("vs-stats-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    let out = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .arg("stats")
        .env("HOME", &home)
        .output()
        .expect("run stats");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("VisionSqueezer Analytics Report"), "{text}");
    assert!(text.contains("Total Optimizations: 0"), "{text}");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn mcp_unknown_method_returns_error() {
    let mut child = Command::cargo_bin("vision-squeezer-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":99,"method":"nope"}}"#).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    let v: serde_json::Value = serde_json::from_str(&line).expect("json parse");
    assert_eq!(v["error"]["code"], serde_json::json!(-32601));

    let _ = child.kill();
}

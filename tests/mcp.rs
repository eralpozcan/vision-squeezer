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
    assert!(names.contains(&"get_savings_stats"));
    assert!(names.contains(&"sandbox_execute"));

    let _ = child.kill();
}

#[test]
fn mcp_optimize_image_returns_base64_and_report() {
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
    let content = &v["result"]["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content.as_str().unwrap()).expect("inner json");
    assert!(inner["optimized_base64"].as_str().unwrap().len() > 0);
    assert!(inner["savings_report"]["tiles_before"].is_u64());
    assert!(inner["savings_report"]["tiles_after"].is_u64());

    let _ = child.kill();
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

use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;
use image::{Rgba, RgbaImage};
use tempfile::tempdir;

fn make_fixture(path: &std::path::Path, w: u32, h: u32) {
    // White background with a black square center — easy crop + non-tile-aligned dims
    let mut img = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    let cw = w / 4;
    let ch = h / 4;
    for x in (w / 2 - cw / 2)..(w / 2 + cw / 2) {
        for y in (h / 2 - ch / 2)..(h / 2 + ch / 2) {
            img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
    img.save(path).expect("fixture save");
}

#[test]
fn cli_version_flag() {
    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn cli_optimizes_image_to_default_output() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.png");
    make_fixture(&input, 1025, 1025);

    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .arg(&input)
        .assert()
        .success();

    let expected = dir.path().join("in.optimized.jpg");
    assert!(expected.exists(), "expected default output to exist");
    let meta = fs::metadata(&expected).unwrap();
    assert!(meta.len() > 0);
}

#[test]
fn cli_dry_run_does_not_write_file() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.png");
    make_fixture(&input, 1025, 1025);
    let out = dir.path().join("out.jpg");

    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--dry-run",
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!out.exists(), "dry-run should not write file");
}

#[test]
fn cli_json_flag_emits_parsable_json() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.png");
    make_fixture(&input, 1025, 1025);

    let output = Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([input.to_str().unwrap(), "--json", "--dry-run"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json parse");
    assert_eq!(v["dry_run"], serde_json::json!(true));
    assert_eq!(v["input_width"], serde_json::json!(1025));
    assert_eq!(v["input_height"], serde_json::json!(1025));
    assert!(v["tokens"]["before"].as_u64().unwrap() > 0);
    assert!(v["token_savings_table"]["claude"]["before"].is_u64());
}

#[test]
fn cli_batch_recursive_processes_subdirs() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    make_fixture(&dir.path().join("a.png"), 600, 400);
    make_fixture(&sub.join("b.png"), 800, 600);

    let out_dir = tempdir().unwrap();

    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([
            dir.path().to_str().unwrap(),
            "--recursive",
            "--output-dir",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out_dir.path().join("a.optimized.jpg").exists());
    assert!(out_dir.path().join("sub").join("b.optimized.jpg").exists());
}

#[test]
fn cli_avif_format_writes_avif_file() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.png");
    make_fixture(&input, 1025, 1025);
    let out = dir.path().join("out.avif");

    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--format",
            "avif",
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    let bytes = fs::read(&out).expect("avif file");
    assert!(bytes.len() > 12);
    // AVIF starts with a "ftyp" box; second 4 bytes after size are b"ftyp"
    assert_eq!(&bytes[4..8], b"ftyp", "expected AVIF ftyp box header");
}

#[test]
fn cli_smart_crop_flag_runs() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.png");
    make_fixture(&input, 600, 400);

    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([input.to_str().unwrap(), "--smart-crop", "--dry-run"])
        .assert()
        .success();
}

#[test]
fn cli_auto_quality_picks_value_in_range_and_reports_it() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.png");
    make_fixture(&input, 1025, 1025);

    let output = Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--auto-quality",
            "0.95",
            "--json",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json parse");
    let q = v["quality"].as_u64().unwrap();
    assert!((40..=95).contains(&q), "auto quality {} out of range", q);
    assert_eq!(v["auto_quality_target"], serde_json::json!(0.95));
}

#[test]
fn cli_stats_csv_emits_valid_header() {
    let output = Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args(["stats", "--csv"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let first_line = stdout.lines().next().expect("csv header");
    assert_eq!(
        first_line,
        "timestamp,model,original_tokens,optimized_tokens,token_savings,original_bytes,optimized_bytes,byte_savings,mode"
    );
}

#[test]
fn cli_stats_csv_output_writes_file() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("history.csv");

    Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args(["stats", "--csv-output", csv_path.to_str().unwrap()])
        .assert()
        .success();

    assert!(csv_path.exists(), "csv file should be written");
    let contents = fs::read_to_string(&csv_path).unwrap();
    assert!(contents.starts_with("timestamp,model,"));
}

#[test]
fn cli_batch_json_aggregates_files() {
    let dir = tempdir().unwrap();
    make_fixture(&dir.path().join("a.png"), 600, 400);
    make_fixture(&dir.path().join("b.png"), 800, 600);

    let output = Command::cargo_bin("vision-squeezer")
        .unwrap()
        .args([dir.path().to_str().unwrap(), "--json", "--dry-run"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json parse");
    assert_eq!(v["count"], serde_json::json!(2));
    assert_eq!(v["recursive"], serde_json::json!(false));
    assert_eq!(v["files"].as_array().unwrap().len(), 2);
    assert!(v["totals"]["input_bytes"].as_u64().unwrap() > 0);
}

use std::fs;
use std::path::{Path, PathBuf};

use vision_squeezer::{
    ImageOp, OutputFormat, ProcessConfig, ProcessMode, VisionModel, encode_to_bytes, process,
    token_savings_table,
};

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif"];

fn print_usage() {
    eprintln!("Usage: vision-squeezer <image> [options]");
    eprintln!("       vision-squeezer stats          (show cumulative savings)");
    eprintln!("       vision-squeezer stats --csv    (export full history as CSV to stdout)");
    eprintln!("       vision-squeezer stats --csv-output <path>  (write CSV to file)");
    eprintln!("       vision-squeezer /vision-stats  (alias for stats)");
    eprintln!("       vision-squeezer setup-hook    (print shell integration script)");
    eprintln!("\nOptions:");
    eprintln!("  --mode ocr|standard|auto  (default: auto)");
    eprintln!("  --format jpeg|webp|avif    (default: jpeg)");
    eprintln!("  --quality 1-100            (default: 75)");
    eprintln!("  --tile-size N              (default: 512)");
    eprintln!("  --no-crop");
    eprintln!("  --bg-tolerance N           (default: 15)");
    eprintln!("  --model claude|gpt4o|gpt5|gemini  model-aware resizing");
    eprintln!("  --max-tiles N              (limit maximum token tiles)");
    eprintln!("  --output, -o <path>        (custom output path)");
    eprintln!(
        "  --json                     (machine-readable JSON output, suppresses human table)"
    );
    eprintln!("  --dry-run                  (run pipeline, skip disk write, skip stats logging)");
    eprintln!("  --recursive, -r            (batch mode: walk subdirs of input directory)");
    eprintln!("  --output-dir <path>        (batch mode: mirror tree into this directory)");
    eprintln!("  --smart-crop               (edge-energy crop instead of corner-tolerance)");
    eprintln!("  --auto-quality <0..1>      (binary-search quality to hit SSIM target, e.g. 0.95)");
    eprintln!("  --ops 'JSON'               (Think in Code: list of atomic operations)");
    eprintln!(
        "                             ex: --ops '[{{\"op\":\"crop\",\"x\":0,\"y\":0,\"width\":100,\"height\":100}},{{\"op\":\"grayscale\"}}]'"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Initialize DB
    let _ = vision_squeezer::Persistence::init_db();

    if matches!(
        args.get(1).map(|s| s.as_str()),
        Some("--version") | Some("-V") | Some("version")
    ) {
        println!("vision-squeezer {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if matches!(
        args.get(1).map(|s| s.as_str()),
        Some("stats") | Some("/vision-stats")
    ) {
        let csv_mode = args.iter().any(|a| a == "--csv");
        let csv_output = args
            .iter()
            .position(|a| a == "--csv-output")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from);

        if csv_mode || csv_output.is_some() {
            export_stats_csv(csv_output);
        } else {
            print_stats();
        }
        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("setup-hook") {
        print_hook_script();
        return;
    }

    if args.len() < 2 {
        print_usage();
        return;
    }

    let path = PathBuf::from(&args[1]);

    // Parse flags
    let mut cfg = ProcessConfig::builder();
    let mut mode = ProcessMode::Auto;
    let mut fmt = OutputFormat::Jpeg;
    let mut custom_output: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut ops: Vec<ImageOp> = Vec::new();
    let mut json_output = false;
    let mut dry_run = false;
    let mut recursive = false;
    let mut auto_quality: Option<f64> = None;
    let mut i = 2usize;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    custom_output = Some(PathBuf::from(p));
                }
            }
            "--mode" => {
                i += 1;
                match args.get(i).map(|s| s.as_str()) {
                    Some("ocr") => mode = ProcessMode::Ocr,
                    Some("standard") => mode = ProcessMode::Standard,
                    _ => mode = ProcessMode::Auto,
                }
            }
            "--format" => {
                i += 1;
                fmt = match args.get(i).map(|s| s.as_str()) {
                    Some("webp") => OutputFormat::WebP,
                    Some("avif") => OutputFormat::Avif,
                    _ => OutputFormat::Jpeg,
                };
            }
            "--quality" => {
                i += 1;
                if let Some(q) = args.get(i).and_then(|s| s.parse().ok()) {
                    cfg = cfg.quality(q);
                }
            }
            "--tile-size" => {
                i += 1;
                if let Some(t) = args.get(i).and_then(|s| s.parse().ok()) {
                    cfg = cfg.tile_size(t);
                }
            }
            "--max-tiles" => {
                i += 1;
                if let Some(m) = args.get(i).and_then(|s| s.parse().ok()) {
                    cfg = cfg.max_tiles(m);
                }
            }
            "--no-crop" => {
                cfg = cfg.crop(false);
            }
            "--bg-tolerance" => {
                i += 1;
                if let Some(t) = args.get(i).and_then(|s| s.parse().ok()) {
                    cfg = cfg.bg_tolerance(t);
                }
            }
            "--model" => {
                i += 1;
                let m = match args.get(i).map(|s| s.as_str()) {
                    Some("gpt4o") | Some("gpt-4o") => Some(VisionModel::Gpt4o),
                    Some("gpt5") | Some("gpt-5") | Some("gpt5.5") => Some(VisionModel::Gpt5),
                    Some("gemini") => Some(VisionModel::Gemini15),
                    Some("llama") | Some("llama-vision") => Some(VisionModel::LlamaVision),
                    Some("qwen") | Some("qwen-vl") => Some(VisionModel::QwenVl),
                    Some("deepseek") | Some("deepseek-vl") => Some(VisionModel::DeepseekVl),
                    _ => Some(VisionModel::Claude),
                };
                if let Some(model) = m {
                    cfg = cfg.target_model(model);
                }
            }
            "--ops" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    let parsed: Vec<vision_squeezer::ImageOp> =
                        serde_json::from_str(s).expect("failed to parse --ops JSON");
                    ops.extend(parsed);
                }
            }
            "--json" => json_output = true,
            "--dry-run" => dry_run = true,
            "--recursive" | "-r" => recursive = true,
            "--output-dir" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    output_dir = Some(PathBuf::from(p));
                }
            }
            "--smart-crop" => {
                cfg = cfg.smart_crop(true);
            }
            "--auto-quality" => {
                i += 1;
                if let Some(t) = args.get(i).and_then(|s| s.parse().ok()) {
                    auto_quality = Some(t);
                }
            }
            _ => {}
        }
        i += 1;
    }
    let cfg = cfg.output_format(fmt).build();

    let opts = RunOpts {
        cfg: &cfg,
        mode,
        custom_output,
        output_dir,
        ops,
        json_output,
        dry_run,
        quiet: false,
        auto_quality,
    };

    if path.is_dir() {
        run_batch(&path, recursive, &opts);
    } else {
        let _ = run_one(&path, &opts);
    }
}

struct RunOpts<'a> {
    cfg: &'a ProcessConfig,
    mode: ProcessMode,
    custom_output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    ops: Vec<ImageOp>,
    json_output: bool,
    dry_run: bool,
    quiet: bool,
    auto_quality: Option<f64>,
}

struct FileOutcome {
    json: serde_json::Value,
    output_bytes: u64,
    input_bytes: u64,
    tiles_before: u32,
    tiles_after: u32,
    tokens_before: u32,
    tokens_after: u32,
}

fn run_one(path: &Path, opts: &RunOpts) -> Option<FileOutcome> {
    let cfg = opts.cfg;
    let mode = opts.mode;

    let input_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let img = match image::open(path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("[skip] {}: {}", path.display(), e);
            return None;
        }
    };
    let (orig_w, orig_h) = (img.width(), img.height());

    let want_human = !opts.json_output && !opts.quiet;

    if want_human {
        println!(
            "Input:  {}  {}×{}  ({:.1} MB)",
            path.display(),
            orig_w,
            orig_h,
            input_bytes as f64 / 1_048_576.0
        );
    }

    let img = if !opts.ops.is_empty() {
        if want_human {
            println!("Sandbox: Applying {} operations...", opts.ops.len());
        }
        vision_squeezer::process_with_operations(img, opts.ops.clone())
    } else {
        img
    };

    let mut result = process(img, mode, input_bytes, cfg);

    let ext = match cfg.output_format {
        OutputFormat::WebP => "webp",
        OutputFormat::Avif => "avif",
        OutputFormat::Jpeg => "jpg",
    };
    let out_path = resolve_output_path(path, opts, ext);
    let (bytes, used_quality) = if let Some(target) = opts.auto_quality {
        vision_squeezer::encode_with_auto_quality(&result.image, cfg, target, 40, 95)
            .expect("auto-quality encode failed")
    } else {
        (
            encode_to_bytes(&result.image, cfg).expect("encode failed"),
            cfg.quality,
        )
    };
    let output_bytes = bytes.len() as u64;
    result.report.bytes_after = Some(output_bytes);

    if !opts.dry_run {
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&out_path, &bytes).expect("write failed");
    }

    let target_model_name = match cfg.target_model {
        Some(VisionModel::Claude) => "Claude",
        Some(VisionModel::Gpt4o) => "GPT-4o",
        Some(VisionModel::Gpt5) => "GPT-5",
        Some(VisionModel::Gemini15) => "Gemini",
        Some(VisionModel::LlamaVision) => "Llama Vision",
        Some(VisionModel::QwenVl) => "Qwen-VL",
        Some(VisionModel::DeepseekVl) => "DeepSeek-VL",
        None => "Agnostic",
    };

    let m = cfg.target_model.unwrap_or(VisionModel::Claude);
    let orig_tokens = vision_squeezer::estimate_tokens(orig_w, orig_h, m).tokens;
    let opt_tokens = vision_squeezer::estimate_tokens(result.width, result.height, m).tokens;

    let table = token_savings_table(orig_w, orig_h, result.width, result.height);

    let json = serde_json::json!({
        "input_path": path.display().to_string(),
        "output_path": if opts.dry_run { serde_json::Value::Null } else { serde_json::Value::String(out_path.display().to_string()) },
        "input_width": orig_w,
        "input_height": orig_h,
        "output_width": result.width,
        "output_height": result.height,
        "input_bytes": input_bytes,
        "output_bytes": output_bytes,
        "format": ext,
        "quality": used_quality,
        "auto_quality_target": opts.auto_quality,
        "smart_crop": cfg.smart_crop,
        "mode": format!("{:?}", mode),
        "model": target_model_name,
        "dry_run": opts.dry_run,
        "tokens": {
            "before": orig_tokens,
            "after": opt_tokens,
            "saved": orig_tokens.saturating_sub(opt_tokens)
        },
        "tiles": {
            "before": result.report.tiles_before,
            "after": result.report.tiles_after,
            "saved": result.report.tiles_saved
        },
        "size_reduction_pct": result.report.size_reduction_pct(),
        "token_savings_table": {
            "claude": { "before": table.claude_before.tokens, "after": table.claude_after.tokens },
            "gpt4o":  { "before": table.gpt4o_before.tokens,  "after": table.gpt4o_after.tokens },
            "gpt5":   { "before": table.gpt5_before.tokens,   "after": table.gpt5_after.tokens },
            "gemini": { "before": table.gemini_before.tokens, "after": table.gemini_after.tokens }
        }
    });

    if opts.json_output && !opts.quiet {
        // single-file JSON; batch caller will aggregate instead
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if want_human {
        println!(
            "Output: {}×{}  ({:.1} MB, {} q{}{}){}",
            result.width,
            result.height,
            output_bytes as f64 / 1_048_576.0,
            ext.to_uppercase(),
            used_quality,
            if opts.auto_quality.is_some() {
                " auto"
            } else {
                ""
            },
            if opts.dry_run {
                "  [DRY-RUN — not written]"
            } else {
                ""
            },
        );

        if let Some(pct) = result.report.size_reduction_pct() {
            println!("File:   {:.1}% smaller", pct);
        }

        println!();
        println!("── Token Estimates ─────────────────────────────────────────");
        table.print();
        println!("────────────────────────────────────────────────────────────");
        if opts.dry_run {
            println!("→ (dry-run, no file written)");
        } else {
            println!("→ {}", out_path.display());
        }
    }

    if !opts.dry_run {
        let _ = vision_squeezer::Persistence::log_optimization(
            target_model_name,
            orig_tokens,
            opt_tokens,
            input_bytes,
            output_bytes,
            &format!("{:?}", mode),
        );
    }

    Some(FileOutcome {
        json,
        output_bytes,
        input_bytes,
        tiles_before: result.report.tiles_before,
        tiles_after: result.report.tiles_after,
        tokens_before: orig_tokens,
        tokens_after: opt_tokens,
    })
}

fn resolve_output_path(input: &Path, opts: &RunOpts, ext: &str) -> PathBuf {
    if let Some(custom) = &opts.custom_output {
        return custom.clone();
    }
    if let Some(out_dir) = &opts.output_dir {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        return out_dir.join(format!("{stem}.optimized.{ext}"));
    }
    input.with_extension(format!("optimized.{ext}"))
}

fn run_batch(root: &Path, recursive: bool, opts: &RunOpts) {
    let files = collect_images(root, recursive);
    if files.is_empty() {
        eprintln!("No image files found under {}", root.display());
        return;
    }

    if !opts.json_output {
        println!(
            "Batch: {} image(s) under {}{}",
            files.len(),
            root.display(),
            if recursive { " (recursive)" } else { "" }
        );
        println!();
    }

    let mut entries: Vec<serde_json::Value> = Vec::with_capacity(files.len());
    let mut totals = BatchTotals::default();

    for f in &files {
        let per_opts = RunOpts {
            cfg: opts.cfg,
            mode: opts.mode,
            custom_output: None,
            output_dir: opts.output_dir.clone().map(|d| mirror_dir(&d, root, f)),
            ops: opts.ops.clone(),
            json_output: false,
            dry_run: opts.dry_run,
            quiet: opts.json_output,
            auto_quality: opts.auto_quality,
        };
        if let Some(out) = run_one(f, &per_opts) {
            totals.add(&out);
            entries.push(out.json);
        }
    }

    if opts.json_output {
        let summary = serde_json::json!({
            "root": root.display().to_string(),
            "recursive": recursive,
            "count": entries.len(),
            "totals": {
                "input_bytes": totals.input_bytes,
                "output_bytes": totals.output_bytes,
                "bytes_saved": totals.input_bytes.saturating_sub(totals.output_bytes),
                "tiles_before": totals.tiles_before,
                "tiles_after": totals.tiles_after,
                "tokens_before": totals.tokens_before,
                "tokens_after": totals.tokens_after,
                "tokens_saved": totals.tokens_before.saturating_sub(totals.tokens_after)
            },
            "files": entries
        });
        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    } else {
        println!();
        println!("── Batch Summary ───────────────────────────────────────────");
        println!("Files:           {}", entries.len());
        println!(
            "Total bytes:     {:.2} MB → {:.2} MB  ({:.1}% saved)",
            totals.input_bytes as f64 / 1_048_576.0,
            totals.output_bytes as f64 / 1_048_576.0,
            if totals.input_bytes > 0 {
                (1.0 - totals.output_bytes as f64 / totals.input_bytes as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "Total tokens:    {} → {}  (saved {})",
            totals.tokens_before,
            totals.tokens_after,
            totals.tokens_before.saturating_sub(totals.tokens_after)
        );
        println!("────────────────────────────────────────────────────────────");
    }
}

#[derive(Default)]
struct BatchTotals {
    input_bytes: u64,
    output_bytes: u64,
    tiles_before: u64,
    tiles_after: u64,
    tokens_before: u64,
    tokens_after: u64,
}

impl BatchTotals {
    fn add(&mut self, o: &FileOutcome) {
        self.input_bytes += o.input_bytes;
        self.output_bytes += o.output_bytes;
        self.tiles_before += o.tiles_before as u64;
        self.tiles_after += o.tiles_after as u64;
        self.tokens_before += o.tokens_before as u64;
        self.tokens_after += o.tokens_after as u64;
    }
}

fn mirror_dir(out_root: &Path, in_root: &Path, file: &Path) -> PathBuf {
    let rel = file.parent().and_then(|p| p.strip_prefix(in_root).ok());
    match rel {
        Some(r) => out_root.join(r),
        None => out_root.to_path_buf(),
    }
}

fn collect_images(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let _ = collect_into(root, recursive, &mut out);
    out.sort();
    out
}

fn collect_into(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                let _ = collect_into(&path, recursive, out);
            }
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        // skip already-optimized outputs
        let stem_lc = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if stem_lc.ends_with(".optimized") {
            continue;
        }
        if IMAGE_EXTS.iter().any(|e| e == &ext.as_str()) {
            out.push(path);
        }
    }
    Ok(())
}

fn print_hook_script() {
    println!(
        r#"
# VisionSqueezer Shell Hook
# Add this to your .zshrc or .bashrc:
#   eval "$(vision-squeezer setup-hook)"

# The 'squeeze' command: optimizes an image and returns the new path
squeeze() {{
    if [ -z "$1" ]; then
        echo "Usage: squeeze <file> [options]"
        return 1
    fi
    local input="$1"
    local output="${{input%.*}}.squeezed.${{input##*.}}"
    vision-squeezer "$input" --output "$output" "${{@:2}}" > /dev/null
    if [ -f "$output" ]; then
        echo "$output"
    else
        echo "Error: Optimization failed"
        return 1
    fi
}}

# Aliases for quick analytics
alias vision-stats='vision-squeezer stats'
alias /vision-stats='vision-squeezer stats'

# Install /vision-stats Claude Code skill (zero-overhead stats — no MCP round-trip)
_vs_install_skill() {{
    local skill_dir="$HOME/.claude/skills/vision-stats"
    local skill_file="$skill_dir/SKILL.md"
    local bin
    bin="$(command -v vision-squeezer 2>/dev/null || echo 'vision-squeezer')"
    if [ ! -f "$skill_file" ]; then
        mkdir -p "$skill_dir"
        cat > "$skill_file" << 'SKILL_EOF'
---
name: vision-stats
description: >
  Show VisionSqueezer cumulative token & byte savings analytics. Zero MCP
  overhead — reads directly from local stats.db via CLI binary. Use when user
  says "vision-stats", "squeeze stats", "token savings", "how much saved",
  "vision-squeezer stats", "optimization history", or "/vision-stats".
allowed-tools: Bash
---

# vision-stats — VisionSqueezer Analytics Skill

Zero-overhead stats. Calls `vision-squeezer stats` directly — no MCP round-trip.

## Trigger

`/vision-stats` or any of: "vision stats", "squeeze stats", "show savings", "how much have I saved", "optimization stats"

## Action

Run this binary resolution chain, stop at first success:

```bash
vision-squeezer stats 2>/dev/null \
  || ~/.cargo/bin/vision-squeezer stats 2>/dev/null \
  || "$(dirname "$(command -v vision-squeezer-mcp 2>/dev/null)")/vision-squeezer" stats 2>/dev/null \
  || find "$HOME/.cargo/bin" "$HOME/Desktop" "$HOME/Projects" -maxdepth 6 -name "vision-squeezer" -not -path "*/deps/*" -not -path "*/debug/*" 2>/dev/null | head -1 | xargs -I{{}} {{}} stats 2>/dev/null \
  || echo "vision-squeezer not found. Install: cargo install --git https://github.com/eralpozcan/vision-squeezer"
```

Print output verbatim. No wrapping, no commentary, no interpretation.

## Error handling

Binary not found → tell user to run `cargo install --path .` from project root or `eval "$(vision-squeezer setup-hook)"` after install.

## Notes

- Stats persist in local stats.db on the user's machine
- MCP tool `get_savings_stats` does the same but costs ~150 tokens overhead — use this skill instead
SKILL_EOF
        echo "[vision-squeezer] /vision-stats skill installed → $skill_file"
    fi
}}
_vs_install_skill
unset -f _vs_install_skill

# Install /vision-doctor Claude Code skill (version check + update guidance)
_vs_install_doctor_skill() {{
    local skill_dir="$HOME/.claude/skills/vision-doctor"
    local skill_file="$skill_dir/SKILL.md"
    if [ ! -f "$skill_file" ]; then
        mkdir -p "$skill_dir"
        cat > "$skill_file" << 'SKILL_EOF'
---
name: vision-doctor
description: >
  Check VisionSqueezer installation health and version status. Detects installed
  version, compares against latest npm release, and shows update command if outdated.
  Use when user says "vision-doctor", "check vision-squeezer version", "update vision-squeezer",
  "is vision-squeezer up to date", "upgrade vision-squeezer", or "/vision-doctor".
allowed-tools: Bash
---

# vision-doctor — VisionSqueezer Health Check Skill

Checks binary installation, current version, and latest available version.

## Trigger

`/vision-doctor` or any of: "vision doctor", "check vision-squeezer", "update vision-squeezer",
"is vision-squeezer up to date", "upgrade vision-squeezer", "vision-squeezer version"

## Action

Run the following shell script:

```bash
BIN=$(command -v vision-squeezer 2>/dev/null)
if [ -z "$BIN" ] && [ -x "$HOME/.cargo/bin/vision-squeezer" ]; then
  BIN="$HOME/.cargo/bin/vision-squeezer"
fi
if [ -n "$BIN" ] && [ -x "$BIN" ]; then
  INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
else
  INSTALLED=""
  BIN=""
fi
LATEST=$(npm view vision-squeezer version 2>/dev/null)
MCP_CMD=$(claude mcp list 2>/dev/null | grep vision-squeezer | head -1 || echo "")
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "LATEST=$LATEST"
echo "MCP=$MCP_CMD"
```

## Output format

Display as a markdown checklist:

```
## VisionSqueezer Doctor

- [x/ ] Binary found: <path or "not found (using npx)">
- [x/ ] Installed version: <version or "n/a — npx always pulls latest">
- [x/ ] Latest version (npm): <version>
- [x/ ] MCP registered: <yes/no>
- [x/ ] Status: <see below>
```

### Status logic

| Condition | Status |
|-----------|--------|
| `INSTALLED` == `LATEST` | ✅ Up to date |
| `INSTALLED` != `LATEST`, both non-empty | ⚠️ Update available — run `/vision-upgrade` |
| `BIN` empty, `MCP` contains "npx" | ✅ Using npx — always latest, no action needed |
| `BIN` empty, no MCP | ❌ Not installed |

### If update available:

```
Update available: v<INSTALLED> → v<LATEST>
Run /vision-upgrade to update.
```

### If not installed:

```
## VisionSqueezer not found

Install via Claude Code (one-liner):
  claude mcp add vision-squeezer -- npx -y vision-squeezer
```

## Notes

- `npx -y vision-squeezer` users are always on latest — show this as ✅, not an error
- cargo install users must run `/vision-upgrade` or `cargo install vision-squeezer` to upgrade
SKILL_EOF
        echo "[vision-squeezer] /vision-doctor skill installed → $skill_file"
    fi
}}
_vs_install_doctor_skill
unset -f _vs_install_doctor_skill

# Install /vision-upgrade Claude Code skill (upgrade to latest)
_vs_install_upgrade_skill() {{
    local skill_dir="$HOME/.claude/skills/vision-upgrade"
    local skill_file="$skill_dir/SKILL.md"
    if [ ! -f "$skill_file" ]; then
        mkdir -p "$skill_dir"
        cat > "$skill_file" << 'SKILL_EOF'
---
name: vision-upgrade
description: >
  Upgrade VisionSqueezer to the latest version. Detects install method (cargo, npm global, npx)
  and runs the correct update command. Use when user says "vision-upgrade", "upgrade vision-squeezer",
  "update vision-squeezer", or "/vision-upgrade".
allowed-tools: Bash
---

# vision-upgrade — VisionSqueezer Upgrade Skill

Detects install method and upgrades to latest.

## Trigger

`/vision-upgrade` or any of: "vision upgrade", "upgrade vision-squeezer", "update vision-squeezer", "install latest vision-squeezer"

## Action

Run the following detection script first:

```bash
BIN=$(command -v vision-squeezer 2>/dev/null)
[ -z "$BIN" ] && [ -x "$HOME/.cargo/bin/vision-squeezer" ] && BIN="$HOME/.cargo/bin/vision-squeezer"
INSTALLED=""
[ -n "$BIN" ] && [ -x "$BIN" ] && INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
LATEST=$(npm view vision-squeezer version 2>/dev/null)
NPM_GLOBAL=$(npm list -g vision-squeezer --depth=0 2>/dev/null | grep vision-squeezer | head -1)
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "LATEST=$LATEST"
echo "NPM_GLOBAL=$NPM_GLOBAL"
```

### Then run the appropriate upgrade command:

**If `NPM_GLOBAL` non-empty** (npm global install):
```bash
npm install -g vision-squeezer
```

**If `BIN` contains `.cargo`** (cargo install):
```bash
cargo install vision-squeezer
```

**If `BIN` empty** (npx user):
No action needed — npx always pulls latest. Confirm to user.

### After upgrade, verify:
```bash
vision-squeezer --version 2>/dev/null || ~/.cargo/bin/vision-squeezer --version 2>/dev/null
```

## Output format

```
## VisionSqueezer Upgrade

- [ ] Detected install method: <cargo / npm global / npx>
- [ ] Version before: v<INSTALLED or "n/a">
- [ ] Running upgrade...
- [ ] Version after: v<NEW_VERSION>
- [ ] Status: ✅ Updated to v<LATEST> / ✅ Already on latest (npx)
```

## Notes

- npx users: always on latest, no upgrade needed — tell them explicitly
- If cargo install fails (no Rust): suggest switching to npx with `claude mcp add vision-squeezer -- npx -y vision-squeezer`
SKILL_EOF
        echo "[vision-squeezer] /vision-upgrade skill installed → $skill_file"
    fi
}}
_vs_install_upgrade_skill
unset -f _vs_install_upgrade_skill
"#
    );
}

fn print_stats() {
    match vision_squeezer::Persistence::get_stats() {
        Ok(stats) => {
            println!("\x1b[1m── VisionSqueezer Analytics ────────────────────────────────\x1b[0m");
            println!("Total Optimizations: {}", stats.total_optimizations);
            println!(
                "Total Tokens Saved:  \x1b[32m{}\x1b[0m",
                stats.total_token_savings()
            );
            println!(
                "Total Bytes Saved:   \x1b[32m{:.2} MB\x1b[0m",
                stats.total_byte_savings() as f64 / 1_048_576.0
            );
            println!(
                "Estimated USD Saved: \x1b[35m${:.2}\x1b[0m",
                stats.estimated_usd_saved()
            );
            println!("────────────────────────────────────────────────────────────");
            if !stats.history.is_empty() {
                println!("\x1b[2mLast 5 operations:\x1b[0m");
                for (i, op) in stats.history.iter().take(5).enumerate() {
                    let date = op.timestamp.split('T').next().unwrap_or("");
                    println!(
                        "{}. {} | {:8} | {} → {} tokens",
                        i + 1,
                        date,
                        op.model,
                        op.original_tokens,
                        op.optimized_tokens
                    );
                }
            }
        }
        Err(e) => eprintln!("Error retrieving stats: {}", e),
    }
}

fn export_stats_csv(output: Option<PathBuf>) {
    let history = match vision_squeezer::Persistence::get_all_history() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error retrieving history: {}", e);
            std::process::exit(1);
        }
    };

    let mut buf = String::new();
    buf.push_str("timestamp,model,original_tokens,optimized_tokens,token_savings,original_bytes,optimized_bytes,byte_savings,mode\n");
    for r in &history {
        let token_sav = r.original_tokens.saturating_sub(r.optimized_tokens);
        let byte_sav = r.original_bytes.saturating_sub(r.optimized_bytes);
        buf.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            r.timestamp,
            r.model,
            r.original_tokens,
            r.optimized_tokens,
            token_sav,
            r.original_bytes,
            r.optimized_bytes,
            byte_sav,
            r.mode,
        ));
    }

    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, &buf) {
                eprintln!("Error writing CSV to {}: {}", path.display(), e);
                std::process::exit(1);
            }
            eprintln!("Wrote {} rows to {}", history.len(), path.display());
        }
        None => print!("{}", buf),
    }
}

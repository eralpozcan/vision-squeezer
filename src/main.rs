use std::fs;
use std::path::PathBuf;

use vision_squeezer::{
    OutputFormat, ProcessConfig, ProcessMode, VisionModel, encode_to_bytes, process,
    token_savings_table,
};

fn print_usage() {
    eprintln!("Usage: vision-squeezer <image> [options]");
    eprintln!("       vision-squeezer stats          (show cumulative savings)");
    eprintln!("       vision-squeezer /vision-stats  (alias for stats)");
    eprintln!("       vision-squeezer setup-hook    (print shell integration script)");
    eprintln!("\nOptions:");
    eprintln!("  --mode ocr|standard|auto  (default: auto)");
    eprintln!("  --format jpeg|webp         (default: jpeg)");
    eprintln!("  --quality 1-100            (default: 75)");
    eprintln!("  --tile-size N              (default: 512)");
    eprintln!("  --no-crop");
    eprintln!("  --bg-tolerance N           (default: 15)");
    eprintln!("  --model claude|gpt4o|gpt5|gemini  model-aware resizing");
    eprintln!("  --max-tiles N              (limit maximum token tiles)");
    eprintln!("  --output, -o <path>        (custom output path)");
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
        Some("stats") | Some("/vision-stats")
    ) {
        print_stats();
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
    let input_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let img = image::open(&path).expect("failed to open image");
    let (orig_w, orig_h) = (img.width(), img.height());

    // Parse flags
    let mut cfg = ProcessConfig::builder();
    let mut mode = ProcessMode::Auto;
    let mut fmt = OutputFormat::Jpeg;
    let mut custom_output: Option<PathBuf> = None;
    let mut ops: Vec<vision_squeezer::ImageOp> = Vec::new();
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
                if args.get(i).map(|s| s.as_str()) == Some("webp") {
                    fmt = OutputFormat::WebP;
                }
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
            _ => {}
        }
        i += 1;
    }
    let cfg = cfg.output_format(fmt).build();

    println!(
        "Input:  {}×{}  ({:.1} MB)",
        orig_w,
        orig_h,
        input_bytes as f64 / 1_048_576.0
    );

    let img = if !ops.is_empty() {
        println!("Sandbox: Applying {} operations...", ops.len());
        vision_squeezer::process_with_operations(img, ops)
    } else {
        img
    };

    let mut result = process(img, mode, input_bytes, &cfg);

    // Encode
    let ext = match cfg.output_format {
        OutputFormat::WebP => "webp",
        OutputFormat::Jpeg => "jpg",
    };
    let out_path = custom_output.unwrap_or_else(|| path.with_extension(format!("optimized.{ext}")));
    let bytes = encode_to_bytes(&result.image, &cfg).expect("encode failed");
    let output_bytes = bytes.len() as u64;
    fs::write(&out_path, &bytes).expect("write failed");
    result.report.bytes_after = Some(output_bytes);

    println!(
        "Output: {}×{}  ({:.1} MB, {} q{})",
        result.width,
        result.height,
        output_bytes as f64 / 1_048_576.0,
        ext.to_uppercase(),
        cfg.quality,
    );

    if let Some(pct) = result.report.size_reduction_pct() {
        println!("File:   {:.1}% smaller", pct);
    }

    println!();
    println!("── Token Estimates ─────────────────────────────────────────");
    let table = token_savings_table(orig_w, orig_h, result.width, result.height);
    table.print();
    println!("────────────────────────────────────────────────────────────");
    println!("→ {}", out_path.display());

    // Log to DB for Analytics
    let target_model_name = match cfg.target_model {
        Some(VisionModel::Claude) => "Claude",
        Some(VisionModel::Gpt4o) => "GPT-4o",
        Some(VisionModel::Gpt5) => "GPT-5",
        Some(VisionModel::Gemini15) => "Gemini",
        None => "Agnostic",
    };

    let m = cfg.target_model.unwrap_or(VisionModel::Claude);
    let orig_tokens = vision_squeezer::estimate_tokens(orig_w, orig_h, m).tokens;
    let opt_tokens = vision_squeezer::estimate_tokens(result.width, result.height, m).tokens;

    let _ = vision_squeezer::Persistence::log_optimization(
        target_model_name,
        orig_tokens,
        opt_tokens,
        input_bytes,
        output_bytes,
        &format!("{:?}", mode),
    );
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
    
    # Run vision-squeezer and capture output path
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

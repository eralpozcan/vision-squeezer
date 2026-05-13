# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # compile
cargo run            # run
cargo test           # all tests
cargo test <name>    # single test by name
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

Rust 2024 edition. Three source files:

- `src/lib.rs` — core pipeline: semantic crop → tile-aware resize → OCR binarize, `ProcessConfig`, token estimation for Claude/GPT-4o/Gemini, WebP/JPEG output
- `src/main.rs` — CLI binary (`vision-squeezer`): file I/O, `--format` flag, token savings table
- `src/mcp_server.rs` — MCP binary (`vision-squeezer-mcp`): JSON-RPC over stdio, exposes `optimize_image` tool

## Key Types

- `ProcessConfig` — all tunable params (tile size, quality, crop tolerance, output format, provider)
- `VisionModel` — `Claude` / `Gpt4o` / `Gemini`, drives `estimate_tokens()`
- `OutputFormat` — `Jpeg` / `WebP`

## Notes

- Token savings are dimensional only — format (JPEG vs WebP) affects file size, not API tokens
- GPT-4o pre-fits to 2048px before tiling → may show 0% token savings for certain inputs (correct behavior)
- MCP server communicates via stdin/stdout; do not add logging to stdout

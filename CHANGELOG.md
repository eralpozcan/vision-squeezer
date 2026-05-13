# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [0.1.1] - 2026-05-13

### Added
- `mcpName` field in `package.json` for MCP Registry verification
- Published to [MCP Registry](https://registry.modelcontextprotocol.io) (`io.github.eralpozcan/vision-squeezer`)
- `server.json` for `mcp-publisher` CLI

### Fixed
- Cargo.toml and npm versions now in sync

---

## [0.1.0] - 2026-05-13

### Added
- **Three-stage optimization pipeline**: semantic crop → tile-aware resize → OCR binarize
- **Multi-provider token estimation**: Claude, GPT-4o, GPT-5, Gemini with provider-specific tile math
- **Output formats**: JPEG and WebP with configurable quality
- **Sandbox mode**: apply atomic ops (`crop`, `grayscale`, `binarize`, `resize`, `contrast`, `brightness`) locally before sending to LLM — CLI via `--ops`, MCP via `sandbox_execute` tool
- **MCP server** (`vision-squeezer-mcp`): stdio JSON-RPC with `optimize_image`, `sandbox_execute`, `get_savings_stats` tools
- **CLI** (`vision-squeezer`): `--model`, `--format`, `--ops`, `--max-tiles`, `--no-crop`, `--bg-tolerance`, `--quality` flags
- **Persistence & Analytics**: SQLite tracking of cumulative token/byte savings via `vision-squeezer stats`
- **Shell hook**: `eval "$(vision-squeezer setup-hook)"` for shell integration
- **ProcessConfig**: fully configurable pipeline parameters with builder API
- GitHub Actions CI (test + lint) and release workflows (multi-platform builds + crates.io auto-publish)
- GitHub community files: issue templates, FUNDING.yml, CODE_OF_CONDUCT.md, CONTRIBUTING.md
- Netlify deployment for docs site
- npm package (`npx -y vision-squeezer`) for zero-install MCP usage

### Notes
- GPT-4o shows 0% token savings for certain inputs — correct behavior due to 2048px pre-fitting step
- MCP server communicates via stdin/stdout; do not redirect stdout in shell environments

[0.1.1]: https://github.com/eralpozcan/vision-squeezer/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/eralpozcan/vision-squeezer/releases/tag/v0.1.0

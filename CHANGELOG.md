# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [0.2.2] - 2026-05-24

### Fixed
- **CI: manylinux bumped to 2_28** — Python wheel builds for `aarch64` were failing with `libwebp-sys` v0.9.6 due to GCC 4.8 in manylinux2014 lacking C99 mode and ARMv8.2 NEON element-indexed instruction support.
- **CI: crates.io re-publish tolerance** — Re-running a release tag after a partial CI failure no longer fails the job when the version is already published on crates.io.
- **CI: `cargo publish --token` deprecation** — Moved registry token to `CARGO_REGISTRY_TOKEN` environment variable.

---

## [0.2.1] - 2026-05-24

### Added
- **CSV export for stats**: `vision-squeezer stats --csv` emits full optimization history to stdout. `--csv-output <path>` writes to a file. Columns: `timestamp,model,original_tokens,optimized_tokens,token_savings,original_bytes,optimized_bytes,byte_savings,mode`.
- `Persistence::get_all_history()` — unbounded history retrieval (the existing `get_stats()` is still capped at the last 50 for the dashboard view).

### Fixed
- `cargo fmt --check` regressions across `benches/pipeline.rs`, `src/lib.rs`, `src/main.rs`, `tests/cli.rs`, `tests/mcp.rs`. CI lint job now green.

---

## [0.2.0] - 2026-05-24

### Added
- **`--json` flag**: emit a machine-readable JSON record per invocation (CLI + batch). Suppresses human table output for pipeline use.
- **`--dry-run` flag**: run the full pipeline (incl. token math) without writing to disk or updating the stats DB.
- **Batch / recursive mode**: pass a directory as input. `--recursive` walks subtrees, `--output-dir` mirrors structure. Combined with `--json` emits an aggregate record with per-file array + totals.
- **AVIF output**: `--format avif`. Typically 20–50% smaller than WebP at equal quality, ~3× smaller than JPEG. Token math unchanged (format ≠ tokens).
- **`--smart-crop` flag**: Sobel-lite gradient-energy crop. Finds the bounding box of high-edge-density regions instead of the corner-tolerance crop, which is more aggressive on photographic content while preserving the salient subject.
- **`--auto-quality <target>` flag**: binary-searches output quality in [40,95] to hit a given SSIM target (typically 0.95). Picks the smallest file that still passes a perceptual threshold.
- **Integration tests** (`tests/cli.rs`, `tests/mcp.rs`) and **criterion benches** (`benches/pipeline.rs`).
- **Python bindings**: new `python/` subcrate using pyo3 + maturin. `pip install vision-squeezer` (once published). Exposes `optimize_image`, `estimate_tokens`, `optimal_dimensions` with full feature parity including smart-crop and auto-quality. CI workflow auto-builds wheels for Linux/macOS/Windows on tag push.

### Public API
- `OutputFormat::Avif` variant
- `ProcessConfig::smart_crop` (builder: `.smart_crop(bool)`)
- `vision_squeezer::saliency_crop(&DynamicImage, margin: u32) -> DynamicImage`
- `vision_squeezer::ssim(&DynamicImage, &DynamicImage) -> f64`
- `vision_squeezer::encode_with_auto_quality(&DynamicImage, &ProcessConfig, target: f64, min_q: u8, max_q: u8) -> Result<(Vec<u8>, u8), String>`

### MCP
- `optimize_image.output_format` schema now accepts `"avif"` (additive — existing `"jpeg"` and `"webp"` unchanged).

### Changed
- CLI single-file flow extracted into `run_one`; new `run_batch` dispatcher for directories.

---

## [0.1.9] - 2026-05-15

### Fixed
- **Skills always updated on install**: `postinstall.js` now overwrites existing skill files — previously installed users were stuck on old versions

### Changed
- **README restructured**: Install section moved to top, CLI Usage follows immediately, case studies and deep-dive sections moved below

---

## [0.1.8] - 2026-05-15

### Added
- **`/vision-upgrade` Claude Code skill**: detects install method (cargo / npm global / npx) and runs the correct upgrade command automatically
- **`/vision-doctor` improved**: npx users now shown as ✅ instead of "unknown"; links to `/vision-upgrade` when update available; checks MCP registration status

### Fixed
- **`--version` flag**: binary now responds to `--version`, `-V`, and `version` — outputs `vision-squeezer X.Y.Z`. Previously unhandled, causing `/vision-doctor` to report "unknown" version.

---

## [0.1.7] - 2026-05-15

### Added
- **Auto-install skills on `npm install`**: `postinstall.js` now writes `/vision-stats` and `/vision-doctor` skills to `~/.claude/skills/` automatically — no `setup-hook` or manual step needed

---

## [0.1.6] - 2026-05-14

### Added
- **`/vision-doctor` Claude Code skill**: version check — compares installed binary vs latest npm release, shows update command per install method (cargo / npm / npx)
- **Favicons & PWA manifest** on docs site: full favicon stack (ico, 16×16, 32×32, apple-touch-icon), populated `site.webmanifest` with theme colors
- **`sitemap.xml` and `robots.txt`** for visionsqueezer.com

### Changed
- `setup-hook` now also writes `/vision-doctor` skill to `~/.claude/skills/` on first run
- README: merged separate shell hook + vision-stats sections into unified "Shell Hook & Claude Code Skills" section with skills table
- OG/Twitter image URLs made absolute; `og:url` added to docs site

### Infrastructure
- Added `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` — enables `/plugins add vision-stats@vision-squeezer` and `/plugins add vision-doctor@vision-squeezer` via Claude Code marketplace

---

## [0.1.5] - 2026-05-14

### Added
- **`/vision-stats` Claude Code skill**: zero-overhead analytics via direct CLI call — no MCP round-trip (~150 token saving per stats query)
- **Marketplace distribution**: skill lives at `skills/vision-stats/SKILL.md`; installable via `/plugins add vision-stats@vision-squeezer` or auto-installed by `setup-hook`
- **Cookie Consent banner** on docs site (GDPR-friendly, localStorage-based)
- **Umami analytics** on docs site — loaded only after user consent
- **Dynamic version badge** on docs site — fetched from GitHub Releases API at page load

### Changed
- `setup-hook` now writes `/vision-stats` skill to `~/.claude/skills/` on first run (idempotent)
- Shell hook binary resolution: 4-stage fallback chain (PATH → `~/.cargo/bin` → MCP sibling dir → `find`)
- Docs install selector: added "Claude Code Skill (/vision-stats)" option
- README: added `Claude Code Skill` section with marketplace and setup-hook install paths

---

## [0.1.4] - 2026-05-13

### Fixed
- `package.json` version synced to match `Cargo.toml` (0.1.4)

---

## [0.1.3] - 2026-05-13

### Fixed
- CI release workflow: fixed binary rename step using `dist/` to avoid same-file `mv` error

---

## [0.1.2] - 2026-05-13

### Fixed
- CI: opt into Node.js 24 via `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` for GitHub Actions compatibility

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

[0.1.5]: https://github.com/eralpozcan/vision-squeezer/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/eralpozcan/vision-squeezer/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/eralpozcan/vision-squeezer/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/eralpozcan/vision-squeezer/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/eralpozcan/vision-squeezer/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/eralpozcan/vision-squeezer/releases/tag/v0.1.0

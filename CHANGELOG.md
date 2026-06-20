# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [0.5.0] - 2026-06-20

### Added
- **`optimize_image_batch` MCP tool** — optimize up to 64 images in a single tool call. Each entry takes the same arguments as `optimize_image`; the response is a per-image `results` array. A failing image yields an `{ "ok": false, "error": ... }` entry without aborting the batch. Eliminates per-image round-trips when an agent processes galleries or document sets.

### Security
- **Bounded image decode (decompression-bomb / OOM guard).** `decode_base64_image` now caps base64 input length and decodes through `image::Limits` (max 16384px per dimension, 100 MP, RGBA allocation ceiling). A crafted, tiny-on-disk image declaring enormous dimensions can no longer exhaust memory on the MCP server. Covers both `optimize_image` and `sandbox_execute`, which share this decode path. (OWASP A03/A05.)

## [0.4.0] - 2026-06-11

### Added
- **Three new provider targets for token estimation and dimension snapping:** `llama`, `qwen`, `deepseek` (CLI `--model`, MCP `target_model`).
  - **Llama 3.2 / 3.3 Vision (Mllama)** — 560×560 tiles, aspect-ratio canvas capped at 4 tiles, ~1601 tokens/tile. Source: `transformers` `MllamaVisionConfig`. (Llama 4's native-multimodal vision encoder is a different scheme and is not modeled.)
  - **Qwen2-VL / 2.5-VL / 3-VL** — 28px effective grid (14px patch × 2×2 merge), `tokens = (W/28)·(H/28)` bounded to `[4, 16384]`. Source: `qwen_vl_utils.smart_resize`.
  - **DeepSeek-VL2** — SigLIP-SO400M-384 + 2× pixel-shuffle (14×14 = 196 tokens/tile), anyres `(m·384, n·384)` canvas with `m·n ≤ 9`; exact token layout `210 (global) + 1 (separator) + (nh·14)·(nw·14 + 1)`. Source: DeepSeek-VL2 paper §2 (arXiv:2412.10302, 13 Dec 2024) + reference processor. Open-weights; the win is local-inference context, not API billing.
- **Documentation site rebuilt** on Nuxt UI + Nuxt Content (multi-page, auto sitemap / llms.txt / OG images). New per-provider pages with exact formulas, cited primary sources, verification dates, and proportional savings tables.

### Notes
- Formula constants were verified against primary sources (model configs, reference tokenizer code, technical reports) on 2026-06-11. Hosted-API per-tile billing can differ from a model's own token footprint — treat absolute numbers as indicative and the tile/patch counts as authoritative.

---

## [0.3.5] - 2026-05-26

### Fixed
- **`postinstall.js` no longer overwrites skills with stale inline content.** Pre-0.3.5 versions hardcoded the SKILL.md text as JS string literals inside `installSkills()`, so every `npm install` / `npx -y vision-squeezer` clobbered `~/.claude/skills/vision-*/SKILL.md` with the v0.3.0-era wording — including the false "Status: ✅ Using npx — always latest, no action needed" line that masked broken installs across the entire 0.3.x debugging session. The function now reads from the bundled `plugins/vision-squeezer-mcp/skills/<name>/SKILL.md` files shipped in the npm tarball, so both install paths (npm + Claude Code plugin marketplace) share one on-disk source of truth.
- **`plugins/vision-squeezer-mcp/.mcp.json` is now version-pinned.** The plugin marketplace registration was `npx -y vision-squeezer` (unpinned), so users who installed via `/plugin install vision-squeezer-mcp@vision-squeezer` got the same npx cache-freeze bug the installer was already protecting against on its own path. Now reads `["-y", "vision-squeezer@<VERSION>"]`. The release invariants in `CLAUDE.md` require this file to be bumped in lockstep with `package.json`.
- **`vision-doctor` + `vision-upgrade` MCP probes now run with `cwd=$HOME`.** When the probe ran from the user's current shell cwd and the user happened to be inside the `vision-squeezer` project directory (whose `package.json` is `name: vision-squeezer`), `npx` detected the local package, skipped the install step, and the probe false-negatived with `sh: vision-squeezer: command not found`. Pinning the subprocess cwd to `$HOME` guarantees a neutral resolution context.

### Changed
- **`CLAUDE.md` release invariants expanded** to document the `.mcp.json` pinning rule (with a `grep` invariant for verification), the probe-cwd requirement, and the prohibition on inline SKILL.md content in `postinstall.js`. These all come from concrete bugs hit in 0.3.0–0.3.4.

---

## [0.3.4] - 2026-05-25

### Changed
- **Plugin marketplace consolidated to a single plugin.** Previously the marketplace listed four separate plugins: `vision-squeezer-mcp` for the MCP server and three standalone plugins (`vision-doctor`, `vision-upgrade`, `vision-stats`) for the skills. End users had to run four `/plugin install` commands to get the full experience, and most stopped after the first — they then saw stale skill output (the marketplace cache was fresh, but the un-installed skills were shadowed by older user-level copies). The four plugin entries are now collapsed into one `vision-squeezer-mcp` plugin that bundles the MCP server **and** all three skills. `/plugin install vision-squeezer-mcp@vision-squeezer` is now the complete install.
- The three skill directories moved from `skills/<name>/` (repo root) to `plugins/vision-squeezer-mcp/skills/<name>/` so they ship inside the plugin's source directory.

### Breaking
- Users who previously installed any of `vision-doctor@vision-squeezer`, `vision-upgrade@vision-squeezer`, or `vision-stats@vision-squeezer` as standalone plugins will see them disappear from the marketplace after `/plugin marketplace update vision-squeezer`. The same skills are now reachable by installing (or updating) `vision-squeezer-mcp@vision-squeezer`. Cleanup:
  ```
  /plugin remove vision-doctor@vision-squeezer
  /plugin remove vision-upgrade@vision-squeezer
  /plugin remove vision-stats@vision-squeezer
  /plugin update vision-squeezer-mcp@vision-squeezer
  ```
- Older user-level skill copies at `~/.claude/skills/vision-{doctor,upgrade,stats}/` will shadow the bundled versions. Delete them after upgrading: `rm -rf ~/.claude/skills/vision-doctor ~/.claude/skills/vision-upgrade ~/.claude/skills/vision-stats`.

---

## [0.3.3] - 2026-05-25

### Fixed
- **npm tarball trimmed from 29 MB → 16 kB.** The consolidated `publish` job in 0.3.2 downloaded GitHub Release artifacts into `artifacts/` and `dist/` before invoking `npm publish`, so npm packed the platform binaries (and a stale bundled macOS binary) into the npm tarball. `.gitignore` is not always honored by npm pack — add an explicit `files` allowlist in `package.json` listing only `bin/install.js`, `bin/run.js`, and `postinstall.js`. The runtime binary is still downloaded by `postinstall.js` from the GitHub Release on install.
- **`vision-squeezer install` now pins the MCP version.** Old: `claude mcp add vision-squeezer -- npx -y vision-squeezer`. New: `claude mcp add vision-squeezer -- npx -y vision-squeezer@<PKG_VERSION>`. Without the `@version` suffix, npm's `~/.npm/_npx` cache freezes users on whatever tarball was first resolved — multiple "MCP failed to connect" reports trace back to this. The pinned version busts the cache on every upgrade.

### Changed
- **`vision-upgrade` skill rewritten for self-healing recovery.** Now flushes `~/.npm/_npx`, re-registers the MCP with the latest pinned version, and runs a real `initialize` probe against the registered command. `--force` mode wipes the cache and re-registers across all scopes. Replaces the previous version which falsely claimed npx users were "always on latest" (true before pinning, no longer true after).
- **`vision-doctor` skill rewritten for accurate, actionable diagnostics.** Distinguishes `cargo` / `npm-global` / `npx-pinned` / `npx-unpinned` install modes, parses the pinned version out of the MCP registration, and **actively probes** the registered command via a JSON-RPC `initialize` request (portable `python3` timeout — macOS has no GNU `timeout`). Failures surface the captured stderr verbatim and end with a one-line fix: `vision-upgrade` or `vision-upgrade --force`.
- **`CLAUDE.md`** restructured: behavioral guidelines (Think Before Coding / Simplicity / Surgical Changes / Goal-Driven Execution) up top, project context (commands, architecture, release invariants, installer contract) below.

---

## [0.3.2] - 2026-05-25

### Fixed
- **Release pipeline: `cargo publish` no longer trips on the dirty working tree** — when the three publish jobs were collapsed into one runner in 0.3.1, the `download-artifact` step left `artifacts/` and `dist/` as untracked files in the checkout. `cargo publish` refuses to run against a dirty git tree, so the crates.io step failed and the subsequent `npm publish` step never ran (both registries stayed pinned at 0.3.0 even though the GitHub Release for v0.3.1 was created). Reordered the steps so `cargo publish` runs first against a clean tree, then artifacts are downloaded for the GitHub Release + npm publish. Also added `/artifacts/` and `/dist/` to `.gitignore` as belt-and-suspenders in case the step order is ever shuffled.

### Note
- crates.io and npm have a 0.3.1 gap; both go from 0.3.0 → 0.3.2. The GitHub Release for v0.3.1 (binaries only) remains. No functional changes between 0.3.1 and 0.3.2 — this is purely a release-pipeline fix.

---

## [0.3.1] - 2026-05-25

### Changed
- **`release.yml` collapsed three jobs into one** — `release`, `publish-crates`, and `publish-npm` previously ran on three separate runners. Each step is sub-minute work, so two of those runners were paying the ~30s boot cost for nothing. They're now sequential steps in a single `publish` job (saves ~1 minute of wall-clock per tag and avoids the multi-runner artifact-download dance).
- **`python.yml` no longer auto-builds on tags** — PyPI publishing is still commented out in this file, so every tag was producing five sets of wheels that nothing consumed. Switched the trigger to `workflow_dispatch` only; re-enable the tag trigger once PyPI publishing is wired up.

---

## [0.3.0] - 2026-05-25

### Fixed
- **MCP: notifications no longer get a response** — `notifications/initialized` (and any other JSON-RPC notification) is now correctly handled as a notification per the spec. Previously the server required an `id` field on every request, which caused notifications to fail parsing and emit a spurious `parse error` reply. Some clients interpreted that as a protocol failure, leading to ~30s connect timeouts on cold start.

### Added
- **`npx vision-squeezer install`** — interactive installer that prompts for the target CLI (Claude Code / Codex / Qwen), install method (`plugin` / `mcp-add`), and install scope (`user` / `local` / `project`), then runs the matching command. Use `--client` / `--method` / `--scope` / `--yes` flags for non-interactive setups.
- **`vision-squeezer-mcp` Claude Code plugin** — bundles the MCP server config so users can install everything in one shot via `/plugin install vision-squeezer-mcp@vision-squeezer`. No manual `claude mcp add` required. Lives under `plugins/vision-squeezer-mcp/` and is listed in `.claude-plugin/marketplace.json`.

### Changed
- **README install section** — leads with the plugin marketplace one-liner; documents the three `mcp add` scopes (`user`, `local`, `project`) explicitly instead of defaulting silently to `local`.
- **Plugin marketplace versions** synced to crate/npm version (0.1.9 → 0.3.0).
- **CI cost reductions** —
  - `ci.yml` merged `test` + `lint` into a single `check` job (saves one runner spin-up per push), added `paths-ignore` for docs/markdown changes, and added `concurrency.cancel-in-progress` so superseded PR pushes are killed.
  - `release.yml` and `python.yml` got the same `concurrency` block to cancel re-tagged builds. Artifact retention dropped from the 90-day default to 7–14 days.
  - `python.yml` trimmed Python interpreters from `3.8..3.13` to `3.10..3.13` (3.8/3.9 are EOL) and dropped the Intel-macOS target (`macos-latest` is arm64 only — Intel would be billed at 10× the Linux rate).

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

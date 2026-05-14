# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

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

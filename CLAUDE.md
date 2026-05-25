# CLAUDE.md

Guidance for Claude Code (claude.ai/code) when working in this repository.

The first half is **behavior** (how to work). The second half is **project context** (what to work on).

---

## Part 1 — Behavioral Guidelines

Bias toward caution over speed. For trivial tasks, use judgment.

### 1. Think Before Coding

Don't assume. Don't hide confusion. Surface tradeoffs.

Before implementing:
- State assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them — don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 2. Simplicity First

Minimum code that solves the problem. Nothing speculative.

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Test: "Would a senior engineer call this overcomplicated?" If yes, simplify.

### 3. Surgical Changes

Touch only what you must. Clean up only your own mess.

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it — don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that **your** changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: every changed line should trace directly to the user's request.

### 4. Goal-Driven Execution

Define success criteria. Loop until verified.

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass."
- "Fix the bug" → "Write a test that reproduces it, then make it pass."
- "Refactor X" → "Ensure tests pass before and after."

For multi-step tasks, state a brief plan:

```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

---

## Part 2 — Project Context

### Commands

```bash
cargo build          # compile
cargo run            # run
cargo test           # all tests
cargo test <name>    # single test by name
cargo clippy         # lint
cargo fmt            # format
```

### Architecture

Rust 2024 edition. Three source files:

- `src/lib.rs` — core pipeline: semantic crop → tile-aware resize → OCR binarize, `ProcessConfig`, token estimation for Claude/GPT-4o/Gemini, WebP/JPEG output.
- `src/main.rs` — CLI binary (`vision-squeezer`): file I/O, `--format` flag, token savings table.
- `src/mcp_server.rs` — MCP binary (`vision-squeezer-mcp`): JSON-RPC over stdio, exposes `optimize_image` tool.

### Key Types

- `ProcessConfig` — all tunable params (tile size, quality, crop tolerance, output format, provider).
- `VisionModel` — `Claude` / `Gpt4o` / `Gemini`, drives `estimate_tokens()`.
- `OutputFormat` — `Jpeg` / `WebP`.

### Distribution

The project ships across three registries — every release bumps all of them in sync:

- **crates.io** (`vision-squeezer`) — Rust source crate, installable via `cargo install vision-squeezer`.
- **npm** (`vision-squeezer`) — wrapper package. `bin/run.js` spawns the bundled MCP binary; `postinstall.js` downloads the platform-correct binary from the GitHub Release on install.
- **GitHub Releases** (`v*` tags) — prebuilt MCP binaries for darwin-arm64, linux-x86_64, linux-arm64, win-x86_64.

The Claude Code plugin marketplace (`.claude-plugin/marketplace.json`) bundles four plugins: `vision-squeezer-mcp`, `vision-stats`, `vision-doctor`, `vision-upgrade`.

### Release Workflow Invariants

- **Bump `Cargo.lock` in lockstep with `Cargo.toml`.** Cargo writes the workspace crate's own version into `Cargo.lock`; if the two drift, `cargo publish` regenerates the lockfile mid-run, dirties the working tree, and aborts. The release workflow uses `cargo publish --locked` to catch this at CI build time.
- **Job order matters in `release.yml`.** `cargo publish` runs **before** `download-artifact` so the tree stays clean — downloaded artifacts otherwise leave `artifacts/` and `dist/` as untracked files and trip the dirty-tree check.
- **All version-bearing manifests must be bumped together:** `Cargo.toml`, `Cargo.lock`, `package.json`, `.claude-plugin/plugin.json`, `plugins/vision-squeezer-mcp/.claude-plugin/plugin.json`, and the four `version` entries in `.claude-plugin/marketplace.json`.

### Installer + MCP Registration

- `bin/install.js` registers the MCP via `npx -y vision-squeezer@<PINNED_VERSION>` — the explicit `@X.Y.Z` is **load-bearing**. Without it, `npx`'s cache in `~/.npm/_npx` freezes users on whatever tarball was first resolved, even after `npm install -g vision-squeezer@latest` bumps the global. Past "MCP failed to connect" reports trace back to this cache.
- The `vision-upgrade` skill flushes `~/.npm/_npx` and re-registers with the new pinned version on every upgrade.
- The `vision-doctor` skill must actively probe the registered MCP command (spawn it, send `initialize`, read the response). Registration without a successful probe is a broken install.

### Notes

- Token savings are dimensional only — format (JPEG vs WebP) affects file size, not API tokens.
- GPT-4o pre-fits to 2048px before tiling → may show 0% token savings for certain inputs (correct behavior).
- **MCP server communicates via stdin/stdout — do not add logging to stdout.** Any non-JSON-RPC byte on stdout breaks the protocol. Use stderr for diagnostics.

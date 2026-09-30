<p align="center">
  <img src="assets/logo.png" width="300" alt="VisionSqueezer Logo" />
</p>

# VisionSqueezer

<p align="center">
  <a href="https://github.com/eralpozcan/vision-squeezer/actions"><img src="https://img.shields.io/github/actions/workflow/status/eralpozcan/vision-squeezer/ci.yml?label=build" alt="Build"></a>
  <a href="https://crates.io/crates/vision-squeezer"><img src="https://img.shields.io/crates/v/vision-squeezer" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/vision-squeezer"><img src="https://img.shields.io/npm/v/vision-squeezer" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Elastic--2.0-blue" alt="License"></a>
</p>

LLM-native image optimization middleware & MCP server. Reduces vision model token consumption by preprocessing images into tile-boundary-aligned, padding-free formats.

Works with **any agent or editor** that speaks MCP — Claude, GPT, Gemini, Codex, or your own.

---

## Install

### Interactive (recommended)

Picks the client, method, and scope for you:

```bash
npx vision-squeezer install
```

Prompts for:
- Target CLI — Claude Code / Codex CLI / Qwen Code / OpenCode / Gemini CLI / Kimi CLI
- Install method (Claude Code only) — `plugin` (bundles MCP + stats/doctor/upgrade skills) or `mcp-add` (server only)
- Install scope (`mcp-add` only) — `user` (all projects, recommended), `local` (this project only), `project` (share via `.mcp.json`)

Scripted setups pass the choices directly:

```bash
npx vision-squeezer install --client claude --method plugin --yes
npx vision-squeezer install --client claude --method mcp-add --scope user --yes
```

### Claude Code — plugin marketplace (one-liner, bundles skills)

```
/plugin marketplace add eralpozcan/vision-squeezer
/plugin install vision-squeezer-mcp@vision-squeezer
```

Installs the MCP server *and* `/vision-stats`, `/vision-doctor`, `/vision-upgrade` skills as a single Claude Code plugin. Restart open Claude Code sessions for the MCP server to attach.

### Claude Code — `mcp add` (server only)

```bash
# All projects on this machine (recommended)
claude mcp add --scope user vision-squeezer -- npx -y vision-squeezer

# This project only (Claude Code's default)
claude mcp add vision-squeezer -- npx -y vision-squeezer

# Share with the team via .mcp.json in the repo
claude mcp add --scope project vision-squeezer -- npx -y vision-squeezer
```

### Claude Desktop

Add to `~/.config/claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### Cursor

```bash
cursor --add-mcp '{"name":"vision-squeezer","type":"stdio","command":"npx","args":["-y","vision-squeezer"]}'
```

Or add to `.cursor/mcp.json`:
```json
{
  "servers": {
    "vision-squeezer": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

<details>
<summary><b>Click here to view installation instructions for 10+ other IDEs and Agents (VS Code, JetBrains, Windsurf, Zed, etc.)</b></summary>

### VS Code Copilot

Add to `.vscode/mcp.json`:
```json
{
  "servers": {
    "vision-squeezer": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### JetBrains (IntelliJ, WebStorm, PyCharm)

Open **Tools → GitHub Copilot → Model Context Protocol (MCP) → Configure**, then add:
```json
{
  "servers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### Windsurf

Add to `~/.codeium/windsurf/mcp_config.json`:
```json
{
  "mcpServers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### Gemini CLI

```bash
gemini mcp add --scope user vision-squeezer -- npx -y vision-squeezer
```

Or add to `~/.gemini/settings.json` (user) / `.gemini/settings.json` (project):
```json
{
  "mcpServers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### Codex CLI

```bash
codex mcp add vision-squeezer -- npx -y vision-squeezer
```

Or add to `~/.codex/config.toml`:
```toml
[mcp_servers.vision-squeezer]
command = "npx"
args = ["-y", "vision-squeezer"]
```

### Qwen Code

```bash
qwen mcp add vision-squeezer -- npx -y vision-squeezer
```

### OpenCode

`opencode mcp add` is interactive-only, so add directly to `~/.config/opencode/opencode.json` (global) or `opencode.json` in the repo root (project):
```json
{
  "mcp": {
    "vision-squeezer": {
      "type": "local",
      "command": ["npx", "-y", "vision-squeezer"],
      "enabled": true
    }
  }
}
```

### Kimi CLI

```bash
kimi mcp add vision-squeezer -- npx -y vision-squeezer
```

### Zed

Add to `~/.config/zed/settings.json`:
```json
{
  "context_servers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### Kiro

Add to `.kiro/settings/mcp.json` (workspace) or `~/.kiro/settings/mcp.json` (global):
```json
{
  "mcpServers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

### Antigravity

MCP-only, no hooks needed. Configure via the Antigravity MCP settings:
```json
{
  "mcpServers": {
    "vision-squeezer": {
      "command": "npx",
      "args": ["-y", "vision-squeezer"]
    }
  }
}
```

</details>

### Manual install (Rust binary)

```bash
# From crates.io
cargo install vision-squeezer

# Or from source
git clone https://github.com/eralpozcan/vision-squeezer && cd vision-squeezer
make install   # builds → ~/.local/bin/
```

Then use the binary path directly in any config above instead of `npx`:
```json
{ "command": "vision-squeezer-mcp" }
```

> **Tip:** Run `npx -y vision-squeezer --setup` to print ready configs with auto-detected paths.

---

## CLI Usage

```bash
vision-squeezer path/to/image.jpg \
  --mode auto|ocr|standard \      # default: auto (detects text/grayscale)
  --format jpeg|webp|avif \       # default: jpeg
  --quality 85 \                  # output quality 1-100 (default: 75)
  --tile-size 256 \               # patch size in px (default: 512)
  --no-crop \                     # disable padding removal
  --smart-crop \                  # edge-energy crop (vs corner-tolerance)
  --auto-quality 0.95 \           # binary-search quality to hit SSIM target
  --bg-tolerance 25 \             # background detection 0-255 (default: 15)
  --model <provider-alias> \ # model-aware resizing; see the model catalog
  --max-tiles 20 \                # hard cap on tile count
  --json \                        # machine-readable JSON output
  --dry-run                       # run pipeline, skip disk write
```

### Batch mode

Pass a directory instead of a file:

```bash
vision-squeezer ./screenshots --recursive --output-dir ./optimized
vision-squeezer ./screenshots --recursive --json > report.json
```

---

<details>
<summary><b>The Math: How Vision Models Bill You in 2026</b></summary>

If you send raw images to an LLM, you are leaking tokens. Modern vision models do not care about your file size (MB/KB); they only care about **pixel dimensions**, but each provider calculates costs completely differently. `vision-squeezer` simulates these algorithms to find the mathematical minimum size that drops your token usage without losing visual context.

### 1. OpenAI GPT-6 / GPT-5.6 (32px Patches)
Current OpenAI vision models count **32×32 patches**, fit high-detail images within a 2048px edge and 2500-patch budget, then apply a 1.2 multiplier.
* **The Fix:** `--model gpt6` removes padding, fits the patch budget, and avoids partially used edge patches. A 1024×1024 input is 1024 patches / 1229 tokens.

### 2. Claude 4.7+ (28px Patches)
Claude counts `ceil(Width/28) × ceil(Height/28)` visual tokens. Claude 4.7+ high-resolution vision uses a 2576px edge and 4784-token budget; `claude-standard` covers the earlier 1568px / 1568-token tier.
* **The Fix:** Strip padding, fit the selected tier, and align dimensions to the 28px grid.

### 3. Gemini 3 (Large Tiles)
Gemini uses a massive **768×768 tile** system (if the image is > 384px). Each tile is a flat 258 tokens.
* **The Fix:** An 800×600 image triggers a 2×1 tile grid (516 tokens). `vision-squeezer` snaps it down slightly to fit exactly inside a 768×768 box, dropping the cost to 258 tokens (**50% savings**).

### 4. Llama 3.2 / 3.3 Vision (560px Tiles)
Meta's Mllama vision tiles images on a **560×560** grid, capped at 4 tiles (~1601 tokens each).
* **The Fix:** A 2400×1670 screenshot trimmed to 2400×1200 drops from a 2×2 to a 2×1 canvas: **6,404 → 3,202 tokens (−50%)**. (Llama 4 uses a different vision encoder and is not modeled.)

### 5. Qwen2-VL / 2.5-VL / 3-VL (28px Patch Grid)
Alibaba's Qwen-VL uses a **28px effective grid** (14px patch × 2×2 merge); `tokens = (W/28)·(H/28)` bounded to `[4, 16384]`.
* **The Fix:** The patch is small, so area is the lever — a 1024×1024 image with its border stripped to 896×896 drops **1,369 → 1,024 tokens (−25%)**.

### 6. DeepSeek Flash / DeepSeek-VL2
DeepSeek Flash now accepts images through its OpenAI-compatible API and documents an upper bound of **384 image tokens per image**. Use `--model deepseek` for the API profile; use `--model deepseek-local` for the exact DeepSeek-VL2 open-weight tile math.

### 7. Kimi K2.5 / K2.6 / K3
Kimi supports native image and video input through Moonshot's OpenAI-compatible API. Moonshot does not publish a stable image billing grid, so `--model kimi` gives an advisory native-resolution estimate while still applying exact crop and file-size optimization.

### 8. DeepSeek-VL2 (384px Anyres Tiles)
SigLIP-384 + 2× pixel-shuffle gives 196 tokens/tile on a `(m·384, n·384)` canvas (`m·n ≤ 9`).
* **The Fix:** An 800×768 image snapped to 768×768 drops from 3×2 to 2×2 tiles: **1,415 → 1,023 tokens (−28%)**. (Open weights — the win is local-inference context, not API billing.)

Legacy `gpt4o` and `gpt5` profiles remain available for endpoints that still use 512px high-detail tiles.

> Full provider math and sources: **[visionsqueezer.com/providers](https://visionsqueezer.com/providers/claude)**

</details>

## Pipeline

```
Input image
  → crop_padding               remove solid-color borders
  → calculate_optimal_dims     snap to tile boundary (always down)
  → [enforce_max_tiles]        optional tile budget cap
  → resize_exact               Lanczos3
  → [binarize]                 OCR mode only: Otsu threshold
  → JPEG/WebP encode           configurable quality & format
```

### 🦀 Why Rust?

VisionSqueezer is a performance-critical middleware. We chose Rust for three uncompromising reasons:

- **Invisible Latency:** Image processing should never be the bottleneck. Rust ensures that snapping, cropping, and encoding happen in milliseconds, making the optimization layer truly invisible to the developer's workflow.
- **Minimal Footprint:** As an MCP server running in the background of your IDE, VisionSqueezer is designed to be ultra-lightweight, consuming near-zero CPU and RAM when idle.
- **Wasm-Ready:** Rust's first-class support for WebAssembly allows us to bring the same high-performance optimization to the browser and the Edge (Cloudflare Workers), enabling client-side squeezing before the image even hits the network.

---

### Legacy benchmark snapshots

The snapshots below were captured with the pre-patch-token estimator and are retained only as historical compression examples. Use `--json --dry-run` for current GPT-6/Claude 4.7 token estimates.

### Case Study 1: Standard Image (istanbul.jpg)
To demonstrate the impact on standard images, here is the run on a 2400×1670 image (4 MP, 0.5 MB) across the three scenarios:

#### Example 1: Agnostic Optimization (Default)
When no target model is specified, Squeezer reduces the file size and mathematically optimizes boundaries to be generally efficient across all models.

```bash
vision-squeezer data/istanbul.jpg
```
```text
Input:  2400×1670  (0.5 MB)
Output: 2048×1536  (0.3 MB, JPG q75)
File:   28.6% smaller

── Token Estimates ─────────────────────────────────────────
Model          Before    After      Saved
------------------------------------------
Claude           5344     4194     1150 (21.5%)
GPT-4o           1105      765      340 (30.8%)
GPT-5            1536     1536        0 (0.0%)
Gemini           3096     1548     1548 (50.0%)
────────────────────────────────────────────────────────────
```

<details>
<summary><b>View Advanced Model-Targeted Optimizations (GPT-4o & Claude)</b></summary>

### Example 2: Model-Targeted Optimization (GPT-4o)
If you tell Squeezer the target model, it reverses the model's exact internal calculation (e.g. GPT-4.5's 768px short-side scaling algorithm) and mathematically shrinks the image just enough to fit the absolute minimum tile grid.

```bash
vision-squeezer data/istanbul.jpg --model gpt4o
```
```text
Input:  2400×1670  (0.5 MB)
Output: 2399×1200  (0.3 MB, JPG q75)
File:   33.6% smaller

── Token Estimates ─────────────────────────────────────────
Model          Before    After      Saved
------------------------------------------
Claude           5344     3838     1506 (28.2%)
GPT-4o           1105     1105        0 (0.0%)
GPT-5            1536     1536        0 (0.0%)
Gemini           3096     2064     1032 (33.3%)
────────────────────────────────────────────────────────────
```
*Notice how targeting `gpt4o` perfectly fits the image into a solid 6-tile boundary (2399x1200) mathematically calculated backwards from OpenAI's short-side scaling algorithm. It maximizes resolution exactly up to the point where an extra tile would be billed.*

### Example 3: Model-Targeted Optimization (Claude)
This historical run used Claude's former area estimator; current releases use the 28px patch model documented above.

```bash
vision-squeezer data/istanbul.jpg --model claude
```
```text
Input:  2400×1670  (0.5 MB)
Output: 2304×1536  (0.4 MB, JPG q75)
File:   21.3% smaller

── Token Estimates ─────────────────────────────────────────
Model          Before    After      Saved
------------------------------------------
Claude           5344     4718      626 (11.7%)
GPT-4o           1105     1105        0 (0.0%)
GPT-5            1536     1536        0 (0.0%)
Gemini           3096     1548     1548 (50.0%)
────────────────────────────────────────────────────────────
```
*Claude benefits tremendously from even minor dimension reductions. By snapping the width and height slightly downwards, we immediately shaved off over 600 tokens while preserving the massive 2304×1536 resolution.*

</details>

### Case Study 2: 12-Megapixel High-Res Image (istanbul2.jpg)
To demonstrate the impact on massive images, here is the run on a 4096×3072 image (12 MP, 2.2 MB) across the three scenarios:

#### Example 1: Agnostic Optimization

```bash
vision-squeezer data/istanbul2.jpg
```
```text
Input:  4096×3072  (2.2 MB)
Output: 3584×2560  (1.3 MB, JPG q75)
File:   39.6% smaller

── Token Estimates ─────────────────────────────────────────
Model          Before    After      Saved
------------------------------------------
Claude          16777    12233     4544 (27.1%)
GPT-4o            765     1105        0 (0.0%)
GPT-5            1536     1536        0 (0.0%)
Gemini           6192     5160     1032 (16.7%)
────────────────────────────────────────────────────────────
```
*(Notice the **OpenAI Aspect Ratio Anomaly**: Squeezer removed heavy letterboxing (padding) from this image. By removing the padding, the image became "wider". Because OpenAI's API forces the *new* short side to 768px, the wide aspect ratio pushed the long side into a 3rd tile grid column! This is a fascinating edge case where cropping padding mathematically INCREASES your GPT-4o token cost. If you specifically use `--model gpt4o` on this image, Squeezer will detect this paradox and use a different grid constraint).*

<details>
<summary><b>View Advanced Model-Targeted Optimizations (GPT-4o & Claude)</b></summary>

#### Example 2: Model-Targeted Optimization (GPT-4o)

```bash
vision-squeezer data/istanbul2.jpg --model gpt4o
```
```text
Input:  4096×3072  (2.2 MB)
Output: 4095×2048  (1.2 MB, JPG q75)
File:   43.2% smaller

── Token Estimates ─────────────────────────────────────────
Model          Before    After      Saved
------------------------------------------
Claude          16777    11182     5595 (33.3%)
GPT-4o            765     1105        0 (0.0%)
GPT-5            1536     1536        0 (0.0%)
Gemini           6192     4644     1548 (25.0%)
────────────────────────────────────────────────────────────
```
*(By explicitly targeting `gpt4o`, Squeezer optimizes the boundaries such that the new aspect ratio is safely contained. While GPT-4o still bills for the 6-tile layout due to the image's inherent width, Squeezer shrinks the file footprint by 43% without sacrificing high-resolution details.)*

#### Example 3: Model-Targeted Optimization (Claude)

```bash
vision-squeezer data/istanbul2.jpg --model claude
```
```text
Input:  4096×3072  (2.2 MB)
Output: 3840×2816  (1.5 MB, JPG q75)
File:   31.5% smaller

── Token Estimates ─────────────────────────────────────────
Model          Before    After      Saved
------------------------------------------
Claude          16777    14417     2360 (14.1%)
GPT-4o            765     1105        0 (0.0%)
GPT-5            1536     1536        0 (0.0%)
Gemini           6192     5160     1032 (16.7%)
────────────────────────────────────────────────────────────
```
*(Historical estimator output; current Claude estimates use the 28px patch profile.)*

> **💡 FAQ: Wait, why did targeting `gpt4o` save 33% of Claude tokens, but targeting `claude` only saved 14%?**
> *Because of the **Quality vs. Aggression trade-off**. OpenAI enforces a strict maximum internal resolution (2048px). When you target `gpt4o`, Squeezer must aggressively squash the massive 4096px image down to fit OpenAI's constraints (4095x2048). This massive loss in total pixel area mathematically translates to a huge token drop for Claude.*
> *However, Claude has **no such maximum limits**. When you explicitly target `claude`, Squeezer knows it doesn't need to destroy your image's resolution. It carefully keeps the massive 3840x2816 size to preserve ultra-fine detail, only trimming the absolute minimum padding to give you the most cost-efficient **lossless** version possible.*

</details>

---

## Current model sanity checks

| Profile | Input | Estimated tokens |
| --- | --- | ---: |
| GPT-6 / GPT-5.6 | 1024×1024 | 1,229 |
| GPT-6 / GPT-5.6 | 2048×2048 | 3,000 after the 2,500-patch cap |
| Claude 4.7+ | 1000×1000 | 1,296 |
| Legacy GPT-5 / 5.1 | 1024×1024 | 630 |

## Legacy benchmark / savings

Historical pre-patch-token results. Run `vision-squeezer <image> --model gpt6 --json --dry-run` for current numbers.

| Original Size | Model | Tokens Before | Tokens After | Saved |
|---------------|-------|---------------|--------------|-------|
| **1025 × 1025**<br>*(Screenshot)* | Claude 4.5+<br>GPT-4.5<br>Gemini 2.0+ | 1,400<br>425<br>1,032 | 1,024<br>255<br>258 | **26.8%**<br>40.0%<br>75.0% |
| **4032 × 3024**<br>*(Phone Camera)* | Claude 4.5+<br>GPT-4.5<br>Gemini 2.0+ | 16,257<br>2,125<br>6,192 | 12,232<br>1,745<br>4,128 | **24.8%**<br>17.9%<br>33.3% |
| **800 × 600**<br>*(Web Image)* | Claude 4.5+<br>GPT-4.5<br>Gemini 2.0+ | 640<br>255<br>1,032 | 341<br>255<br>258 | **46.7%**<br>0.0%<br>75.0% |

*(Legacy snapshot; current OpenAI integrations should use `gpt6`.)*

---

## MCP Tool: `optimize_image`

| Argument | Type | Required | Default |
|----------|------|----------|---------|
| `image_base64` | string | ✓ | — |
| `mode` | `"auto"` \| `"standard"` \| `"ocr"` | — | `"auto"` |
| `output_format` | `"jpeg"` \| `"webp"` | — | `"jpeg"` |
| `quality` | integer 1–100 | — | 75 |
| `tile_size` | integer | — | 512 |
| `crop` | boolean | — | true |
| `bg_tolerance` | integer 0–255 | — | 15 |
| `max_tiles` | integer | — | — |
| `target_model` | string enum | — | Core models plus `glm`, `pixtral`, `gemma`, `internvl`, `minicpm`, `molmo`, `aya`, `phi4`, `granite`, `llava`, `falcon`, `minimax`, `step`, `ling`, `voyage` |

**Response:**
```json
{
  "optimized_base64": "...",
  "savings_report": {
    "tiles_before": 48,
    "tiles_after": 35,
    "tiles_saved": 13,
    "token_reduction_pct": "27.1",
    "size_reduction_pct": "58.9"
  }
}
```

## Config Reference

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `quality` | u8 1–100 | 75 | JPEG/WebP output quality |
| `tile_size` | u32 | 512 | Custom grid size; ignored when `target_model` is set |
| `crop` | bool | true | Remove solid-color padding borders |
| `bg_tolerance` | u8 0–255 | 15 | Max channel delta for background detection |
| `output_format` | jpeg/webp | jpeg | Output encoding. WebP is ~30-50% smaller |
| `max_tiles` | u32 | — | Hard cap on tile count (progressive downscale) |
| `target_model` | string | — | Model-aware profile; use `gpt6` for current OpenAI models |

## Supported Models

## Platform support

Prebuilt MCP binaries cover Linux x86_64/ARM64, macOS Apple Silicon/Intel, and Windows x86_64/ARM64. Python wheels cover Linux x86_64/ARM64, macOS Apple Silicon/Intel, and Windows x86_64. Unsupported Python architectures can install from source with `pip install --no-binary vision-squeezer vision-squeezer`.

| Model | Tile Size | Pre-scaling | Token Formula |
|-------|-----------|-------------|---------------|
| GPT-6 / GPT-5.6 | 32×32 patches | 2048px edge / 2500 patches | ceil(patches × 1.2) |
| Claude 4.7+ high resolution | 28×28 patches | 2576px edge / 4784 patches | 1 token per patch |
| Claude standard | 28×28 patches | 1568px edge / 1568 patches | 1 token per patch |
| GPT-4o / GPT-4.5 (legacy) | 512×512 | fit 2048px → scale short 768px | 85 + tiles × 170 |
| GPT-5 / GPT-5.1 (legacy) | 512×512 | fit 2048px → scale short 768px | 70 + tiles × 140 |
| Gemini 3 | 768×768 | flat tier at ≤384×384 | 258 per tile |
| DeepSeek Flash | provider-managed | up to 384 image tokens | conservative 384-token cap |
| Kimi K2.5/K2.6/K3 | native-resolution | provider-specific | advisory 28px estimate |
| GLM, Pixtral/Mistral, Gemma, InternVL | provider-specific | provider-specific | advisory generic profile |
| MiniCPM, Molmo, Aya, Phi-4, Granite | provider-specific | provider-specific | advisory generic profile |
| LLaVA, Falcon, MiniMax, Step, Ling, Voyage | provider-specific | provider-specific | advisory generic profile |

---

## Advanced Features

### Persistence & Analytics

VisionSqueezer tracks every optimization locally in `~/.vision-squeezer/stats.db`.

```bash
vision-squeezer stats
```
```text
── VisionSqueezer Analytics ────────────────────────────────
Total Optimizations: 42
Total Tokens Saved:  842,500
Total Bytes Saved:   156.40 MB
Estimated USD Saved: $2.11
────────────────────────────────────────────────────────────
```

### Shell Hook & Claude Code Skills

Add to `.zshrc` / `.bashrc`:

```bash
eval "$(vision-squeezer setup-hook)"
```

This installs two things at once:

- **`squeeze` alias** — optimize and capture the output path in one step:
  ```bash
  img_path=$(squeeze data/logo.png --model gemini)
  ```
- **Claude Code skills** — written to `~/.claude/skills/` automatically on first run

#### Available Skills

| Skill | Trigger | What it does |
|-------|---------|--------------|
| `vision-stats` | `/vision-stats` | Show cumulative token & byte savings — reads local stats.db, zero MCP overhead |
| `vision-doctor` | `/vision-doctor` | Check installed version vs latest npm release, show update command if outdated |
| `vision-upgrade` | `/vision-upgrade` | Detect install method (cargo/npm/npx) and run the correct upgrade command |

**Example output:**

```
/vision-stats
── VisionSqueezer Analytics ────────────────────────────────
Total Optimizations: 42
Total Tokens Saved:  842,500
Total Bytes Saved:   156.40 MB
Estimated USD Saved: $2.11
────────────────────────────────────────────────────────────

/vision-doctor
## VisionSqueezer Doctor
- [x] Binary found: /usr/local/bin/vision-squeezer
- [x] Installed version: 0.1.8
- [x] Latest version (npm): 0.1.8
- [x] Status: Up to date
```

**Marketplace install** (alternative to `setup-hook`):
Add to `~/.claude/settings.json`:
```json
{
  "extraKnownMarketplaces": {
    "vision-squeezer": {
      "source": { "source": "github", "repo": "eralpozcan/vision-squeezer" }
    }
  }
}
```
Then run `/plugins add vision-stats@vision-squeezer`, `/plugins add vision-doctor@vision-squeezer`, or `/plugins add vision-upgrade@vision-squeezer` in Claude Code.

### Sandbox Mode: "Think in Code"

Execute atomic operations locally before the image ever reaches an LLM. The agent decides _how_ to process the image; you pay zero tokens for the intermediate steps.

**CLI:**
```bash
vision-squeezer screenshot.png --ops '[{"op":"crop","x":10,"y":20,"width":500,"height":500},{"op":"binarize"}]'
```

**MCP tool — `sandbox_execute`:**

| Argument | Type | Description |
|----------|------|-------------|
| `image_base64` | string | Input image |
| `operations` | array | Ordered list of ops to apply |

Supported ops: `crop`, `grayscale`, `binarize`, `resize`, `contrast`, `brightness`.

### Crawler Integration

Optimize images on the fly in any Playwright/Puppeteer scraping pipeline — zero API token waste on raw screenshots.

```javascript
await page.route('**/*.{png,jpg}', async (route) => {
  const response = await route.fetch();
  const body = await response.body();
  const optimized = await squeeze(body); // pipe through vision-squeezer
  route.fulfill({ body: optimized });
});
```

---

## Contributing

PRs welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup and guidelines.
Please follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## License

Elastic License 2.0 (ELv2) — See [LICENSE](LICENSE) for details.

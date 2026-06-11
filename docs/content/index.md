---
seo:
  title: VisionSqueezer — Stop Leaking Vision Tokens
  description: LLM-native image optimization middleware & MCP server. Mathematically snaps images to Claude, GPT, and Gemini grid boundaries to cut vision token usage by up to 90%.
---

::u-page-hero{class="dark:bg-gradient-to-b from-neutral-900 to-neutral-950"}
---
orientation: horizontal
---
#top
:hero-background

#headline
:version-badge

#title
Stop Leaking [Vision Tokens]{.text-primary}.

#description
The LLM-native image optimization middleware. It mathematically snaps your images to Claude, GPT, and Gemini's exact internal grid boundaries to slash token usage by up to 90% — without losing visual detail.

#links
  :::u-button
  ---
  to: /getting-started
  size: xl
  trailing-icon: i-lucide-arrow-right
  ---
  Get started
  :::

  :::u-button
  ---
  icon: i-simple-icons-github
  color: neutral
  variant: outline
  size: xl
  to: https://github.com/eralpozcan/vision-squeezer
  target: _blank
  ---
  View on GitHub
  :::

#default
  ```bash [terminal]
  cargo run -- data/image.jpg --model gpt4o

  // Squeezer simulating OpenAI's short-side algorithm...
  Input:  4096×3072  (2.2 MB)
  Output: 4095×2048  (1.2 MB)

  Tokens Saved: 5,595 (33.3% cheaper)
  ```
::

::u-page-section
#title
The Math Behind the Magic

#description
Every provider tokenizes images differently. Squeezer simulates each provider's internal grid math and snaps your images to the cheapest valid boundary.

#features
  :::u-page-feature
  ---
  icon: i-lucide-square
  ---
  #title
  Claude (Area-Based)

  #description
  Claude bills strictly by pixel area `(W × H / 750)`. Every pixel of padding costs you. Squeezer aggressively crops solid borders, shaving thousands of tokens instantly.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-grid-2x2
  ---
  #title
  GPT-4o (Tiling System)

  #description
  OpenAI forcefully scales the shortest side to 768px, then tiles it. Squeezer simulates this backwards to snap your image right under the exact 512px tile threshold.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-grid-3x3
  ---
  #title
  Gemini (Massive Tiles)

  #description
  Gemini uses huge 768×768 blocks. A slightly overlapping image costs you double. We snap images securely down to the nearest tile boundary.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-flask-conical
  ---
  #title
  Think in Code (Sandbox)

  #description
  Let your agent execute custom crops, binarization, or filters locally. Extract only the context you need to save up to 99.9% tokens.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-bar-chart-3
  ---
  #title
  Persistent Analytics

  #description
  Locally tracks every optimization in a SQLite database. View your cumulative USD savings directly from your terminal or AI agent.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-plug
  ---
  #title
  Universal MCP

  #description
  Works natively with Claude Code, Cursor, Zed, and VS Code. No complex setup — just plug it into your favorite AI tool.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-image
  ---
  #title
  AVIF Output

  #description
  `--format avif` encodes ~20–50% smaller than WebP at equal quality, ~3× smaller than JPEG. Same tokens, less bandwidth.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-crop
  ---
  #title
  Smart Crop & Auto-Quality

  #description
  `--smart-crop` uses edge-energy (Sobel-lite) to keep high-information regions. `--auto-quality 0.95` binary-searches quality to hit a perceptual SSIM target.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-package
  ---
  #title
  Batch & JSON Output

  #description
  Pass a directory + `--recursive` to squeeze a whole tree at once. `--json` emits a structured record for pipelines. `--dry-run` reports without writing.
  :::
::

::u-page-section
#title
Interactive [Savings Calculator]{.text-primary}

#description
Real numbers from the Squeezer pipeline. Pick an image source and an optimization target to see token and file-size savings per provider.

#default
  :savings-calculator
::

::u-page-section
#title
Universal [MCP Integration]{.text-primary}

#description
Select your agent or editor. Thanks to `npx -y`, zero global installation is required — just paste the configuration.

#default
  :install-selector
::

::u-page-section
#title
GPT-5: What changes?

#description
GPT-5 handles up to **10.24 Megapixels** natively (hard cap 1536 tokens). Because of these massive architectural limits, grid-tiling optimization is rarely needed. However, Squeezer still strips heavy padding and compresses file sizes (MBs → KBs) for much faster API uploads and drastically reduced latency.
::

::u-page-section{class="dark:bg-gradient-to-b from-neutral-950 to-neutral-900"}
  :::u-page-c-t-a
  ---
  links:
    - label: Get started
      to: '/getting-started'
      trailingIcon: i-lucide-arrow-right
    - label: View on GitHub
      to: 'https://github.com/eralpozcan/vision-squeezer'
      target: _blank
      variant: subtle
      icon: i-simple-icons-github
  title: Stop paying for padding.
  description: Install VisionSqueezer in one command and start cutting vision token costs across every provider.
  class: dark:bg-neutral-950
  ---

  :stars-bg
  :::
::

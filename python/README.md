# vision-squeezer (Python)

Python bindings for [vision-squeezer](https://github.com/eralpozcan/vision-squeezer) — LLM-native image optimization. Reduces vision model token consumption by snapping to tile boundaries.

## Install

```bash
pip install vision-squeezer
```

From source (requires Rust + `pip install maturin`):

```bash
cd python && maturin develop --release
```

## Usage

```python
import vision_squeezer as vs

# Optimize an image on disk and write next to it.
report = vs.optimize_image(
    "screenshot.png",
    model="claude",
    quality=75,
    output_path="screenshot.optimized.jpg",
)
print(f"Saved {report['tokens_saved']} tokens, {report['size_reduction_pct']:.1f}% smaller")

# Or work with raw bytes.
with open("screenshot.png", "rb") as f:
    raw = f.read()
report = vs.optimize_image(raw, model="gpt4o", format="webp")
# report["bytes"] holds the optimized image
# report["base64"] is the same image, base64-encoded

# Estimate tokens without optimizing.
est = vs.estimate_tokens(4096, 3072, model="claude")
print(est)  # {'model': 'claude', 'tokens': ..., 'tiles': ...}

# Optimal send dimensions for a given model.
print(vs.optimal_dimensions(4096, 3072, model="gpt4o"))
```

## Supported models

- `claude` (3.5 / 4.x — area-based token estimation)
- `gpt4o` (GPT-4o / 4.5 — 2048 fit then 768 short-side, 512px tiles)
- `gpt5` (GPT-5/5.5 — 6000px max, 10.24M pixel cap, 1536 token cap)
- `gemini` (Gemini 2.0/3.0 — flat 258 ≤384×384, else 768px tiles)

## Output formats

`jpeg` (default) and `webp`. Format affects bytes, not tokens.

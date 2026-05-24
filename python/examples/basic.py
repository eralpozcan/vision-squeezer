"""Basic usage of vision_squeezer Python bindings.

Run after `maturin develop --release` from the python/ directory.
"""

import sys

import vision_squeezer as vs


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: python basic.py <image_path>")
        sys.exit(1)

    path = sys.argv[1]

    # Token estimation only.
    for model in ("claude", "gpt4o", "gpt5", "gemini"):
        est = vs.estimate_tokens(4096, 3072, model=model)
        print(f"{model:>8}: {est['tokens']:>6} tokens ({est['tiles']} tiles)")

    print()
    print(f"Optimizing {path}...")
    report = vs.optimize_image(
        path,
        model="claude",
        quality=75,
        output_path=path.rsplit(".", 1)[0] + ".optimized.jpg",
    )

    print(f"  {report['input_width']}x{report['input_height']} -> "
          f"{report['output_width']}x{report['output_height']}")
    print(f"  Tokens: {report['tokens_before']} -> {report['tokens_after']} "
          f"(saved {report['tokens_saved']})")
    print(f"  File size: {report['size_reduction_pct']:.1f}% smaller")


if __name__ == "__main__":
    main()

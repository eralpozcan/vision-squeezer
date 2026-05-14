---
name: vision-stats
description: >
  Show VisionSqueezer cumulative token & byte savings analytics. Zero MCP
  overhead — reads directly from local stats.db via CLI binary. Use when user
  says "vision-stats", "squeeze stats", "token savings", "how much saved",
  "vision-squeezer stats", "optimization history", or "/vision-stats".
allowed-tools: Bash
---

# vision-stats — VisionSqueezer Analytics Skill

Zero-overhead stats. Calls `vision-squeezer stats` directly — no MCP round-trip.

## Trigger

`/vision-stats` or any of: "vision stats", "squeeze stats", "show savings", "how much have I saved", "optimization stats"

## Action

Run this binary resolution chain, stop at first success:

```bash
vision-squeezer stats 2>/dev/null \
  || ~/.cargo/bin/vision-squeezer stats 2>/dev/null \
  || "$(dirname "$(command -v vision-squeezer-mcp 2>/dev/null)")/vision-squeezer" stats 2>/dev/null \
  || find "$HOME/.cargo/bin" "$HOME/Desktop" "$HOME/Projects" -maxdepth 6 -name "vision-squeezer" -not -path "*/deps/*" -not -path "*/debug/*" 2>/dev/null | head -1 | xargs -I{} {} stats 2>/dev/null \
  || echo "vision-squeezer not found. Install: cargo install --git https://github.com/eralpozcan/vision-squeezer"
```

Print output verbatim. No wrapping, no commentary, no interpretation.

## Error handling

Binary not found → tell user to run `cargo install --path .` from project root or `eval "$(vision-squeezer setup-hook)"` after install.

## Notes

- Stats persist in `~/.local/share/vision-squeezer/stats.db` (XDG) or platform equivalent
- MCP tool `get_savings_stats` does the same but costs ~150 tokens overhead — use this skill instead

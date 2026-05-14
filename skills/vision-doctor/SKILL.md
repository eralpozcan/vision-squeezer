---
name: vision-doctor
description: >
  Check VisionSqueezer installation health and version status. Detects installed
  version, compares against latest npm release, and shows update command if outdated.
  Use when user says "vision-doctor", "check vision-squeezer version", "update vision-squeezer",
  "is vision-squeezer up to date", "upgrade vision-squeezer", or "/vision-doctor".
allowed-tools: Bash
---

# vision-doctor — VisionSqueezer Health Check Skill

Checks binary installation, current version, and latest available version.

## Trigger

`/vision-doctor` or any of: "vision doctor", "check vision-squeezer", "update vision-squeezer",
"is vision-squeezer up to date", "upgrade vision-squeezer", "vision-squeezer version"

## Action

Run the following shell script:

```bash
BIN=$(command -v vision-squeezer 2>/dev/null)
if [ -z "$BIN" ] && [ -x "$HOME/.cargo/bin/vision-squeezer" ]; then
  BIN="$HOME/.cargo/bin/vision-squeezer"
fi
if [ -n "$BIN" ] && [ -x "$BIN" ]; then
  INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
else
  INSTALLED=""
  BIN=""
fi
LATEST=$(npm view vision-squeezer version 2>/dev/null)
MCP_CMD=$(claude mcp list 2>/dev/null | grep vision-squeezer | head -1 || echo "")
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "LATEST=$LATEST"
echo "MCP=$MCP_CMD"
```

## Output format

Display as a markdown checklist:

```
## VisionSqueezer Doctor

- [x/ ] Binary found: <path or "not found (using npx)">
- [x/ ] Installed version: <version or "n/a — npx always pulls latest">
- [x/ ] Latest version (npm): <version>
- [x/ ] MCP registered: <yes/no>
- [x/ ] Status: <see below>
```

### Status logic

| Condition | Status |
|-----------|--------|
| `INSTALLED` == `LATEST` | ✅ Up to date |
| `INSTALLED` != `LATEST`, both non-empty | ⚠️ Update available — run `/vision-upgrade` |
| `BIN` empty, `MCP` contains "npx" | ✅ Using npx — always latest, no action needed |
| `BIN` empty, no MCP | ❌ Not installed |

### If update available:

```
Update available: v<INSTALLED> → v<LATEST>
Run /vision-upgrade to update.
```

### If not installed:

```
## VisionSqueezer not found

Install via Claude Code (one-liner):
  claude mcp add vision-squeezer -- npx -y vision-squeezer
```

## Notes

- `npx -y vision-squeezer` users are always on latest — show this as ✅, not an error
- cargo install users must run `/vision-upgrade` or `cargo install vision-squeezer` to upgrade

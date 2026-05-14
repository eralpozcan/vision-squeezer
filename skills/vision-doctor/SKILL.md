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

Run the following shell script and display the formatted checklist result:

```bash
# 1. Resolve binary
BIN=$(command -v vision-squeezer 2>/dev/null \
  || echo ~/.cargo/bin/vision-squeezer)

# 2. Get installed version
if [ -x "$BIN" ]; then
  INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
else
  INSTALLED=""
fi

# 3. Get latest npm version
LATEST=$(npm view vision-squeezer version 2>/dev/null)

# 4. Emit result
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "LATEST=$LATEST"
```

## Output format

Display as a markdown checklist based on the values:

```
## VisionSqueezer Doctor

- [x/] Binary found: <path or "not found">
- [x/] Installed version: <version or "unknown">
- [x/] Latest version (npm): <version or "unavailable">
- [x/] Status: Up to date / Update available / Not installed
```

Use `[x]` for OK/pass, `[ ]` for missing/fail.

### If update available (`INSTALLED` != `LATEST` and both non-empty):

Show update commands:

```
## Update available: v<INSTALLED> → v<LATEST>

### Via npx (no action needed — always pulls latest automatically)

### Via cargo:
cargo install vision-squeezer

### Via npm global:
npm install -g vision-squeezer
```

### If not installed:

```
## VisionSqueezer not found

Install via Claude Code (one-liner):
claude mcp add vision-squeezer -- npx -y vision-squeezer

Or via cargo:
cargo install vision-squeezer
```

## Notes

- `npx -y vision-squeezer` users are always on latest — no update needed
- cargo install users must run `cargo install vision-squeezer` to upgrade
- npm global users run `npm install -g vision-squeezer`

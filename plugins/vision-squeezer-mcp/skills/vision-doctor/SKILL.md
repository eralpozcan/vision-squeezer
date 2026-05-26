---
name: vision-doctor
description: >
  Check VisionSqueezer installation health and version status. Detects the installed binary
  or MCP-registered command, parses the pinned npx version, compares against the latest npm
  release, actively probes the MCP server, and surfaces the root cause + one-line fix for any
  failure. Use when user says "vision-doctor", "check vision-squeezer version", "is vision-squeezer
  up to date", "vision-squeezer not working", or "/vision-doctor".
allowed-tools: Bash
---

# vision-doctor — VisionSqueezer Health Check Skill

Reports accurate state for all three install paths (cargo / npm-global / npx) and actively
probes the MCP server. Every failure line ends with a single recovery command — typically
`vision-upgrade` (or `vision-upgrade --force`).

## Trigger

`/vision-doctor` or any of: "vision doctor", "check vision-squeezer", "is vision-squeezer up to date",
"vision-squeezer broken", "MCP not connecting", "vision-squeezer not working", "vision-squeezer version".

## Action — Step 1: Detect state

```bash
LATEST=$(npm view vision-squeezer version 2>/dev/null)

# Direct binary (cargo / npm global)
BIN=$(command -v vision-squeezer 2>/dev/null)
[ -z "$BIN" ] && [ -x "$HOME/.cargo/bin/vision-squeezer" ] && BIN="$HOME/.cargo/bin/vision-squeezer"
INSTALLED=""
[ -n "$BIN" ] && INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)

# MCP registration — parse the actual command + pinned version
MCP_LINE=$(claude mcp list 2>/dev/null | grep -i vision-squeezer | head -1)
MCP_PINNED=$(echo "$MCP_LINE" | grep -oE 'vision-squeezer@[0-9]+\.[0-9]+\.[0-9]+' | sed 's/vision-squeezer@//')
MCP_MODE=""
if [ -n "$MCP_LINE" ]; then
  if [ -n "$MCP_PINNED" ]; then MCP_MODE="npx-pinned";
  elif echo "$MCP_LINE" | grep -q npx; then MCP_MODE="npx-unpinned";
  else MCP_MODE="binary"; fi
fi

# What version is the user effectively running?
EFFECTIVE=""
case "$MCP_MODE" in
  npx-pinned) EFFECTIVE="$MCP_PINNED" ;;
  binary)     EFFECTIVE="$INSTALLED" ;;
  npx-unpinned) EFFECTIVE="unknown (unpinned npx — cache decides)" ;;
esac
[ -z "$EFFECTIVE" ] && [ -n "$INSTALLED" ] && EFFECTIVE="$INSTALLED"

echo "LATEST=$LATEST"
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "MCP_LINE=$MCP_LINE"
echo "MCP_MODE=$MCP_MODE"
echo "MCP_PINNED=$MCP_PINNED"
echo "EFFECTIVE=$EFFECTIVE"
```

## Action — Step 2: Active MCP probe

If `MCP_LINE` non-empty, spawn the registered MCP command and send an `initialize` request.
Capture stderr — most failures are postinstall download errors that the message reveals.

```bash
# Reconstruct the runnable command from the MCP_LINE. Fields after the colon are the command.
# Format examples:
#   vision-squeezer: npx -y vision-squeezer@0.3.3 - ✓ Connected
#   vision-squeezer: /Users/x/.cargo/bin/vision-squeezer-mcp - ✗ Failed to connect
CMD=$(echo "$MCP_LINE" | sed -E 's/^[^:]+:[[:space:]]*//' | sed -E 's/[[:space:]]+- (✓|✗).*$//')
[ -z "$CMD" ] && CMD="npx -y vision-squeezer@${LATEST}"

# Use python3 for a portable 8s timeout (macOS lacks GNU `timeout`).
# cwd=$HOME because npx checks the cwd's package.json — if the user happens to
# be inside the vision-squeezer project dir (name collision), npx skips the
# install and the probe false-negatives with "command not found". Running from
# $HOME guarantees a neutral resolution context.
PROBE_OUT=$(python3 - "$CMD" "$HOME" <<'PY' 2>&1
import subprocess, sys, json
cmd = sys.argv[1]
home = sys.argv[2]
req = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vision-doctor","version":"1"}}}\n'
try:
    r = subprocess.run(cmd, shell=True, input=req, capture_output=True, text=True, timeout=8, cwd=home)
    if r.stdout:
        print(r.stdout.splitlines()[0])
    if r.stderr:
        print("---STDERR---", file=sys.stderr)
        print(r.stderr[:2000], file=sys.stderr)
except subprocess.TimeoutExpired as e:
    print("PROBE_TIMEOUT after 8s", file=sys.stderr)
    if e.stderr:
        print(e.stderr[:2000].decode('utf-8', errors='replace'), file=sys.stderr)
PY
)
if echo "$PROBE_OUT" | head -1 | grep -q '"jsonrpc"'; then
  echo "PROBE=ok"
else
  echo "PROBE=fail"
  echo "PROBE_STDERR<<EOF"
  echo "$PROBE_OUT" | head -20
  echo "EOF"
fi
```

## Output format

Display as a markdown checklist with accurate per-mode language. **Never claim "npx always pulls
latest" — that statement is wrong now that the installer pins versions.**

```
## VisionSqueezer Doctor

- [x] Latest (npm):       <LATEST>
- [x] Install mode:       <cargo / npm-global / npx-pinned / npx-unpinned / not-installed>
- [x] Effective version:  <EFFECTIVE>
- [x] MCP registered:     <yes (scope: X, mode: Y) / no>
- [x] MCP probe:          <✅ connected / ❌ <one-line reason>>
- [x] Status:             <see status table below>
```

### Status logic

| Condition | Status |
|-----------|--------|
| `MCP_MODE` empty AND `BIN` empty | ❌ Not installed — run `npx vision-squeezer install` |
| `MCP_MODE` == `npx-unpinned` | ⚠️ Unpinned npx — cache may be stale. Run `vision-upgrade` |
| `EFFECTIVE` != `LATEST` (both known) | ⚠️ Outdated v`EFFECTIVE` → v`LATEST`. Run `vision-upgrade` |
| `PROBE` == `fail` | ❌ MCP failed to start — see stderr. Run `vision-upgrade --force` |
| `EFFECTIVE` == `LATEST` AND `PROBE` == `ok` | ✅ Healthy |

### When probe fails, include the captured stderr verbatim:

```
MCP probe failed. Captured output:

    <PROBE_STDERR first 20 lines>

Fix: vision-upgrade --force
```

### When not installed:

```
## VisionSqueezer not found

Install:
  npx vision-squeezer install
```

## Notes

- The MCP probe is the **only** authoritative health signal. Registration without successful
  probe is a broken install — surface the stderr so the user (or [[vision-upgrade]]) can act.
- `npx-pinned` is the new default — registered as `npx -y vision-squeezer@X.Y.Z`. The pinned
  version is the effective version; it does **not** auto-update to npm latest.
- `npx-unpinned` exists for users on old installs that pre-date the pin change. Recommend
  `vision-upgrade` to migrate them to the pinned scheme.
- For Codex/Qwen users, replace `claude mcp list` with `codex mcp list` / `qwen mcp list`.

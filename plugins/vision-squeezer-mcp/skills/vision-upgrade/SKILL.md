---
name: vision-upgrade
description: >
  Upgrade VisionSqueezer to the latest version. Detects install method (cargo, npm global, npx)
  and runs the correct update command. Also flushes the npx cache and re-registers the MCP server
  with a pinned version so cached stale tarballs cannot freeze users on old releases. Use when
  user says "vision-upgrade", "upgrade vision-squeezer", "update vision-squeezer", or
  "/vision-upgrade".
allowed-tools: Bash
---

# vision-upgrade — VisionSqueezer Upgrade Skill

Detects install method, flushes npx cache, re-registers MCP with a pinned version, and verifies
the MCP can spawn after upgrade. Single self-healing pass.

## Why this skill is non-trivial

`npx -y vision-squeezer` caches the resolved tarball in `~/.npm/_npx`. After a release bump,
the cache **does not refresh**; users keep running the old version even though `npm view` shows
the new one. This was the root cause behind multiple "MCP failed to connect" reports.

The fix is two-layered:

1. The installer now registers the MCP with an explicit version: `npx -y vision-squeezer@X.Y.Z`.
   Cache busts automatically on every upgrade.
2. This skill nukes `~/.npm/_npx` and re-registers MCP with the new pinned version, so users on
   old (unpinned) registrations get migrated to the pinned scheme.

## Trigger

`/vision-upgrade` or any of: "vision upgrade", "upgrade vision-squeezer", "update vision-squeezer",
"install latest vision-squeezer", "fix vision-squeezer", "vision-squeezer broken — fix it".

Pass `--force` (or user says "force"/"reset"/"clean") to:
- Wipe the npx cache directory entirely.
- Force-remove and re-add the MCP registration in every Claude scope.
- Run the binary install command with `--force`.

## Action

### Step 1 — Detect state

```bash
LATEST=$(npm view vision-squeezer version 2>/dev/null)
NPM_GLOBAL=$(npm list -g vision-squeezer --depth=0 2>/dev/null | grep vision-squeezer | head -1)
CARGO_BIN=""
[ -x "$HOME/.cargo/bin/vision-squeezer" ] && CARGO_BIN="$HOME/.cargo/bin/vision-squeezer"
MCP_LINE=$(claude mcp list 2>/dev/null | grep -i vision-squeezer | head -1)
# Parse the pinned version out of `vision-squeezer@X.Y.Z` if present:
MCP_PINNED=$(echo "$MCP_LINE" | grep -oE 'vision-squeezer@[0-9]+\.[0-9]+\.[0-9]+' | sed 's/vision-squeezer@//')
echo "LATEST=$LATEST"
echo "NPM_GLOBAL=$NPM_GLOBAL"
echo "CARGO_BIN=$CARGO_BIN"
echo "MCP_LINE=$MCP_LINE"
echo "MCP_PINNED=$MCP_PINNED"
```

Pick install method by priority (first that matches wins):

- `CARGO_BIN` non-empty → **cargo**
- `NPM_GLOBAL` non-empty → **npm-global**
- `MCP_LINE` non-empty → **npx** (pinned if `MCP_PINNED` non-empty, else unpinned)
- otherwise → **not-installed** — tell user to run `npx vision-squeezer install`

### Step 2 — Run the matching upgrade command

**cargo**:
```bash
cargo install vision-squeezer --force
```

**npm-global**:
```bash
npm install -g vision-squeezer@latest
```

**npx (pinned or unpinned)**:
No binary install needed — `npx -y vision-squeezer@$LATEST` will fetch it. Skip to Step 3.

### Step 3 — Flush npx cache

Always, regardless of install method (npx cache may have a stale build that another tool spawns):

```bash
rm -rf "$HOME/.npm/_npx"
```

### Step 4 — Re-register MCP with pinned latest version

If `MCP_LINE` was found in Step 1, re-register so the registered command points at the new version:

```bash
# Capture existing scope before remove. `claude mcp list -s user|local|project` outputs
# the registration only if scope matches; the first non-empty scope wins.
SCOPE=""
for s in user local project; do
  if claude mcp list -s "$s" 2>/dev/null | grep -q vision-squeezer; then SCOPE="$s"; break; fi
done
[ -z "$SCOPE" ] && SCOPE="user"  # safest default

claude mcp remove vision-squeezer -s "$SCOPE" 2>/dev/null
claude mcp add -s "$SCOPE" vision-squeezer -- npx -y "vision-squeezer@${LATEST}"
```

For `--force` mode, also remove from the other two scopes (in case the user had duplicates):
```bash
for s in user local project; do
  [ "$s" = "$SCOPE" ] && continue
  claude mcp remove vision-squeezer -s "$s" 2>/dev/null || true
done
```

### Step 5 — Verify MCP can spawn

Probe the registered command — give it 5s to print a JSON-RPC ready signal on stdin/stdout.

```bash
# Portable 8s probe — macOS lacks GNU `timeout`, so use python3.
# cwd=$HOME because npx checks the cwd's package.json — if the user happens to
# be inside the vision-squeezer project dir (name collision), npx skips the
# install and the probe false-negatives with "command not found".
PROBE=$(python3 - "$LATEST" "$HOME" <<'PY' 2>&1
import subprocess, sys
ver = sys.argv[1]
home = sys.argv[2]
req = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vision-upgrade","version":"1"}}}\n'
try:
    r = subprocess.run(['npx', '-y', f'vision-squeezer@{ver}'], input=req, capture_output=True, text=True, timeout=8, cwd=home)
    if r.stdout:
        print(r.stdout.splitlines()[0])
    if r.stderr:
        print(r.stderr[:2000], file=sys.stderr)
except subprocess.TimeoutExpired as e:
    print("PROBE_TIMEOUT after 8s", file=sys.stderr)
PY
)
echo "$PROBE" | head -1 | grep -q '"jsonrpc"' && echo "PROBE=ok" || echo "PROBE=fail: $PROBE"
```

If `PROBE=fail`, surface the captured output to the user and suggest:
- `vision-upgrade --force` (clear cache, re-register from scratch).
- Report the captured stderr verbatim — most failures are postinstall download errors
  (HTTP 403 / 404 / network), and the message tells the user what to do.

## Output format

```
## VisionSqueezer Upgrade

- [x] Latest (npm): <LATEST>
- [x] Detected install: <cargo / npm-global / npx-pinned / npx-unpinned>
- [x] Previous version: <MCP_PINNED or "unknown (unpinned npx)">
- [x] Ran upgrade: <cargo install / npm install -g / skipped (npx)>
- [x] Flushed npx cache: ~/.npm/_npx
- [x] Re-registered MCP: vision-squeezer@<LATEST> (scope: <SCOPE>)
- [x] MCP probe: ✅ connected / ❌ <reason>
- [x] Status: ✅ Upgraded <OLD> → <LATEST>
```

If the probe failed:

```
- [ ] MCP probe: ❌ <captured-error>

Fix: vision-upgrade --force
```

## Notes

- **Do not** tell users "npx always pulls latest" — that statement is wrong now that the
  installer pins versions in the MCP registration. The pinned version freezes them until this
  skill re-registers.
- If `claude` CLI is missing, run only Steps 1–3 and tell the user to re-register manually
  with `npx vision-squeezer install`.
- For Codex/Qwen users, replace `claude mcp` with `codex mcp` / `qwen mcp` — same command shape.

---
name: vision-upgrade
description: >
  Upgrade VisionSqueezer to the latest version. Detects install method (cargo, npm global, npx)
  and runs the correct update command. Use when user says "vision-upgrade", "upgrade vision-squeezer",
  "update vision-squeezer", or "/vision-upgrade".
allowed-tools: Bash
---

# vision-upgrade — VisionSqueezer Upgrade Skill

Detects install method and upgrades to latest.

## Trigger

`/vision-upgrade` or any of: "vision upgrade", "upgrade vision-squeezer", "update vision-squeezer", "install latest vision-squeezer"

## Action

Run the following detection script first:

```bash
BIN=$(command -v vision-squeezer 2>/dev/null)
[ -z "$BIN" ] && [ -x "$HOME/.cargo/bin/vision-squeezer" ] && BIN="$HOME/.cargo/bin/vision-squeezer"
INSTALLED=""
[ -n "$BIN" ] && [ -x "$BIN" ] && INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
LATEST=$(npm view vision-squeezer version 2>/dev/null)
NPM_GLOBAL=$(npm list -g vision-squeezer --depth=0 2>/dev/null | grep vision-squeezer | head -1)
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "LATEST=$LATEST"
echo "NPM_GLOBAL=$NPM_GLOBAL"
```

### Then run the appropriate upgrade command:

**If `NPM_GLOBAL` non-empty** (npm global install):
```bash
npm install -g vision-squeezer
```

**If `BIN` contains `.cargo`** (cargo install):
```bash
cargo install vision-squeezer
```

**If `BIN` empty** (npx user):
No action needed — npx always pulls latest. Confirm to user.

### After upgrade, verify:
```bash
vision-squeezer --version 2>/dev/null || ~/.cargo/bin/vision-squeezer --version 2>/dev/null
```

## Output format

```
## VisionSqueezer Upgrade

- [ ] Detected install method: <cargo / npm global / npx>
- [ ] Version before: v<INSTALLED or "n/a">
- [ ] Running upgrade...
- [ ] Version after: v<NEW_VERSION>
- [ ] Status: ✅ Updated to v<LATEST> / ✅ Already on latest (npx)
```

## Notes

- npx users: always on latest, no upgrade needed — tell them explicitly
- If cargo install fails (no Rust): suggest switching to npx with `claude mcp add vision-squeezer -- npx -y vision-squeezer`

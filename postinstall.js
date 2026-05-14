#!/usr/bin/env node
'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');

const pkg = require('./package.json');
const version = pkg.version;
const REPO = 'eralpozcan/vision-squeezer';
const BIN_DIR = path.join(__dirname, 'bin');

function getAssetName() {
  const p = process.platform;
  const a = process.arch;
  if (p === 'darwin' && a === 'arm64') return 'vision-squeezer-mcp-macos-arm64';
  if (p === 'linux'  && a === 'x64')  return 'vision-squeezer-mcp-linux-x86_64';
  if (p === 'linux'  && a === 'arm64') return 'vision-squeezer-mcp-linux-arm64';
  if (p === 'win32'  && a === 'x64')  return 'vision-squeezer-mcp-windows-x86_64.exe';
  throw new Error(`Unsupported platform: ${p}/${a}. Build from source: cargo install vision-squeezer`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const follow = (u) => {
      https.get(u, { headers: { 'User-Agent': 'vision-squeezer-postinstall' } }, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          return follow(res.headers.location);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`Download failed: HTTP ${res.statusCode} — ${u}`));
        }
        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
        file.on('error', reject);
      }).on('error', reject);
    };
    follow(url);
  });
}

async function main() {
  const asset = getAssetName();
  const url = `https://github.com/${REPO}/releases/download/v${version}/${asset}`;
  const ext = process.platform === 'win32' ? '.exe' : '';
  const dest = path.join(BIN_DIR, `vision-squeezer-mcp${ext}`);

  fs.mkdirSync(BIN_DIR, { recursive: true });

  process.stdout.write(`[vision-squeezer] Downloading ${asset}...`);
  await download(url, dest);
  if (process.platform !== 'win32') fs.chmodSync(dest, 0o755);
  console.log(' done.');
}

function installSkills() {
  const os = require('os');
  const skillsBase = path.join(os.homedir(), '.claude', 'skills');

  const skills = {
    'vision-stats': `---
name: vision-stats
description: >
  Show VisionSqueezer cumulative token & byte savings analytics. Zero MCP
  overhead — reads directly from local stats.db via CLI binary. Use when user
  says "vision-stats", "squeeze stats", "token savings", "how much saved",
  "vision-squeezer stats", "optimization history", or "/vision-stats".
allowed-tools: Bash
---

# vision-stats — VisionSqueezer Analytics Skill

Zero-overhead stats. Calls \`vision-squeezer stats\` directly — no MCP round-trip.

## Trigger

\`/vision-stats\` or any of: "vision stats", "squeeze stats", "show savings", "how much have I saved", "optimization stats"

## Action

Run this binary resolution chain, stop at first success:

\`\`\`bash
vision-squeezer stats 2>/dev/null \\
  || ~/.cargo/bin/vision-squeezer stats 2>/dev/null \\
  || "$(dirname "$(command -v vision-squeezer-mcp 2>/dev/null)")/vision-squeezer" stats 2>/dev/null \\
  || find "$HOME/.cargo/bin" "$HOME/Desktop" "$HOME/Projects" -maxdepth 6 -name "vision-squeezer" -not -path "*/deps/*" -not -path "*/debug/*" 2>/dev/null | head -1 | xargs -I{} {} stats 2>/dev/null \\
  || echo "vision-squeezer not found. Install: cargo install vision-squeezer"
\`\`\`

Print output verbatim. No wrapping, no commentary, no interpretation.

## Error handling

Binary not found → tell user to run \`cargo install vision-squeezer\` or \`eval "$(vision-squeezer setup-hook)"\` after install.

## Notes

- Stats persist in local stats.db on the user's machine
- MCP tool \`get_savings_stats\` does the same but costs ~150 tokens overhead — use this skill instead
`,
    'vision-doctor': `---
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

\`/vision-doctor\` or any of: "vision doctor", "check vision-squeezer", "update vision-squeezer",
"is vision-squeezer up to date", "upgrade vision-squeezer", "vision-squeezer version"

## Action

Run the following shell script and display the formatted checklist result:

\`\`\`bash
BIN=$(command -v vision-squeezer 2>/dev/null || echo ~/.cargo/bin/vision-squeezer)
if [ -x "$BIN" ]; then
  INSTALLED=$("$BIN" --version 2>/dev/null | grep -oE '[0-9]+\\.[0-9]+\\.[0-9]+' | head -1)
else
  INSTALLED=""
fi
LATEST=$(npm view vision-squeezer version 2>/dev/null)
echo "BIN=$BIN"
echo "INSTALLED=$INSTALLED"
echo "LATEST=$LATEST"
\`\`\`

## Output format

Display as a markdown checklist based on the values:

\`\`\`
## VisionSqueezer Doctor

- [x/] Binary found: <path or "not found">
- [x/] Installed version: <version or "unknown">
- [x/] Latest version (npm): <version or "unavailable">
- [x/] Status: Up to date / Update available / Not installed
\`\`\`

Use \`[x]\` for OK/pass, \`[ ]\` for missing/fail.

### If update available (\`INSTALLED\` != \`LATEST\` and both non-empty):

\`\`\`
## Update available: v<INSTALLED> → v<LATEST>

Via cargo:
  cargo install vision-squeezer

Via npm global:
  npm install -g vision-squeezer

Via npx: no action needed — always pulls latest automatically.
\`\`\`

### If not installed:

\`\`\`
## VisionSqueezer not found

Install via Claude Code (one-liner):
  claude mcp add vision-squeezer -- npx -y vision-squeezer

Or via cargo:
  cargo install vision-squeezer
\`\`\`

## Notes

- \`npx -y vision-squeezer\` users are always on latest — no update needed
- cargo install users must run \`cargo install vision-squeezer\` to upgrade
- npm global users run \`npm install -g vision-squeezer\`
`,
  };

  for (const [name, content] of Object.entries(skills)) {
    const skillDir = path.join(skillsBase, name);
    const skillFile = path.join(skillDir, 'SKILL.md');
    if (!fs.existsSync(skillFile)) {
      fs.mkdirSync(skillDir, { recursive: true });
      fs.writeFileSync(skillFile, content, 'utf8');
      console.log(`[vision-squeezer] /${name} skill installed → ${skillFile}`);
    }
  }
}

main().catch((err) => {
  console.error(`\n[vision-squeezer] postinstall failed: ${err.message}`);
  console.error('Install manually: cargo install vision-squeezer');
  process.exit(0); // non-fatal — don't block npm install
});

installSkills();

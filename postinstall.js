#!/usr/bin/env node
'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');

const pkg = require('./package.json');
const version = pkg.version;
const REPO = 'eralpozcan/vision-squeezer';
const BIN_DIR = path.join(__dirname, 'bin');
const BUNDLED_SKILLS_ROOT = path.join(__dirname, 'plugins', 'vision-squeezer-mcp', 'skills');
const USER_SKILLS_BASE = path.join(os.homedir(), '.claude', 'skills');

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

async function downloadBinary() {
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

// Sync user-level Claude skills from the bundled SKILL.md files shipped in this
// npm tarball. The bundled files are the same SKILL.md that ships in the Claude
// Code plugin marketplace — both come from the repo at release time, so the two
// install paths (npm + plugin marketplace) stay in lockstep.
//
// Previously this function hardcoded inline SKILL.md strings, which silently
// rotted across releases and produced the stale "Using npx — always latest"
// output that blocked multiple version bumps. Source-of-truth is now disk.
function syncUserLevelSkills() {
  if (!fs.existsSync(BUNDLED_SKILLS_ROOT)) {
    // Old tarball (pre-0.3.5) or trimmed install — nothing to copy.
    return;
  }

  const skillDirs = fs.readdirSync(BUNDLED_SKILLS_ROOT, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => d.name);

  for (const name of skillDirs) {
    const src = path.join(BUNDLED_SKILLS_ROOT, name, 'SKILL.md');
    if (!fs.existsSync(src)) continue;
    const dstDir = path.join(USER_SKILLS_BASE, name);
    const dstFile = path.join(dstDir, 'SKILL.md');
    fs.mkdirSync(dstDir, { recursive: true });
    fs.copyFileSync(src, dstFile);
    console.log(`[vision-squeezer] /${name} synced → ${dstFile}`);
  }
}

async function main() {
  await downloadBinary();
  syncUserLevelSkills();
}

main().catch((err) => {
  console.error(`\n[vision-squeezer] postinstall failed: ${err.message}`);
  console.error('Install manually: cargo install vision-squeezer');
  process.exit(0); // non-fatal — don't block npm install
});

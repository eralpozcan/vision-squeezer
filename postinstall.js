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
  if (p === 'darwin' && a === 'x64')  return 'vision-squeezer-mcp-macos-x86_64';
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

main().catch((err) => {
  console.error(`\n[vision-squeezer] postinstall failed: ${err.message}`);
  console.error('Install manually: cargo install vision-squeezer');
  process.exit(0); // non-fatal — don't block npm install
});

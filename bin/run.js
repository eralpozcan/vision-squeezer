#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const path = require('path');

const ext = process.platform === 'win32' ? '.exe' : '';
const bin = path.join(__dirname, `vision-squeezer-mcp${ext}`);

if (!require('fs').existsSync(bin)) {
  console.error(`[vision-squeezer] Binary not found: ${bin}`);
  console.error('Re-install: npm install vision-squeezer');
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: true,
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);

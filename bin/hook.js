#!/usr/bin/env node
'use strict';

/**
 * Claude Code PreToolUse hook (matcher: Read). When Claude reads an image from
 * the project, hand it the token-optimized copy instead of the original.
 *
 * Any problem (not an image, outside the project, binary missing, no saving)
 * prints nothing and exits 0, so Claude reads the original file unchanged.
 * Files added with `@` and pasted images never reach a Read call, so they are
 * not covered.
 */

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const IMAGE_EXT = /\.(png|jpe?g|webp|gif)$/i;

function passthrough() {
  process.exit(0);
}

function optimize(input) {
  const file = input.tool_input && input.tool_input.file_path;
  if (input.tool_name !== 'Read' || typeof file !== 'string' || !IMAGE_EXT.test(file)) return;

  // We answer with "allow", which skips the permission prompt. Only rewrite what
  // a plain Read would open without asking: a file inside the project. realpath
  // stops a symlink from pointing us at something outside it.
  const cwd = fs.realpathSync(input.cwd || process.cwd());
  const abs = fs.realpathSync(path.resolve(cwd, file));
  const rel = path.relative(cwd, abs);
  if (rel.startsWith('..') || path.isAbsolute(rel)) return;

  const bin = path.join(__dirname, `vision-squeezer-mcp${process.platform === 'win32' ? '.exe' : ''}`);
  const r = spawnSync(bin, ['optimize', abs], { encoding: 'utf8', timeout: 30000, windowsHide: true });
  if (r.status !== 0) return;

  const report = JSON.parse(r.stdout);
  if (!report.output_path || !(report.tokens_after < report.tokens_before)) return;

  const note = `vision-squeezer: ${report.tokens_before} → ${report.tokens_after} image tokens`;
  return {
    // systemMessage is the line the user sees; the reason alone is not shown for "allow".
    systemMessage: note,
    hookSpecificOutput: {
      hookEventName: 'PreToolUse',
      permissionDecision: 'allow',
      permissionDecisionReason: note,
      updatedInput: { ...input.tool_input, file_path: report.output_path },
    },
  };
}

let raw = '';
process.stdin.on('data', (d) => (raw += d));
process.stdin.on('end', () => {
  try {
    const out = optimize(JSON.parse(raw));
    if (out) process.stdout.write(JSON.stringify(out));
  } catch {
    // fall through: no output means "no change"
  }
  passthrough();
});

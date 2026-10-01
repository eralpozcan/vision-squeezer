#!/usr/bin/env node
'use strict';

/**
 * Image-read hook. When an agent reads an image from the project, hand it the
 * token-optimized copy instead of the original.
 *
 *   npx vision-squeezer hook [--client claude|cursor|gemini|opencode]
 *
 * Each client has its own stdin/stdout contract (ADAPTERS below); the core is shared.
 * Any problem (not an image, outside the project, binary missing, no saving)
 * answers "no change", so the agent reads the original file. Files attached with
 * `@` and pasted images never reach a read tool, so they are not covered.
 */

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const IMAGE_EXT = /\.(png|jpe?g|webp|gif)$/i;
const CACHE_DIR = '.vision-squeezer';

const str = (v) => (typeof v === 'string' ? v : undefined);
const note = (r) => `vision-squeezer: ${r.tokens_before} → ${r.tokens_after} image tokens`;

// `file`: the image path inside the client's tool input (and the key it lives under).
// `outDir`: where the copy goes. Claude Code can read the OS temp dir (checked live);
//   the other clients restrict reads to the workspace, so their copy lives in a
//   self-ignoring .vision-squeezer/ folder inside the project.
// `pass`: what to print for "no change". Cursor treats empty or invalid output as a
//   block, so it needs a real JSON answer; Claude Code takes silence.
const ADAPTERS = {
  claude: {
    target: (i) => (i.tool_name === 'Read' ? { key: 'file_path', file: str(i.tool_input && i.tool_input.file_path) } : null),
    cwd: (i) => i.cwd,
    outDir: null,
    pass: null,
    respond: (i, key, out, r) => ({
      // systemMessage is the line the user sees; the reason alone is not shown for "allow".
      systemMessage: note(r),
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'allow',
        permissionDecisionReason: note(r),
        updatedInput: { ...i.tool_input, [key]: out },
      },
    }),
  },
  cursor: {
    target: (i) => {
      if (i.tool_name !== 'Read') return null;
      const ti = i.tool_input || {};
      const key = ['file_path', 'path', 'target_file', 'filePath'].find((k) => IMAGE_EXT.test(str(ti[k]) || ''));
      return key ? { key, file: ti[key] } : null;
    },
    cwd: (i) => i.cwd || (Array.isArray(i.workspace_roots) ? i.workspace_roots[0] : undefined),
    outDir: CACHE_DIR,
    pass: { permission: 'allow' },
    respond: (i, key, out) => ({ permission: 'allow', updated_input: { ...i.tool_input, [key]: out } }),
  },
  gemini: {
    target: (i) => (i.tool_name === 'read_file' ? { key: 'file_path', file: str(i.tool_input && i.tool_input.file_path) } : null),
    cwd: (i) => i.cwd || process.env.GEMINI_PROJECT_DIR,
    outDir: CACHE_DIR,
    pass: {},
    // hookSpecificOutput.tool_input merges into, and overrides, the model's arguments.
    respond: (i, key, out, r) => ({
      systemMessage: note(r),
      hookSpecificOutput: { hookEventName: 'BeforeTool', tool_input: { [key]: out } },
    }),
  },
  opencode: {
    // Called by the generated plugin, which already filters on the `read` tool.
    target: (i) => ({ key: 'filePath', file: str(i.tool_input && i.tool_input.filePath) }),
    cwd: (i) => i.cwd,
    outDir: CACHE_DIR,
    pass: {},
    respond: (i, key, out) => ({ tool_input: { [key]: out } }),
  },
};

function clientName(argv) {
  const at = argv.indexOf('--client');
  return at >= 0 && argv[at + 1] ? argv[at + 1] : 'claude';
}

// Returns { out, report } for an image we should swap, or null for "no change".
function optimize(adapter, input) {
  const t = adapter.target(input);
  if (!t || !t.file || !IMAGE_EXT.test(t.file)) return null;

  // The answer can skip a permission prompt (Claude Code "allow") and the copy is
  // read in the file's place, so only rewrite what a plain read would open anyway:
  // a file inside the project. realpath stops a symlink pointing us outside it.
  const cwd = fs.realpathSync(adapter.cwd(input) || process.cwd());
  const abs = fs.realpathSync(path.resolve(cwd, t.file));
  const rel = path.relative(cwd, abs);
  if (rel.startsWith('..') || path.isAbsolute(rel)) return null;

  const args = ['optimize', abs];
  if (adapter.outDir) {
    const dir = path.join(cwd, adapter.outDir);
    fs.mkdirSync(dir, { recursive: true });
    const ignore = path.join(dir, '.gitignore');
    if (!fs.existsSync(ignore)) fs.writeFileSync(ignore, '*\n');
    args.push('--out-dir', dir);
  }
  const bin = path.join(__dirname, `vision-squeezer-mcp${process.platform === 'win32' ? '.exe' : ''}`);
  const r = spawnSync(bin, args, { encoding: 'utf8', timeout: 30000, windowsHide: true });
  if (r.status !== 0) return null;

  const report = JSON.parse(r.stdout);
  if (!report.output_path || !(report.tokens_after < report.tokens_before)) return null;
  return { key: t.key, out: report.output_path, report };
}

function run(raw, argv) {
  const adapter = ADAPTERS[clientName(argv)];
  if (!adapter) return null;
  let answer = adapter.pass;
  try {
    const input = JSON.parse(raw);
    const swap = optimize(adapter, input);
    if (swap) answer = adapter.respond(input, swap.key, swap.out, swap.report);
  } catch {
    // fall through to "no change"
  }
  return answer;
}

module.exports = { run, ADAPTERS };

if (require.main === module || process.argv[2] === 'hook') {
  let raw = '';
  process.stdin.on('data', (d) => (raw += d));
  process.stdin.on('end', () => {
    const answer = run(raw, process.argv.slice(2));
    if (answer) process.stdout.write(JSON.stringify(answer));
    process.exit(0);
  });
}

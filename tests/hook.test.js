'use strict';

// Run: cargo build && node --test tests/hook.test.js
const test = require('node:test');
const assert = require('node:assert');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.join(__dirname, '..');
const EXT = process.platform === 'win32' ? '.exe' : '';
const BUILT = ['release', 'debug']
  .map((p) => path.join(ROOT, 'target', p, `vision-squeezer-mcp${EXT}`))
  .find((p) => fs.existsSync(p));
const skip = BUILT ? false : 'build the binary first: cargo build';

// Lay out a package-like dir (hook.js next to the binary), as in the npm package.
const pkg = fs.mkdtempSync(path.join(os.tmpdir(), 'vs-hook-pkg-'));
if (BUILT) {
  fs.copyFileSync(path.join(ROOT, 'bin', 'hook.js'), path.join(pkg, 'hook.js'));
  fs.copyFileSync(BUILT, path.join(pkg, `vision-squeezer-mcp${EXT}`));
  fs.chmodSync(path.join(pkg, `vision-squeezer-mcp${EXT}`), 0o755);
}

const BIG = path.join(ROOT, 'data', 'istanbul.jpg'); // 2400x1670 photo
const TINY_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
  'base64',
);

function project() {
  return fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'vs-hook-proj-')));
}
function runHook(input, raw) {
  const r = spawnSync(process.execPath, [path.join(pkg, 'hook.js')], {
    input: raw !== undefined ? raw : JSON.stringify(input),
    encoding: 'utf8',
    env: { ...process.env, HOME: pkg },
  });
  return { status: r.status, out: r.stdout.trim() ? JSON.parse(r.stdout) : null };
}
const read = (cwd, file) => ({ tool_name: 'Read', cwd, tool_input: { file_path: file } });

test('an image inside the project is rewritten to the optimized copy', { skip }, () => {
  const cwd = project();
  fs.copyFileSync(BIG, path.join(cwd, 'shot.jpg'));
  const { status, out } = runHook(read(cwd, 'shot.jpg'));
  assert.strictEqual(status, 0);
  const o = out.hookSpecificOutput;
  assert.strictEqual(o.hookEventName, 'PreToolUse');
  assert.strictEqual(o.permissionDecision, 'allow');
  assert.match(o.permissionDecisionReason, /image tokens/);
  assert.match(out.systemMessage, /^vision-squeezer: \d+ → \d+ image tokens$/);
  assert.ok(fs.existsSync(o.updatedInput.file_path));
  assert.ok(fs.statSync(o.updatedInput.file_path).size < fs.statSync(BIG).size);
  assert.notStrictEqual(o.updatedInput.file_path, path.join(cwd, 'shot.jpg'));
});

test('other tool_input fields are kept', { skip }, () => {
  const cwd = project();
  fs.copyFileSync(BIG, path.join(cwd, 'a.png'));
  const input = read(cwd, path.join(cwd, 'a.png'));
  input.tool_input.offset = 3;
  const { out } = runHook(input);
  assert.strictEqual(out.hookSpecificOutput.updatedInput.offset, 3);
});

test('a non-image file is left alone', { skip }, () => {
  const cwd = project();
  fs.writeFileSync(path.join(cwd, 'notes.txt'), 'hi');
  assert.strictEqual(runHook(read(cwd, 'notes.txt')).out, null);
});

test('a tool other than Read is left alone', { skip }, () => {
  const cwd = project();
  fs.copyFileSync(BIG, path.join(cwd, 'a.png'));
  const input = read(cwd, 'a.png');
  input.tool_name = 'Write';
  assert.strictEqual(runHook(input).out, null);
});

test('an image outside the project is left alone (we answer with allow)', { skip }, () => {
  const cwd = project();
  const other = project();
  fs.copyFileSync(BIG, path.join(other, 'secret.png'));
  assert.strictEqual(runHook(read(cwd, path.join(other, 'secret.png'))).out, null);
});

test('a symlink inside the project pointing outside is left alone', { skip: skip || process.platform === 'win32' }, () => {
  const cwd = project();
  const other = project();
  fs.copyFileSync(BIG, path.join(other, 'secret.png'));
  fs.symlinkSync(path.join(other, 'secret.png'), path.join(cwd, 'link.png'));
  assert.strictEqual(runHook(read(cwd, 'link.png')).out, null);
});

test('no token saving means no rewrite', { skip }, () => {
  const cwd = project();
  fs.writeFileSync(path.join(cwd, 'tiny.png'), TINY_PNG);
  assert.strictEqual(runHook(read(cwd, 'tiny.png')).out, null);
});

test('a missing file and garbage input exit 0 silently', { skip }, () => {
  const cwd = project();
  assert.deepStrictEqual(runHook(read(cwd, 'nope.png')), { status: 0, out: null });
  assert.deepStrictEqual(runHook(null, 'not json'), { status: 0, out: null });
});

'use strict';

// Run: node --test tests/install.test.js
const test = require('node:test');
const assert = require('node:assert');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const INSTALL = path.join(__dirname, '..', 'bin', 'install.js');
const VERSION = require('../package.json').version;

function run(args, home, cwd) {
  return spawnSync(process.execPath, [INSTALL, ...args], {
    env: { ...process.env, HOME: home, USERPROFILE: home },
    cwd: cwd || home,
    encoding: 'utf8',
  });
}
const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'vs-install-'));
const read = (p) => JSON.parse(fs.readFileSync(p, 'utf8'));
const entry = { command: 'npx', args: ['-y', `vision-squeezer@${VERSION}`] };

test('cursor user scope creates ~/.cursor/mcp.json with a pinned entry', () => {
  const home = tmp();
  const r = run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(read(path.join(home, '.cursor', 'mcp.json')), { mcpServers: { 'vision-squeezer': entry } });
});

test('cursor project scope writes ./.cursor/mcp.json in the cwd', () => {
  const home = tmp();
  const proj = tmp();
  const r = run(['--client', 'cursor', '--scope', 'project', '--yes'], home, proj);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.ok(read(path.join(proj, '.cursor', 'mcp.json')).mcpServers['vision-squeezer']);
  assert.ok(!fs.existsSync(path.join(home, '.cursor')));
});

test('existing servers are kept and re-running is idempotent', () => {
  const home = tmp();
  const file = path.join(home, '.cursor', 'mcp.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify({ mcpServers: { other: { command: 'x' } }, extra: 1 }));
  run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  const r = run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(read(file), { mcpServers: { other: { command: 'x' }, 'vision-squeezer': entry }, extra: 1 });
});

test('a file that cannot be parsed is left untouched', () => {
  const home = tmp();
  const file = path.join(home, '.cursor', 'mcp.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const original = '{\n  // my servers\n  "mcpServers": {}\n}\n';
  fs.writeFileSync(file, original);
  const r = run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  assert.notStrictEqual(r.status, 0);
  assert.strictEqual(fs.readFileSync(file, 'utf8'), original);
});

test('windsurf has one fixed scope and needs no --scope', () => {
  const home = tmp();
  const r = run(['--client', 'windsurf', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.ok(read(path.join(home, '.codeium', 'windsurf', 'mcp_config.json')).mcpServers['vision-squeezer']);
});

test('a scope a client does not support is rejected', () => {
  const r = run(['--client', 'windsurf', '--scope', 'project', '--yes'], tmp());
  assert.notStrictEqual(r.status, 0);
  assert.match(r.stderr, /supports only: user/);
});

test('opencode keeps its own shape and $schema', () => {
  const home = tmp();
  const r = run(['--client', 'opencode', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(read(path.join(home, '.config', 'opencode', 'opencode.json')), {
    $schema: 'https://opencode.ai/config.json',
    mcp: { 'vision-squeezer': { type: 'local', command: ['npx', '-y', `vision-squeezer@${VERSION}`], enabled: true } },
  });
});

test('claude-desktop writes the platform path, or is unavailable on Linux', () => {
  const home = tmp();
  const r = run(['--client', 'claude-desktop', '--yes'], home);
  if (process.platform === 'darwin') {
    assert.strictEqual(r.status, 0, r.stderr);
    const file = path.join(home, 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json');
    assert.ok(read(file).mcpServers['vision-squeezer']);
  } else if (process.platform === 'linux') {
    assert.notStrictEqual(r.status, 0);
    assert.match(r.stderr, /not available on this platform/);
  }
});

test('unknown client lists the valid keys including the new ones', () => {
  const r = run(['--client', 'nope', '--yes'], tmp());
  assert.notStrictEqual(r.status, 0);
  for (const k of ['cursor', 'windsurf', 'claude-desktop', 'vscode']) assert.match(r.stderr, new RegExp(k));
});

// CLI-backed clients: put a fake binary on PATH that records its argv.
function withFakeCli(name, fn) {
  const bin = tmp();
  const log = path.join(bin, `${name}.args`);
  const file = path.join(bin, name);
  fs.writeFileSync(file, `#!/bin/sh\nprintf '%s\\n' "$@" > "${log}"\n`, { mode: 0o755 });
  const r = spawnSync(process.execPath, [INSTALL, ...fn.args], {
    env: { ...process.env, HOME: tmp(), PATH: `${bin}${path.delimiter}${process.env.PATH}` },
    encoding: 'utf8',
  });
  return { r, argv: fs.existsSync(log) ? fs.readFileSync(log, 'utf8').trim().split('\n') : null };
}
const posix = { skip: process.platform === 'win32' };
const pinned = `vision-squeezer@${VERSION}`;

test('vscode runs `code --add-mcp` with a JSON server definition', posix, () => {
  const { r, argv } = withFakeCli('code', { args: ['--client', 'vscode', '--yes'] });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.strictEqual(argv[0], '--add-mcp');
  assert.deepStrictEqual(JSON.parse(argv[1]), { name: 'vision-squeezer', command: 'npx', args: ['-y', pinned] });
});

test('codex keeps the `mcp add` shape', posix, () => {
  const { r, argv } = withFakeCli('codex', { args: ['--client', 'codex', '--scope', 'user', '--yes'] });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(argv, ['mcp', 'add', '--scope', 'user', 'vision-squeezer', '--', 'npx', '-y', pinned]);
});

test('kimi never gets --scope', posix, () => {
  const { r, argv } = withFakeCli('kimi', { args: ['--client', 'kimi', '--yes'] });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(argv, ['mcp', 'add', 'vision-squeezer', '--', 'npx', '-y', pinned]);
});

test('claude local scope omits --scope', posix, () => {
  const { r, argv } = withFakeCli('claude', { args: ['--client', 'claude', '--method', 'mcp-add', '--scope', 'local', '--yes'] });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(argv, ['mcp', 'add', 'vision-squeezer', '--', 'npx', '-y', pinned]);
});

test('gemini rejects the local scope', posix, () => {
  const { r } = withFakeCli('gemini', { args: ['--client', 'gemini', '--scope', 'local', '--yes'] });
  assert.notStrictEqual(r.status, 0);
  assert.match(r.stderr, /supports only: user, project/);
});

// Claude Code image-read hook, added next to `claude mcp add`.
function claudeInstall(extra, { home = tmp(), cwd } = {}) {
  const bin = tmp();
  fs.writeFileSync(path.join(bin, 'claude'), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
  const r = spawnSync(process.execPath, [INSTALL, '--client', 'claude', '--method', 'mcp-add', '--yes', ...extra], {
    env: { ...process.env, HOME: home, PATH: `${bin}${path.delimiter}${process.env.PATH}` },
    cwd: cwd || home,
    encoding: 'utf8',
  });
  return { r, home };
}
const HOOK_CMD = `npx -y vision-squeezer@${VERSION} hook`;
const ours = (settings) => settings.hooks.PreToolUse.filter((e) => e.hooks.some((h) => h.command.endsWith(' hook')));

test('claude mcp-add also adds the image-read hook (user scope)', posix, () => {
  const { r, home } = claudeInstall(['--scope', 'user']);
  assert.strictEqual(r.status, 0, r.stderr);
  const s = read(path.join(home, '.claude', 'settings.json'));
  const [entry] = ours(s);
  assert.strictEqual(entry.matcher, 'Read');
  assert.deepStrictEqual(entry.hooks.map((h) => h.if), ['Read(*.png)', 'Read(*.jpg)', 'Read(*.jpeg)', 'Read(*.webp)', 'Read(*.gif)']);
  assert.ok(entry.hooks.every((h) => h.type === 'command' && h.command === HOOK_CMD));
});

test('the hook is merged: other settings and hooks kept, re-run does not duplicate, old version replaced', posix, () => {
  const home = tmp();
  const file = path.join(home, '.claude', 'settings.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const other = { matcher: 'Bash', hooks: [{ type: 'command', command: 'echo hi' }] };
  const old = { matcher: 'Read', hooks: [{ type: 'command', if: 'Read(*.png)', command: 'npx -y vision-squeezer@0.0.1 hook' }] };
  fs.writeFileSync(file, JSON.stringify({ model: 'x', hooks: { PreToolUse: [other, old], Stop: [] } }));
  claudeInstall(['--scope', 'user'], { home });
  const r = claudeInstall(['--scope', 'user'], { home }).r;
  assert.strictEqual(r.status, 0, r.stderr);
  const s = read(file);
  assert.strictEqual(s.model, 'x');
  assert.deepStrictEqual(s.hooks.Stop, []);
  assert.strictEqual(s.hooks.PreToolUse.length, 2);
  assert.deepStrictEqual(s.hooks.PreToolUse[0], other);
  assert.ok(s.hooks.PreToolUse[1].hooks.every((h) => h.command === HOOK_CMD));
});

test('--no-hook skips the hook', posix, () => {
  const { r, home } = claudeInstall(['--scope', 'user', '--no-hook']);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.ok(!fs.existsSync(path.join(home, '.claude', 'settings.json')));
});

test('project and local scopes write into the project .claude dir', posix, () => {
  const proj = tmp();
  claudeInstall(['--scope', 'project'], { cwd: proj });
  claudeInstall(['--scope', 'local'], { cwd: proj });
  assert.ok(ours(read(path.join(proj, '.claude', 'settings.json'))).length === 1);
  assert.ok(ours(read(path.join(proj, '.claude', 'settings.local.json'))).length === 1);
});

test('settings.json that cannot be parsed is left untouched and the install still succeeds', posix, () => {
  const home = tmp();
  const file = path.join(home, '.claude', 'settings.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, '{ // comment\n}');
  const { r } = claudeInstall(['--scope', 'user'], { home });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.strictEqual(fs.readFileSync(file, 'utf8'), '{ // comment\n}');
  assert.match(r.stderr, /cannot parse/);
});

test('the plugin ships the same hook, pinned to the package version', () => {
  const plugin = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'plugins', 'vision-squeezer-mcp', 'hooks', 'hooks.json'), 'utf8'));
  const [entry] = plugin.hooks.PreToolUse;
  assert.strictEqual(entry.matcher, 'Read');
  assert.ok(entry.hooks.every((h) => h.command === HOOK_CMD), 'bump hooks.json with the version');
});

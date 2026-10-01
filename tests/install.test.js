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

test('gemini rejects the local scope', posix, () => {
  const { r } = withFakeCli('gemini', { args: ['--client', 'gemini', '--scope', 'local', '--yes'] });
  assert.notStrictEqual(r.status, 0);
  assert.match(r.stderr, /supports only: user, project/);
});

// Claude Code: one install method, the plugin, driven through the `claude plugin` CLI.
function fakeClaude(env = {}, { scope = 'local', home = tmp(), cwd } = {}) {
  const bin = tmp();
  const log = path.join(bin, 'calls.log');
  fs.writeFileSync(
    path.join(bin, 'claude'),
    `#!/bin/sh
echo "$*" >> "${log}"
case "$*" in
  "plugin marketplace list") printf '%s' "$FAKE_MARKETPLACES" ;;
  "plugin list") printf '%s' "$FAKE_PLUGINS" ;;
  "mcp remove"*) exit "\${FAKE_MCP_REMOVE_STATUS:-1}" ;;
  "$FAKE_FAIL_ON"*) [ -n "$FAKE_FAIL_ON" ] && exit 7 ;;
esac
exit 0
`,
    { mode: 0o755 },
  );
  const r = spawnSync(process.execPath, [INSTALL, '--client', 'claude', '--scope', scope, '--yes'], {
    env: { ...process.env, HOME: home, PATH: `${bin}${path.delimiter}${process.env.PATH}`, ...env },
    cwd: cwd || home,
    encoding: 'utf8',
  });
  const calls = fs.existsSync(log) ? fs.readFileSync(log, 'utf8').trim().split('\n') : [];
  return { r, calls, home };
}
const TARGET = 'vision-squeezer-mcp@vision-squeezer';
const MARKETPLACE_LISTED = '  ❯ vision-squeezer\n    Source: GitHub (eralpozcan/vision-squeezer)\n';

test('claude installs the plugin: add the marketplace, then install, with the chosen scope', posix, () => {
  const { r, calls } = fakeClaude({}, { scope: 'user' });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(calls, [
    'mcp remove vision-squeezer --scope user',
    'plugin marketplace list',
    'plugin marketplace add eralpozcan/vision-squeezer',
    'plugin list',
    `plugin install ${TARGET} --scope user`,
  ]);
});

test('an already-added marketplace is updated, an already-installed plugin is updated', posix, () => {
  const { r, calls } = fakeClaude({
    FAKE_MARKETPLACES: MARKETPLACE_LISTED,
    FAKE_PLUGINS: `  ❯ ${TARGET}\n    Version: 0.8.0\n`,
  });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.ok(calls.includes('plugin marketplace update vision-squeezer'));
  assert.ok(calls.includes(`plugin update ${TARGET}`));
  assert.ok(!calls.some((c) => c.startsWith('plugin marketplace add') || c.startsWith('plugin install')));
});

test('a marketplace with a similar name does not count as ours', posix, () => {
  const { calls } = fakeClaude({ FAKE_MARKETPLACES: '  ❯ vision-squeezer-fork\n    Source: GitHub (x/y)\n' });
  assert.ok(calls.includes('plugin marketplace add eralpozcan/vision-squeezer'));
});

test('the older standalone MCP registration is removed when present', posix, () => {
  const { r } = fakeClaude({ FAKE_MCP_REMOVE_STATUS: '0' });
  assert.match(r.stdout, /Removed the older standalone MCP registration/);
});

test('the older settings.json hook is removed, other hooks and settings kept', posix, () => {
  const home = tmp();
  const file = path.join(home, '.claude', 'settings.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const other = { matcher: 'Bash', hooks: [{ type: 'command', command: 'echo hi' }] };
  const old = { matcher: 'Read', hooks: [{ type: 'command', if: 'Read(*.png)', command: 'npx -y vision-squeezer@0.7.1 hook' }] };
  fs.writeFileSync(file, JSON.stringify({ model: 'x', hooks: { PreToolUse: [other, old] } }));
  const { r } = fakeClaude({}, { scope: 'user', home });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.match(r.stdout, /Removed the older settings.json hook/);
  assert.deepStrictEqual(read(file), { model: 'x', hooks: { PreToolUse: [other] } });
});

test('removing our only hook leaves no empty hooks object behind', posix, () => {
  const home = tmp();
  const file = path.join(home, '.claude', 'settings.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const old = { matcher: 'Read', hooks: [{ type: 'command', command: 'npx -y vision-squeezer@0.7.1 hook' }] };
  fs.writeFileSync(file, JSON.stringify({ model: 'x', hooks: { PreToolUse: [old] } }));
  fakeClaude({}, { scope: 'user', home });
  assert.deepStrictEqual(read(file), { model: 'x' });
});

test('settings.json that cannot be parsed is left untouched', posix, () => {
  const home = tmp();
  const file = path.join(home, '.claude', 'settings.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, '{ // comment\n}');
  const { r } = fakeClaude({}, { scope: 'user', home });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.strictEqual(fs.readFileSync(file, 'utf8'), '{ // comment\n}');
});

test('a failing claude plugin command stops the install with its exit code', posix, () => {
  const { r, calls } = fakeClaude({ FAKE_FAIL_ON: 'plugin marketplace add' });
  assert.strictEqual(r.status, 7);
  assert.ok(!calls.some((c) => c.startsWith('plugin install')));
});

test('--method is accepted but ignored', posix, () => {
  const bin = tmp();
  fs.writeFileSync(path.join(bin, 'claude'), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
  const r = spawnSync(process.execPath, [INSTALL, '--client', 'claude', '--method', 'mcp-add', '--scope', 'user', '--yes'], {
    env: { ...process.env, HOME: tmp(), PATH: `${bin}${path.delimiter}${process.env.PATH}` },
    encoding: 'utf8',
  });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.match(r.stderr, /--method is no longer used/);
});

const HOOK_CMD = `npx -y vision-squeezer@${VERSION} hook`;

test('the plugin ships the same hook, pinned to the package version', () => {
  const plugin = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'plugins', 'vision-squeezer-mcp', 'hooks', 'hooks.json'), 'utf8'));
  const [entry] = plugin.hooks.PreToolUse;
  assert.strictEqual(entry.matcher, 'Read');
  assert.ok(entry.hooks.every((h) => h.command === HOOK_CMD), 'bump hooks.json with the version');
});

// ── Image-read hooks for Cursor, Gemini CLI and OpenCode ─────────────────────

const cmd = (client) => `npx -y vision-squeezer@${VERSION} hook --client ${client}`;

test('cursor install also writes the preToolUse hook next to mcp.json', () => {
  const home = tmp();
  const r = run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.deepStrictEqual(read(path.join(home, '.cursor', 'hooks.json')), {
    version: 1,
    hooks: { preToolUse: [{ command: cmd('cursor'), matcher: 'Read', timeout: 30 }] },
  });
});

test('cursor hook: other hooks kept, re-run replaces our entry instead of adding a second', () => {
  const home = tmp();
  const file = path.join(home, '.cursor', 'hooks.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const other = { command: './lint.sh', matcher: 'Shell' };
  const old = { command: 'npx -y vision-squeezer@0.7.1 hook --client cursor', matcher: 'Read', timeout: 30 };
  fs.writeFileSync(file, JSON.stringify({ version: 1, hooks: { preToolUse: [other, old], stop: [{ command: './done.sh' }] } }));
  run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  const r = run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  const d = read(file);
  assert.deepStrictEqual(d.hooks.preToolUse, [other, { command: cmd('cursor'), matcher: 'Read', timeout: 30 }]);
  assert.deepStrictEqual(d.hooks.stop, [{ command: './done.sh' }]);
});

test('cursor hooks.json that cannot be parsed is left untouched, the MCP entry is still written', () => {
  const home = tmp();
  const file = path.join(home, '.cursor', 'hooks.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, '{ // mine\n}');
  const r = run(['--client', 'cursor', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.strictEqual(fs.readFileSync(file, 'utf8'), '{ // mine\n}');
  assert.ok(read(path.join(home, '.cursor', 'mcp.json')).mcpServers['vision-squeezer']);
});

test('cursor project scope writes ./.cursor/hooks.json in the project', () => {
  const home = tmp();
  const proj = tmp();
  run(['--client', 'cursor', '--scope', 'project', '--yes'], home, proj);
  assert.ok(read(path.join(proj, '.cursor', 'hooks.json')).hooks.preToolUse.length === 1);
  assert.ok(!fs.existsSync(path.join(home, '.cursor', 'hooks.json')));
});

test('gemini install adds a BeforeTool hook on read_file to settings.json', posix, () => {
  const bin = tmp();
  fs.writeFileSync(path.join(bin, 'gemini'), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
  const home = tmp();
  const file = path.join(home, '.gemini', 'settings.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const other = { matcher: 'write_file', hooks: [{ name: 'guard', type: 'command', command: './guard.sh' }] };
  fs.writeFileSync(file, JSON.stringify({ theme: 'x', hooks: { BeforeTool: [other] } }));
  const env = { ...process.env, HOME: home, PATH: `${bin}${path.delimiter}${process.env.PATH}` };
  const go = () => spawnSync(process.execPath, [INSTALL, '--client', 'gemini', '--scope', 'user', '--yes'], { env, cwd: home, encoding: 'utf8' });
  go();
  const r = go();
  assert.strictEqual(r.status, 0, r.stderr);
  const d = read(file);
  assert.strictEqual(d.theme, 'x');
  assert.strictEqual(d.hooks.BeforeTool.length, 2);
  assert.deepStrictEqual(d.hooks.BeforeTool[0], other);
  assert.deepStrictEqual(d.hooks.BeforeTool[1], {
    matcher: 'read_file',
    hooks: [{ name: 'vision-squeezer', type: 'command', command: cmd('gemini'), timeout: 30000 }],
  });
});

test('opencode install writes a pinned plugin that rewrites read filePath', () => {
  const home = tmp();
  const r = run(['--client', 'opencode', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  const file = path.join(home, '.config', 'opencode', 'plugins', 'vision-squeezer.js');
  const src = fs.readFileSync(file, 'utf8');
  assert.match(src, /Generated by `npx vision-squeezer install --client opencode`/);
  assert.ok(src.includes(`"vision-squeezer@${VERSION}", "hook", "--client", "opencode"`));
  assert.ok(src.includes('"tool.execute.before"') && src.includes('input.tool !== "read"') && src.includes('output.args.filePath = swap.filePath'));
  // it is valid JavaScript (ES module syntax checked by compiling as an async function body is not possible, so parse it as a module)
  const check = spawnSync(process.execPath, ['--input-type=module', '--check'], { input: src, encoding: 'utf8' });
  assert.strictEqual(check.status, 0, check.stderr);
});

test('opencode plugin: re-run overwrites ours, a foreign file of the same name is left alone', () => {
  const home = tmp();
  const file = path.join(home, '.config', 'opencode', 'plugins', 'vision-squeezer.js');
  run(['--client', 'opencode', '--scope', 'user', '--yes'], home);
  run(['--client', 'opencode', '--scope', 'user', '--yes'], home);
  assert.match(fs.readFileSync(file, 'utf8'), /Generated by/);
  fs.writeFileSync(file, '// my own plugin\n');
  const r = run(['--client', 'opencode', '--scope', 'user', '--yes'], home);
  assert.strictEqual(r.status, 0, r.stderr);
  assert.strictEqual(fs.readFileSync(file, 'utf8'), '// my own plugin\n');
  assert.match(r.stderr, /is not ours/);
});

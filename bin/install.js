#!/usr/bin/env node
'use strict';

/**
 * Interactive MCP installer for VisionSqueezer.
 *
 *   npx vision-squeezer install            # prompts for client + scope
 *   npx vision-squeezer install --scope user
 *   npx vision-squeezer install --client claude --scope project --yes
 *   npx vision-squeezer install --client cursor --yes
 */

const { spawnSync } = require('child_process');
const readline = require('readline');
const path = require('path');
const fs = require('fs');
const os = require('os');

// Pinning to an exact version in the MCP command line busts the npx cache on
// every upgrade. Without it, `npx -y vision-squeezer` silently reuses an old
// cached tarball even after `npm install -g vision-squeezer@latest` — users
// then see "MCP failed to connect" with a stale binary they can't update.
const PKG_VERSION = require(path.join(__dirname, '..', 'package.json')).version;

const SCOPES = [
  {
    key: 'user',
    label: 'user',
    description: 'All projects on this machine (recommended)',
  },
  {
    key: 'local',
    label: 'local',
    description: 'This project only, private to you (Claude Code default)',
  },
  {
    key: 'project',
    label: 'project',
    description: 'Shared via .mcp.json checked into the repo',
  },
];

// `scopes` lists the scopes a client supports (omitted = all). `kind: 'json'`
// clients have no non-interactive CLI, so the installer edits their JSON config
// (see JSON_TARGETS). Zed is not here: its settings.json allows comments, so a
// rewrite could destroy the user's file.
const CLIENTS = [
  { key: 'claude', label: 'Claude Code', cli: 'claude' },
  { key: 'codex', label: 'Codex CLI', cli: 'codex' },
  { key: 'qwen', label: 'Qwen Code', cli: 'qwen' },
  { key: 'opencode', label: 'OpenCode', cli: 'opencode', kind: 'json', scopes: ['user', 'project'] },
  { key: 'gemini', label: 'Gemini CLI', cli: 'gemini', scopes: ['user', 'project'] },
  { key: 'kimi', label: 'Kimi CLI', cli: 'kimi', scopes: ['user'] },
  { key: 'cursor', label: 'Cursor', kind: 'json', scopes: ['user', 'project'] },
  { key: 'windsurf', label: 'Windsurf', kind: 'json', scopes: ['user'] },
  { key: 'claude-desktop', label: 'Claude Desktop', kind: 'json', scopes: ['user'] },
  { key: 'vscode', label: 'VS Code', cli: 'code', scopes: ['user'] },
];

// Clients without a usable `mcp add` (OpenCode's is interactive-only; Cursor,
// Windsurf and Claude Desktop have none) get their JSON config written directly.
const CONFIG_METHOD = { key: 'config-file', label: 'config file' };

const NPX_ARGS = ['-y', `vision-squeezer@${PKG_VERSION}`];
const stdioEntry = () => ({ command: 'npx', args: NPX_ARGS });

function claudeDesktopPath() {
  if (process.platform === 'darwin') {
    return path.join(os.homedir(), 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json');
  }
  if (process.platform === 'win32' && process.env.APPDATA) {
    return path.join(process.env.APPDATA, 'Claude', 'claude_desktop_config.json');
  }
  return null; // no Linux build of Claude Desktop
}

const JSON_TARGETS = {
  opencode: {
    file: (scope) => scope.key === 'user'
      ? path.join(os.homedir(), '.config', 'opencode', 'opencode.json')
      : path.join(process.cwd(), 'opencode.json'),
    root: 'mcp',
    entry: () => ({ type: 'local', command: ['npx', ...NPX_ARGS], enabled: true }),
    schema: 'https://opencode.ai/config.json',
  },
  cursor: {
    file: (scope) => scope.key === 'user'
      ? path.join(os.homedir(), '.cursor', 'mcp.json')
      : path.join(process.cwd(), '.cursor', 'mcp.json'),
    root: 'mcpServers',
    entry: stdioEntry,
  },
  windsurf: {
    file: () => path.join(os.homedir(), '.codeium', 'windsurf', 'mcp_config.json'),
    root: 'mcpServers',
    entry: stdioEntry,
  },
  'claude-desktop': {
    file: claudeDesktopPath,
    root: 'mcpServers',
    entry: stdioEntry,
  },
};

const MARKETPLACE_REPO = 'eralpozcan/vision-squeezer';
const PLUGIN_NAME = 'vision-squeezer-mcp';
const MARKETPLACE_NAME = 'vision-squeezer';

function parseArgs(argv) {
  const out = { scope: null, client: null, method: null, yes: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--scope' || a === '-s') out.scope = argv[++i];
    else if (a === '--client' || a === '-c') out.client = argv[++i];
    else if (a === '--method' || a === '-m') out.method = argv[++i];
    else if (a === '--yes' || a === '-y') out.yes = true;
    else if (a === '--help' || a === '-h') out.help = true;
  }
  return out;
}

function printHelp() {
  console.log(`vision-squeezer install — register the MCP server with an AI CLI

Usage:
  npx vision-squeezer install [options]

Options:
  -c, --client <name>   Target (claude | codex | qwen | opencode | gemini | kimi | cursor | windsurf | claude-desktop | vscode)
  -s, --scope <name>    Install scope (user | local | project)
  -y, --yes             Skip confirmation prompt
  -h, --help            Show this help

Claude Code:
  Installs the plugin, which bundles the MCP server, the /vision-stats,
  /vision-doctor and /vision-upgrade skills, and a hook that hands Claude the
  token-optimized copy of images it reads inside the project:
    claude plugin marketplace add eralpozcan/vision-squeezer
    claude plugin install vision-squeezer-mcp@vision-squeezer --scope <scope>
  An older standalone registration (claude mcp add) and its settings.json hook
  are removed first. Restart Claude Code or run /reload-plugins afterwards.
  Pasted images and @-files are not covered.

Scopes:
  user      All projects on this machine (recommended)
  local     This project only, private to you (default for Claude Code)
  project   Shared with the repo

OpenCode:
  \`opencode mcp add\` is interactive-only (no flags), so this installer
  writes the MCP entry directly into OpenCode's JSON config instead:
    user      ~/.config/opencode/opencode.json (global)
    project   ./opencode.json (repo root)
  OpenCode has no 'local' scope.
  Also writes a plugin that swaps image reads for the optimized copy:
    user      ~/.config/opencode/plugins/vision-squeezer.js
    project   ./.opencode/plugins/vision-squeezer.js

Gemini CLI:
  gemini mcp add --scope X vision-squeezer -- npx -y vision-squeezer@<version>
  Scope writes to ~/.gemini/settings.json (user) or .gemini/settings.json
  (project). Gemini CLI has no 'local' scope.
  Also adds an image-read hook (BeforeTool on read_file) to the same settings.json.

Kimi CLI:
  kimi mcp add vision-squeezer -- npx -y vision-squeezer@<version>
  Always writes the single global ~/.kimi/mcp.json — no scope flag exists.

Cursor:
  Adds an "mcpServers" entry to a JSON file (existing servers are kept):
    user      ~/.cursor/mcp.json
    project   ./.cursor/mcp.json
  Also adds an image-read hook (preToolUse on Read) to hooks.json next to it.

Windsurf:
  Adds an "mcpServers" entry to ~/.codeium/windsurf/mcp_config.json (user only).

Claude Desktop:
  Adds an "mcpServers" entry to claude_desktop_config.json (user only;
  macOS: ~/Library/Application Support/Claude, Windows: %APPDATA%\\Claude).

VS Code:
  code --add-mcp '{"name":"vision-squeezer","command":"npx","args":[...]}'
  Adds the server to your VS Code user profile (user only).

JSON config files that cannot be parsed (for example with comments) are left
untouched; add the entry by hand (see https://visionsqueezer.com/getting-started/mcp-setup).
`);
}

function prompt(rl, question) {
  return new Promise((resolve) => rl.question(question, (a) => resolve(a.trim())));
}

async function pickFromList(rl, label, items, fallbackKey) {
  console.log(`\n${label}:`);
  items.forEach((it, i) => {
    const desc = it.description ? `  — ${it.description}` : '';
    console.log(`  ${i + 1}) ${it.label}${desc}`);
  });
  const def = items.findIndex((it) => it.key === fallbackKey);
  const defLabel = def >= 0 ? def + 1 : 1;
  const ans = await prompt(rl, `Choice [${defLabel}]: `);
  if (!ans) return items[def >= 0 ? def : 0];
  const n = parseInt(ans, 10);
  if (Number.isFinite(n) && n >= 1 && n <= items.length) return items[n - 1];
  const byKey = items.find((it) => it.key === ans.toLowerCase());
  if (byKey) return byKey;
  console.error(`Invalid choice: ${ans}`);
  return pickFromList(rl, label, items, fallbackKey);
}

function commandExists(cmd) {
  const r = spawnSync(process.platform === 'win32' ? 'where' : 'which', [cmd], {
    stdio: 'ignore',
  });
  return r.status === 0;
}

// ── Image-read hooks for other clients ───────────────────────────────────────
//
// Cursor, Gemini CLI and OpenCode document a way to rewrite a file read before it
// runs, so `install` adds a hook next to the MCP entry. These follow the vendors'
// docs but are not yet checked inside the real apps. Failure is harmless: the
// hook answers "no change" and the agent reads the original file.

const hookCommand = (client) => `npx -y vision-squeezer@${PKG_VERSION} hook --client ${client}`;
const ourHookRe = (client) => new RegExp(`vision-squeezer@\\S+ hook --client ${client}$`);
const isOurCommand = (client, cmd) => typeof cmd === 'string' && ourHookRe(client).test(cmd);

// Edit a JSON file in place. A file that cannot be parsed is left untouched.
function editJson(file, mutate) {
  let data = {};
  if (fs.existsSync(file)) {
    try {
      data = JSON.parse(fs.readFileSync(file, 'utf8'));
    } catch (err) {
      console.error(`Could not add the image-read hook: cannot parse ${file} (${err.message}). Left untouched.`);
      return false;
    }
  } else {
    fs.mkdirSync(path.dirname(file), { recursive: true });
  }
  mutate(data);
  fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
  return true;
}

function reportHook(ok, file) {
  if (ok) console.log(`Added the image-read hook to ${file} (experimental: not yet verified inside the app).`);
}

function installCursorHook(scope) {
  const file = scope.key === 'user'
    ? path.join(os.homedir(), '.cursor', 'hooks.json')
    : path.join(process.cwd(), '.cursor', 'hooks.json');
  reportHook(editJson(file, (d) => {
    d.version = d.version || 1;
    d.hooks = d.hooks || {};
    const list = Array.isArray(d.hooks.preToolUse) ? d.hooks.preToolUse : [];
    d.hooks.preToolUse = [
      ...list.filter((e) => !isOurCommand('cursor', e && e.command)),
      { command: hookCommand('cursor'), matcher: 'Read', timeout: 30 },
    ];
  }), file);
}

function installGeminiHook(scope) {
  const file = scope.key === 'user'
    ? path.join(os.homedir(), '.gemini', 'settings.json')
    : path.join(process.cwd(), '.gemini', 'settings.json');
  reportHook(editJson(file, (d) => {
    d.hooks = d.hooks || {};
    const list = Array.isArray(d.hooks.BeforeTool) ? d.hooks.BeforeTool : [];
    const ours = (e) => Array.isArray(e && e.hooks) && e.hooks.some((h) => isOurCommand('gemini', h.command));
    d.hooks.BeforeTool = [
      ...list.filter((e) => !ours(e)),
      { matcher: 'read_file', hooks: [{ name: 'vision-squeezer', type: 'command', command: hookCommand('gemini'), timeout: 30000 }] },
    ];
  }), file);
}

const OPENCODE_PLUGIN_MARKER = '// Generated by `npx vision-squeezer install --client opencode`';

function opencodePlugin() {
  return [
    OPENCODE_PLUGIN_MARKER,
    '// Hands the agent a token-optimized copy of images it reads inside the project.',
    'import { spawnSync } from "node:child_process"',
    '',
    'const IMAGE = /\\.(png|jpe?g|webp|gif)$/i',
    '',
    'export const VisionSqueezer = async ({ directory }) => ({',
    '  "tool.execute.before": async (input, output) => {',
    '    if (input.tool !== "read") return',
    '    const file = output.args && output.args.filePath',
    '    if (typeof file !== "string" || !IMAGE.test(file)) return',
    `    const r = spawnSync("npx", ["-y", "vision-squeezer@${PKG_VERSION}", "hook", "--client", "opencode"], {`,
    '      input: JSON.stringify({ cwd: directory, tool_input: output.args }),',
    '      encoding: "utf8",',
    '      timeout: 30000,',
    '      shell: process.platform === "win32",',
    '    })',
    '    try {',
    '      const swap = JSON.parse(r.stdout || "{}").tool_input',
    '      if (swap && swap.filePath) output.args.filePath = swap.filePath',
    '    } catch {}',
    '  },',
    '})',
    '',
  ].join('\n');
}

function installOpenCodePlugin(scope) {
  const file = scope.key === 'user'
    ? path.join(os.homedir(), '.config', 'opencode', 'plugins', 'vision-squeezer.js')
    : path.join(process.cwd(), '.opencode', 'plugins', 'vision-squeezer.js');
  if (fs.existsSync(file) && !fs.readFileSync(file, 'utf8').startsWith(OPENCODE_PLUGIN_MARKER)) {
    console.error(`Could not add the image-read plugin: ${file} exists and is not ours. Left untouched.`);
    return;
  }
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, opencodePlugin());
  reportHook(true, file);
}

const HOOK_INSTALLERS = { cursor: installCursorHook, gemini: installGeminiHook, opencode: installOpenCodePlugin };

// ── Claude Code plugin ───────────────────────────────────────────────────────

// Older installers registered the hook in settings.json. The plugin ships it now,
// so a leftover copy would run twice.
function isOurHook(entry) {
  return Array.isArray(entry && entry.hooks)
    && entry.hooks.some((h) => typeof h.command === 'string' && /vision-squeezer@\S+ hook$/.test(h.command));
}

function claudeSettingsPath(scope) {
  if (scope.key === 'user') return path.join(os.homedir(), '.claude', 'settings.json');
  return path.join(process.cwd(), '.claude', scope.key === 'project' ? 'settings.json' : 'settings.local.json');
}

// Returns true when something was removed. A file that cannot be parsed is left alone.
function removeOldClaudeHook(scope) {
  const file = claudeSettingsPath(scope);
  if (!fs.existsSync(file)) return false;
  let settings;
  try {
    settings = JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return false;
  }
  const pre = settings.hooks && Array.isArray(settings.hooks.PreToolUse) ? settings.hooks.PreToolUse : [];
  const kept = pre.filter((e) => !isOurHook(e));
  if (kept.length === pre.length) return false;
  if (kept.length) settings.hooks.PreToolUse = kept;
  else delete settings.hooks.PreToolUse;
  if (!Object.keys(settings.hooks).length) delete settings.hooks;
  fs.writeFileSync(file, JSON.stringify(settings, null, 2) + '\n');
  return true;
}

function buildArgs(client, scope) {
  // codex/qwen/gemini/kimi accept the same shape: `<cli> mcp add [--scope X] NAME -- npx -y vision-squeezer@<version>`
  // `local` is the default for these CLIs — omit the flag.
  // Kimi CLI has no scope concept (single global ~/.kimi/mcp.json) — never pass --scope.
  if (client.key === 'vscode') {
    return ['--add-mcp', JSON.stringify({ name: 'vision-squeezer', ...stdioEntry() })];
  }
  const args = ['mcp', 'add'];
  if (client.key !== 'kimi' && scope.key !== 'local') {
    args.push('--scope', scope.key);
  }
  args.push('vision-squeezer', '--', 'npx', ...NPX_ARGS);
  return args;
}

async function main() {
  const argv = process.argv.slice(2);
  const opts = parseArgs(argv);
  if (opts.help) {
    printHelp();
    return 0;
  }

  console.log('VisionSqueezer MCP installer\n');

  let client;
  if (opts.client) {
    client = CLIENTS.find((c) => c.key === opts.client.toLowerCase());
    if (!client) {
      console.error(`Unknown client: ${opts.client}. Expected one of: ${CLIENTS.map((c) => c.key).join(', ')}`);
      return 1;
    }
  }
  let scope;
  if (opts.scope) {
    scope = SCOPES.find((s) => s.key === opts.scope.toLowerCase());
    if (!scope) {
      console.error(`Unknown scope: ${opts.scope}. Expected one of: ${SCOPES.map((s) => s.key).join(', ')}`);
      return 1;
    }
  }
  if (client && client.scopes && scope && !client.scopes.includes(scope.key)) {
    console.error(`${client.label} supports only: ${client.scopes.join(', ')} (got '${scope.key}').`);
    return 1;
  }

  if (opts.method) console.error('Note: --method is no longer used; Claude Code is installed as a plugin.');

  const singleScope = () => client && client.scopes && client.scopes.length === 1;
  const interactive = !client || (!singleScope() && !scope);
  let rl;
  if (interactive) {
    if (!process.stdin.isTTY) {
      console.error('Non-interactive shell detected. Pass --client (and --scope) explicitly.');
      printHelp();
      return 1;
    }
    rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  }

  try {
    if (!client) client = await pickFromList(rl, 'Pick a client', CLIENTS, 'claude');
    // A single supported scope (kimi, windsurf, claude-desktop, vscode) is not a choice.
    if (!scope && client.scopes && client.scopes.length === 1) {
      scope = SCOPES.find((s) => s.key === client.scopes[0]);
    }
    if (!scope) {
      const scopeList = client.scopes ? SCOPES.filter((s) => client.scopes.includes(s.key)) : SCOPES;
      scope = await pickFromList(rl, 'Pick install scope', scopeList, 'user');
    }

    if (client.kind !== 'json' && !commandExists(client.cli)) {
      console.error(`\n'${client.cli}' CLI not found in PATH.`);
      console.error(`Install it first, then re-run: npx vision-squeezer install --client ${client.key} --scope ${scope.key}`);
      return 1;
    }

    if (client.key === 'claude') return await runClaudePluginInstall(rl, scope, opts.yes);
    if (client.kind === 'json') return await runJsonInstall(client, rl, scope, opts.yes);

    const args = buildArgs(client, scope);
    console.log(`\nWill run: ${client.cli} ${args.join(' ')}`);

    if (interactive && !opts.yes) {
      const confirm = await prompt(rl, 'Proceed? [Y/n]: ');
      if (confirm && !/^y(es)?$/i.test(confirm)) {
        console.log('Cancelled.');
        return 0;
      }
    }
    if (rl) rl.close();

    const result = spawnSync(client.cli, args, { stdio: 'inherit' });
    if (result.error) {
      console.error(result.error.message);
      return 1;
    }
    if ((result.status ?? 0) !== 0) {
      return result.status ?? 1;
    }
    console.log(`\nDone. VisionSqueezer registered with ${client.label} (scope: ${scope.key}).`);
    if (HOOK_INSTALLERS[client.key]) HOOK_INSTALLERS[client.key](scope);
    return 0;
  } finally {
    if (rl && !rl.closed) rl.close();
  }
}

async function runJsonInstall(client, rl, scope, yes) {
  const target = JSON_TARGETS[client.key];
  const configPath = target.file(scope);
  if (!configPath) {
    console.error(`${client.label} is not available on this platform.`);
    return 1;
  }
  console.log(`\nWill write MCP entry to: ${configPath}`);

  if (rl && !yes) {
    const confirm = await prompt(rl, 'Proceed? [Y/n]: ');
    if (confirm && !/^y(es)?$/i.test(confirm)) {
      console.log('Cancelled.');
      return 0;
    }
  }
  if (rl) rl.close();

  let config = {};
  if (fs.existsSync(configPath)) {
    try {
      config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    } catch (err) {
      console.error(`Cannot parse ${configPath}: ${err.message}`);
      console.error('Left untouched. It may contain comments; add the entry by hand (see --help).');
      return 1;
    }
  } else {
    fs.mkdirSync(path.dirname(configPath), { recursive: true });
    if (target.schema) config.$schema = target.schema;
  }
  config[target.root] = config[target.root] || {};
  config[target.root]['vision-squeezer'] = target.entry();
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');

  console.log(`\nDone. VisionSqueezer registered with ${client.label} (scope: ${scope.key}) at ${configPath}.`);
  if (HOOK_INSTALLERS[client.key]) HOOK_INSTALLERS[client.key](scope);
  if (client.key !== 'opencode') console.log(`Restart ${client.label} to load it.`);
  return 0;
}

const claudeRun = (args, opts = {}) => spawnSync('claude', args, { encoding: 'utf8', ...opts });

async function runClaudePluginInstall(rl, scope, yes) {
  const target = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
  console.log(`\nWill install the ${PLUGIN_NAME} plugin (MCP server + skills + image-read hook), scope: ${scope.key}:`);
  console.log(`  claude plugin marketplace add ${MARKETPLACE_REPO}   (update if already added)`);
  console.log(`  claude plugin install ${target} --scope ${scope.key}   (update if already installed)`);
  console.log('An older standalone registration and settings.json hook are removed first.');

  if (rl && !yes) {
    const confirm = await prompt(rl, 'Proceed? [Y/n]: ');
    if (confirm && !/^y(es)?$/i.test(confirm)) {
      console.log('Cancelled.');
      return 0;
    }
  }
  if (rl) rl.close();

  // Migrate from the old two-method installer: the plugin provides both again.
  if (claudeRun(['mcp', 'remove', 'vision-squeezer', '--scope', scope.key]).status === 0) {
    console.log('Removed the older standalone MCP registration.');
  }
  if (removeOldClaudeHook(scope)) console.log('Removed the older settings.json hook.');

  const run = (args) => {
    const r = claudeRun(args, { stdio: 'inherit' });
    if (r.error) console.error(r.error.message);
    return r.error ? 1 : (r.status ?? 0);
  };

  const marketplaces = claudeRun(['plugin', 'marketplace', 'list']).stdout || '';
  const marketplaceAdded = new RegExp(`(^|\\s)${MARKETPLACE_NAME}\\s*\\n\\s+Source:`).test(marketplaces);
  let code = run(marketplaceAdded
    ? ['plugin', 'marketplace', 'update', MARKETPLACE_NAME]
    : ['plugin', 'marketplace', 'add', MARKETPLACE_REPO]);
  if (code !== 0) return code;

  const installed = (claudeRun(['plugin', 'list']).stdout || '').includes(target);
  code = run(installed ? ['plugin', 'update', target] : ['plugin', 'install', target, '--scope', scope.key]);
  if (code !== 0) return code;

  console.log('\nDone. Restart Claude Code, or run /reload-plugins, to load the MCP server, skills and hook.');
  return 0;
}

main().then((code) => process.exit(code)).catch((err) => {
  console.error(err.message || err);
  process.exit(1);
});

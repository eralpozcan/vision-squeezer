#!/usr/bin/env node
'use strict';

/**
 * Interactive MCP installer for VisionSqueezer.
 *
 *   npx vision-squeezer install            # prompts for client + scope
 *   npx vision-squeezer install --scope user
 *   npx vision-squeezer install --client claude --scope project --yes
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

const CLIENTS = [
  { key: 'claude', label: 'Claude Code', cli: 'claude' },
  { key: 'codex', label: 'Codex CLI', cli: 'codex' },
  { key: 'qwen', label: 'Qwen Code', cli: 'qwen' },
  { key: 'opencode', label: 'OpenCode', cli: 'opencode' },
  { key: 'gemini', label: 'Gemini CLI', cli: 'gemini' },
  { key: 'kimi', label: 'Kimi CLI', cli: 'kimi' },
];

// Install methods available for Claude Code only. Codex/Qwen go straight to
// `mcp add`. OpenCode has no non-interactive `mcp add` — it only reads
// config files — so it gets its own OPENCODE_METHOD below instead.
const METHODS = [
  {
    key: 'plugin',
    label: 'Claude plugin marketplace',
    description: 'One-command install via /plugin — bundles MCP server + stats/doctor/upgrade skills',
  },
  {
    key: 'mcp-add',
    label: 'claude mcp add',
    description: 'Register only the MCP server (no bundled skills) using `claude mcp add`',
  },
];

// OpenCode's CLI (`opencode mcp add`) is interactive-only with no flags to
// pass name/command non-interactively, so we write its JSON config directly.
const OPENCODE_METHOD = { key: 'config-file', label: 'OpenCode config file' };

const MARKETPLACE_REPO = 'eralpozcan/vision-squeezer';
const PLUGIN_NAME = 'vision-squeezer-mcp';

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
  -c, --client <name>   Target CLI (claude | codex | qwen | opencode | gemini | kimi)
  -m, --method <name>   Install method for Claude Code (plugin | mcp-add)
  -s, --scope <name>    Install scope for 'mcp-add' (user | local | project)
  -y, --yes             Skip confirmation prompt
  -h, --help            Show this help

Methods (Claude Code only):
  plugin    /plugin marketplace add eralpozcan/vision-squeezer
            /plugin install vision-squeezer-mcp@vision-squeezer
            Bundles MCP server + stats/doctor/upgrade skills.

  mcp-add   claude mcp add [--scope X] vision-squeezer -- npx -y vision-squeezer@<version>
            Server only, no skills bundled. The version is pinned to the
            installer's package version so the npx cache busts on upgrade.

Scopes (mcp-add):
  user      All projects on this machine (recommended)
  local     This project only, private to you (default for Claude Code)
  project   Shared via .mcp.json checked into the repo

OpenCode:
  \`opencode mcp add\` is interactive-only (no flags), so this installer
  writes the MCP entry directly into OpenCode's JSON config instead:
    user      ~/.config/opencode/opencode.json (global)
    project   ./opencode.json (repo root)
  OpenCode has no 'local' scope.

Gemini CLI:
  gemini mcp add --scope X vision-squeezer -- npx -y vision-squeezer@<version>
  Scope writes to ~/.gemini/settings.json (user) or .gemini/settings.json
  (project). Gemini CLI has no 'local' scope.

Kimi CLI:
  kimi mcp add vision-squeezer -- npx -y vision-squeezer@<version>
  Always writes the single global ~/.kimi/mcp.json — no scope flag exists.
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

function buildArgs(client, scope) {
  // claude/codex/qwen/gemini accept the same shape: `<cli> mcp add [--scope X] NAME -- npx -y vision-squeezer@<version>`
  // `local` is the Claude Code default — omit the flag to keep behavior identical to docs.
  // Kimi CLI has no scope concept (single global ~/.kimi/mcp.json) — never pass --scope.
  const args = ['mcp', 'add'];
  if (client.key !== 'kimi' && scope.key !== 'local') {
    args.push('--scope', scope.key);
  }
  args.push('vision-squeezer', '--', 'npx', '-y', `vision-squeezer@${PKG_VERSION}`);
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
  if (client && (client.key === 'opencode' || client.key === 'gemini') && scope && scope.key === 'local') {
    console.error(`${client.label} has no 'local' scope. Use 'user' or 'project'.`);
    return 1;
  }

  let method;
  if (opts.method) {
    method = METHODS.find((m) => m.key === opts.method.toLowerCase());
    if (!method) {
      console.error(`Unknown method: ${opts.method}. Expected one of: ${METHODS.map((m) => m.key).join(', ')}`);
      return 1;
    }
  }

  // For non-Claude clients there is no /plugin equivalent — auto-force mcp-add.
  if (client && client.key !== 'claude' && !method) {
    method = METHODS.find((m) => m.key === 'mcp-add');
  }
  const needsMethodPrompt = () => {
    if (!client) return true; // client unknown → maybe claude → may need method
    if (client.key !== 'claude') return false;
    return !method;
  };
  const needsScopePrompt = () => {
    if (method && method.key === 'plugin') return false;
    if (client && client.key === 'kimi') return false; // single global config, no scope
    return !scope;
  };

  const interactive = !client || needsMethodPrompt() || needsScopePrompt();
  let rl;
  if (interactive) {
    if (!process.stdin.isTTY) {
      console.error('Non-interactive shell detected. Pass --client (and --method / --scope) explicitly.');
      printHelp();
      return 1;
    }
    rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  }

  try {
    if (!client) client = await pickFromList(rl, 'Pick an AI CLI', CLIENTS, 'claude');

    if (client.key === 'claude') {
      if (!method) method = await pickFromList(rl, 'Pick install method', METHODS, 'plugin');
    } else if (client.key === 'opencode') {
      // `opencode mcp add` is interactive-only, no non-interactive shape to target.
      method = OPENCODE_METHOD;
    } else if (client.key === 'kimi') {
      // No --scope flag exists; kimi mcp add always writes ~/.kimi/mcp.json.
      method = METHODS.find((m) => m.key === 'mcp-add');
      if (!scope) scope = { key: 'user', label: 'user' };
    } else {
      // codex / qwen / gemini have no plugin marketplace concept
      method = METHODS.find((m) => m.key === 'mcp-add');
    }

    if ((method.key === 'mcp-add' || method.key === 'config-file') && !scope) {
      const scopeList = (client.key === 'opencode' || client.key === 'gemini')
        ? SCOPES.filter((s) => s.key !== 'local')
        : SCOPES;
      scope = await pickFromList(rl, 'Pick install scope', scopeList, 'user');
    }

    if (!commandExists(client.cli) && method.key !== 'config-file') {
      console.error(`\n'${client.cli}' CLI not found in PATH.`);
      const flags = method.key === 'plugin'
        ? `--client ${client.key} --method plugin`
        : `--client ${client.key} --method mcp-add --scope ${scope.key}`;
      console.error(`Install it first, then re-run: npx vision-squeezer install ${flags}`);
      return 1;
    }

    if (method.key === 'plugin') {
      return await runPluginInstall(rl, opts.yes);
    }

    if (method.key === 'config-file') {
      return await runOpencodeInstall(rl, scope, opts.yes);
    }

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
    return 0;
  } finally {
    if (rl && !rl.closed) rl.close();
  }
}

function opencodeConfigPath(scope) {
  return scope.key === 'user'
    ? path.join(os.homedir(), '.config', 'opencode', 'opencode.json')
    : path.join(process.cwd(), 'opencode.json');
}

async function runOpencodeInstall(rl, scope, yes) {
  const configPath = opencodeConfigPath(scope);
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
    config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
  } else {
    fs.mkdirSync(path.dirname(configPath), { recursive: true });
    config.$schema = 'https://opencode.ai/config.json';
  }
  config.mcp = config.mcp || {};
  config.mcp['vision-squeezer'] = {
    type: 'local',
    command: ['npx', '-y', `vision-squeezer@${PKG_VERSION}`],
    enabled: true,
  };
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');

  console.log(`\nDone. VisionSqueezer registered with OpenCode (scope: ${scope.key}) at ${configPath}.`);
  return 0;
}

async function runPluginInstall(rl, yes) {
  // /plugin commands run inside Claude Code's TUI — they aren't shell-callable.
  // Print copy-paste instructions instead of trying to spawn a slash command.
  console.log(`
Claude plugin marketplace install
─────────────────────────────────
Open Claude Code and run these two commands:

  /plugin marketplace add ${MARKETPLACE_REPO}
  /plugin install ${PLUGIN_NAME}@vision-squeezer

The first command registers the marketplace; the second installs the bundled
MCP server. After install, restart any open Claude Code session for the MCP
server to attach.
`);

  if (rl && !yes) {
    const ans = await prompt(rl, 'Copy the marketplace add command to clipboard? [Y/n]: ');
    if (!ans || /^y(es)?$/i.test(ans)) {
      const text = `/plugin marketplace add ${MARKETPLACE_REPO}`;
      const ok = copyToClipboard(text);
      console.log(ok ? 'Copied. Paste in Claude Code.' : 'Clipboard unavailable — copy manually.');
    }
    rl.close();
  }
  return 0;
}

function copyToClipboard(text) {
  const candidates = process.platform === 'darwin'
    ? [['pbcopy', []]]
    : process.platform === 'win32'
      ? [['clip', []]]
      : [['xclip', ['-selection', 'clipboard']], ['xsel', ['--clipboard', '--input']], ['wl-copy', []]];
  for (const [cmd, args] of candidates) {
    const r = spawnSync(cmd, args, { input: text, stdio: ['pipe', 'ignore', 'ignore'] });
    if (!r.error && (r.status ?? 0) === 0) return true;
  }
  return false;
}

main().then((code) => process.exit(code)).catch((err) => {
  console.error(err.message || err);
  process.exit(1);
});

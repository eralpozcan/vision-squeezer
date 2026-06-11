<script setup lang="ts">
const installData: Record<string, { label: string, comment: string, code: string }> = {
  'claude-code': {
    label: 'Claude Code (CLI)',
    comment: '# Zero-config one-liner for Claude Code',
    code: 'claude mcp add vision-squeezer -- npx -y vision-squeezer'
  },
  'cursor': {
    label: 'Cursor (AI Editor)',
    comment: '// Add to .cursor/mcp.json',
    code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
  },
  'vscode': {
    label: 'VS Code Copilot',
    comment: '// Add to settings.json (VS Code Copilot)',
    code: '{\n  "github.copilot.mcp.servers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
  },
  'windsurf': {
    label: 'Windsurf (Codeium)',
    comment: '// Add to ~/.codeium/windsurf/mcp_config.json',
    code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
  },
  'jetbrains': {
    label: 'JetBrains (IntelliJ, WebStorm)',
    comment: '// Tools → GitHub Copilot → MCP → Configure',
    code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
  },
  'zed': {
    label: 'Zed Editor',
    comment: '// Add to ~/.config/zed/settings.json',
    code: '{\n  "context_servers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
  },
  'claude-desktop': {
    label: 'Claude Desktop (Mac/Win)',
    comment: '// Add to ~/.config/claude/claude_desktop_config.json',
    code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
  },
  'shell-hook': {
    label: 'Shell Integration (Zsh/Bash)',
    comment: "# Add to your .zshrc or .bashrc for the 'squeeze' command",
    code: 'eval "$(npx -y vision-squeezer setup-hook)"'
  },
  'claude-skill': {
    label: 'Claude Code Skill (/vision-stats)',
    comment: '# Zero-overhead /vision-stats skill for Claude Code',
    code: '# Option 1: Auto-install via shell hook (recommended)\neval "$(vision-squeezer setup-hook)"\n\n# Option 2: Install via Claude Code marketplace\n# Add to ~/.claude/settings.json > extraKnownMarketplaces:\n# "vision-squeezer": { "source": { "source": "github", "repo": "eralpozcan/vision-squeezer" } }\n# Then in Claude Code:\n/plugins add vision-stats@vision-squeezer'
  },
  'python': {
    label: 'Python (pip install)',
    comment: '# Python bindings via pyo3 / maturin',
    code: 'pip install vision-squeezer\n\n# Usage\nimport vision_squeezer as vs\nreport = vs.optimize_image(\n    "screenshot.png",\n    model="claude",\n    auto_quality=0.95,\n    output_path="screenshot.optimized.jpg",\n)\nprint(report["tokens_saved"], report["size_reduction_pct"])'
  }
}

const options = Object.entries(installData).map(([value, v]) => ({ value, label: v.label }))
const selected = ref('claude-code')
const current = computed(() => installData[selected.value]!)

const copied = ref(false)
async function copy() {
  try {
    await navigator.clipboard.writeText(current.value.code)
    copied.value = true
    setTimeout(() => (copied.value = false), 2000)
  } catch {
    // clipboard unavailable
  }
}
</script>

<template>
  <div class="my-6">
    <USelectMenu
      v-model="selected"
      :items="options"
      value-key="value"
      class="w-full sm:w-80 mb-4"
    />

    <div class="rounded-lg border border-default bg-elevated/50 overflow-hidden">
      <div class="flex items-center justify-between px-4 py-2 border-b border-default">
        <span class="text-xs font-mono text-muted">{{ current.comment }}</span>
        <UButton
          :icon="copied ? 'i-lucide-check' : 'i-lucide-copy'"
          :color="copied ? 'success' : 'neutral'"
          variant="ghost"
          size="xs"
          @click="copy"
        >
          {{ copied ? 'Copied!' : 'Copy' }}
        </UButton>
      </div>
      <pre class="overflow-x-auto p-4 text-sm"><code class="font-mono">{{ current.code }}</code></pre>
    </div>
  </div>
</template>

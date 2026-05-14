// Benchmark Data Engine based on README calculations
const benchmarkData = {
    standard: {
        desc: "Optimizing a standard 2400x1670 screenshot.",
        agnostic: {
            claude: { val: "1,150", pct: "-21.5%", sub: "5,344 → 4,194 tokens", hl: true },
            gpt4o: { val: "340", pct: "-30.8%", sub: "1,105 → 765 tokens", hl: true },
            file: { val: "28.6%", sub: "0.5MB → 0.3MB" },
            note: "When no target is specified, Squeezer reduces file size and mathematically optimizes boundaries to be generally efficient across all models."
        },
        gpt4o: {
            claude: { val: "1,506", pct: "-28.2%", sub: "5,344 → 3,838 tokens", hl: false },
            gpt4o: { val: "Maximum", pct: "Locked", sub: "Perfectly locked to 6 tiles", hl: true },
            file: { val: "33.6%", sub: "0.5MB → 0.3MB" },
            note: "Targeting GPT-4o perfectly fits the image into a solid 6-tile boundary (2399x1200) mathematically calculated backwards from OpenAI's short-side scaling algorithm."
        },
        claude: {
            claude: { val: "626", pct: "-11.7%", sub: "5,344 → 4,718 tokens", hl: true },
            gpt4o: { val: "0", pct: "0%", sub: "1,105 → 1,105 tokens", hl: false },
            file: { val: "21.3%", sub: "0.5MB → 0.4MB" },
            note: "By targeting Claude, Squeezer preserves the massive 2304x1536 resolution and solely trims solid padding, minimizing token cost via Claude's area-based formula."
        }
    },
    highres: {
        desc: "Optimizing a massive 4096x3072 photograph (12MP).",
        agnostic: {
            claude: { val: "4,544", pct: "-27.1%", sub: "16,777 → 12,233 tokens", hl: true },
            gpt4o: { val: "0", pct: "Anomaly", sub: "OpenAI Grid Paradox Detected", hl: false },
            file: { val: "39.6%", sub: "2.2MB → 1.3MB" },
            note: "Notice the OpenAI Aspect Ratio Anomaly: Removing padding made the image 'wider', which ironically pushes the long-side into a new OpenAI grid row! (Use --model gpt4o to fix)."
        },
        gpt4o: {
            claude: { val: "5,595", pct: "-33.3%", sub: "16,777 → 11,182 tokens", hl: false },
            gpt4o: { val: "Maximum", pct: "Locked", sub: "Grid boundary perfectly contained", hl: true },
            file: { val: "43.2%", sub: "2.2MB → 1.2MB" },
            note: "By explicitly targeting gpt4o, Squeezer optimizes the boundaries such that the new aspect ratio is safely contained. File footprint shrinks by 43%."
        },
        claude: {
            claude: { val: "2,360", pct: "-14.1%", sub: "16,777 → 14,417 tokens", hl: true },
            gpt4o: { val: "0", pct: "0%", sub: "765 → 1,105 (Padding trim anomaly)", hl: false },
            file: { val: "31.5%", sub: "2.2MB → 1.5MB" },
            note: "Squeezer refuses to aggressively downscale (like GPT requires), instead carefully trimming padding to preserve 10+ Megapixels of ultra-fine detail."
        }
    }
};

let currentImage = 'standard';
let currentTarget = 'agnostic';

function updateUI() {
    const data = benchmarkData[currentImage][currentTarget];
    
    // Update Description
    const targetName = currentTarget === 'gpt4o' ? 'GPT-4o' : (currentTarget === 'claude' ? 'Claude' : 'Agnostic');
    document.getElementById('calc-desc').innerHTML = `${benchmarkData[currentImage].desc} Target: <b>${targetName}</b>.`;
    
    // Claude Update
    document.getElementById('val-claude').innerHTML = `${data.claude.val} <small>(${data.claude.pct})</small>`;
    document.getElementById('sub-claude').innerText = data.claude.sub;
    document.getElementById('card-claude').className = `stat-card ${data.claude.hl ? 'highlight-card' : ''}`;
    
    // GPT Update
    document.getElementById('val-gpt').innerHTML = `${data.gpt4o.val} <small>(${data.gpt4o.pct})</small>`;
    document.getElementById('sub-gpt').innerText = data.gpt4o.sub;
    document.getElementById('card-gpt').className = `stat-card ${data.gpt4o.hl ? 'highlight-card' : ''}`;
    
    // File Update
    document.getElementById('val-file').innerHTML = `${data.file.val} <small>Smaller</small>`;
    document.getElementById('sub-file').innerText = data.file.sub;
    
    // Note Update
    document.getElementById('calc-note').innerText = data.note;
}

// Event Listeners for Image Select
document.querySelectorAll('#image-toggle .toggle-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
        document.querySelectorAll('#image-toggle .toggle-btn').forEach(b => b.classList.remove('active'));
        e.target.classList.add('active');
        currentImage = e.target.getAttribute('data-val');
        updateUI();
    });
});

// Event Listeners for Target Select
document.querySelectorAll('#target-toggle .toggle-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
        document.querySelectorAll('#target-toggle .toggle-btn').forEach(b => b.classList.remove('active'));
        e.target.classList.add('active');
        currentTarget = e.target.getAttribute('data-val');
        updateUI();
    });
});

// Initialize first view
updateUI();

// Copy to Clipboard Logic
function copyCode(btn, text) {
    navigator.clipboard.writeText(text).then(() => {
        const originalHTML = btn.innerHTML;
        btn.innerHTML = '✓ Copied!';
        btn.style.color = '#27c93f';
        btn.style.borderColor = '#27c93f';
        setTimeout(() => {
            btn.innerHTML = originalHTML;
            btn.style.color = '';
            btn.style.borderColor = '';
        }, 2000);
    }).catch(err => {
        console.error('Failed to copy text: ', err);
    });
}

// MCP Installation Data Logic
const installData = {
    'claude-code': {
        comment: "# Zero-config one-liner for Claude Code",
        code: "claude mcp add vision-squeezer -- npx -y vision-squeezer"
    },
    'cursor': {
        comment: "// Add to .cursor/mcp.json",
        code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
    },
    'vscode': {
        comment: "// Add to settings.json (VS Code Copilot)",
        code: '{\n  "github.copilot.mcp.servers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
    },
    'windsurf': {
        comment: "// Add to ~/.codeium/windsurf/mcp_config.json",
        code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
    },
    'jetbrains': {
        comment: "// Tools → GitHub Copilot → MCP → Configure",
        code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
    },
    'zed': {
        comment: "// Add to ~/.config/zed/settings.json",
        code: '{\n  "context_servers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
    },
    'claude-desktop': {
        comment: "// Add to ~/.config/claude/claude_desktop_config.json",
        code: '{\n  "mcpServers": {\n    "vision-squeezer": {\n      "command": "npx",\n      "args": ["-y", "vision-squeezer"]\n    }\n  }\n}'
    },
    'shell-hook': {
        comment: "# Add to your .zshrc or .bashrc for the 'squeeze' command",
        code: 'eval "$(npx -y vision-squeezer setup-hook)"'
    },
    'claude-skill': {
        comment: "# Zero-overhead /vision-stats skill for Claude Code",
        code: '# Option 1: Auto-install via shell hook (recommended)\neval "$(vision-squeezer setup-hook)"\n\n# Option 2: Install via Claude Code marketplace\n# Add to ~/.claude/settings.json > extraKnownMarketplaces:\n# "vision-squeezer": { "source": { "source": "github", "repo": "eralpozcan/vision-squeezer" } }\n# Then in Claude Code:\n/plugins add vision-stats@vision-squeezer'
    }
};

function updateInstallCode() {
    const editor = document.getElementById('editor-select').value;
    const data = installData[editor];
    document.getElementById('install-comment').innerText = data.comment;
    document.getElementById('install-code').innerText = data.code;
}

function copyInstallCode(btn) {
    const codeText = document.getElementById('install-code').innerText;
    copyCode(btn, codeText);
}

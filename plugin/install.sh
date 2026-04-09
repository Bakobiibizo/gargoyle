#!/usr/bin/env bash
#
# Gargoyle Memory Plugin Installer
#
# Installs the gargoyle-memory plugin for Claude Code (CLI) and
# packages it for Claude Desktop (Cowork).
#
# Usage:
#   bash install.sh           # Install for CLI + package .plugin
#   bash install.sh --cli     # CLI only
#   bash install.sh --desktop # Desktop .plugin only

set -euo pipefail

PLUGIN_DIR="$(cd "$(dirname "$0")" && pwd)"
COWORK_DIR="${TEMP:-/tmp}/gargoyle-memory-cowork"
PLUGIN_OUT="${TEMP:-/tmp}/gargoyle-memory.plugin"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

MODE="${1:-all}"

echo "Gargoyle Memory Plugin Installer"
echo "================================"
echo "Plugin source: $PLUGIN_DIR"
echo ""

# --- Check dependencies ---
if ! command -v python3 &>/dev/null; then
    echo "ERROR: python3 not found. Install Python 3.10+."
    exit 1
fi

if ! python3 -c "import httpx" 2>/dev/null; then
    echo "Installing httpx..."
    pip install httpx
fi

# --- CLI Install ---
if [[ "$MODE" == "all" || "$MODE" == "--cli" ]]; then
    echo "--- CLI Plugin Setup ---"
    echo ""
    echo "To use with Claude Code CLI, run:"
    echo "  claude --plugin-dir $PLUGIN_DIR"
    echo ""
    echo "Or add to your settings.json enabledPlugins."
    echo ""
fi

# --- Desktop / Cowork Package ---
if [[ "$MODE" == "all" || "$MODE" == "--desktop" ]]; then
    echo "--- Desktop Plugin Package ---"

    rm -rf "$COWORK_DIR"
    mkdir -p "$COWORK_DIR/.claude-plugin"
    mkdir -p "$COWORK_DIR/skills/search-memory"
    mkdir -p "$COWORK_DIR/agents"
    mkdir -p "$COWORK_DIR/hooks"
    mkdir -p "$COWORK_DIR/scripts"

    # Manifest (Cowork format)
    cat > "$COWORK_DIR/.claude-plugin/plugin.json" << 'MANIFEST'
{
  "name": "gargoyle-memory",
  "version": "0.1.0",
  "description": "Automatic memory surfacing and accumulation via the gargoyle knowledge graph.",
  "author": { "name": "richa" },
  "hooks": "./hooks/hooks.json",
  "mcpServers": "./.mcp.json"
}
MANIFEST

    # Hooks (Cowork format with matchers)
    cat > "$COWORK_DIR/hooks/hooks.json" << 'HOOKS'
{
  "UserPromptSubmit": [
    {
      "matcher": "",
      "hooks": [
        {
          "type": "command",
          "command": "python3 ${CLAUDE_PLUGIN_ROOT}/scripts/memory-search.py",
          "timeout": 8
        }
      ]
    }
  ],
  "Stop": [
    {
      "matcher": "",
      "hooks": [
        {
          "type": "command",
          "command": "python3 ${CLAUDE_PLUGIN_ROOT}/scripts/memory-writer.py",
          "timeout": 12
        }
      ]
    }
  ]
}
HOOKS

    # Copy scripts (with env var config)
    cp "$PLUGIN_DIR/scripts/memory-search.py" "$COWORK_DIR/scripts/"
    cp "$PLUGIN_DIR/scripts/memory-writer.py" "$COWORK_DIR/scripts/"
    cp "$PLUGIN_DIR/scripts/requirements.txt" "$COWORK_DIR/scripts/"

    # Make scripts use env vars for portability
    sed -i 's|DB_PATH = "D:/apps/gargoyle/src-tauri/gargoyle.db"|DB_PATH = os.environ.get("GARGOYLE_DB_PATH", "D:/apps/gargoyle/src-tauri/gargoyle.db")|' "$COWORK_DIR/scripts/memory-search.py"
    sed -i 's|ERASMUS_TEXT_URL = "https://text-erasmus.ngrok.dev/v1/chat/completions"|ERASMUS_TEXT_URL = os.environ.get("ERASMUS_TEXT_URL", "https://text-erasmus.ngrok.dev/v1/chat/completions")|' "$COWORK_DIR/scripts/memory-search.py"
    sed -i 's|ERASMUS_EMBED_URL = "https://dev-erasmus.ngrok.dev/embed"|ERASMUS_EMBED_URL = os.environ.get("ERASMUS_EMBED_URL", "https://dev-erasmus.ngrok.dev/embed")|' "$COWORK_DIR/scripts/memory-search.py"
    sed -i 's|DB_PATH = "D:/apps/gargoyle/src-tauri/gargoyle.db"|DB_PATH = os.environ.get("GARGOYLE_DB_PATH", "D:/apps/gargoyle/src-tauri/gargoyle.db")|' "$COWORK_DIR/scripts/memory-writer.py"
    sed -i 's|ERASMUS_TEXT_URL = "https://text-erasmus.ngrok.dev/v1/chat/completions"|ERASMUS_TEXT_URL = os.environ.get("ERASMUS_TEXT_URL", "https://text-erasmus.ngrok.dev/v1/chat/completions")|' "$COWORK_DIR/scripts/memory-writer.py"

    # Copy skill and agent
    cp "$PLUGIN_DIR/skills/search-memory/SKILL.md" "$COWORK_DIR/skills/search-memory/"
    cp "$PLUGIN_DIR/agents/memory-searcher.md" "$COWORK_DIR/agents/"

    # MCP config
    cp "$PLUGIN_DIR/.mcp.json" "$COWORK_DIR/"

    # README
    cat > "$COWORK_DIR/README.md" << 'README'
# Gargoyle Memory Plugin

Automatic memory surfacing and accumulation for Claude via the gargoyle knowledge graph.

## What it does

- **On every prompt**: Searches gargoyle for relevant entities and injects them as context
- **After every response**: Extracts noteworthy observations and writes them as typed entities
- **Manual deep search**: `/gargoyle-memory:search-memory <query>`

## Requirements

- Python 3.10+ with `httpx` (`pip install httpx`)
- Gargoyle database (set `GARGOYLE_DB_PATH` env var or use default)
- Erasmus inference endpoint running (set `ERASMUS_TEXT_URL` or use default)

## Environment Variables

| Variable | Default |
|---|---|
| `GARGOYLE_DB_PATH` | `D:/apps/gargoyle/src-tauri/gargoyle.db` |
| `ERASMUS_TEXT_URL` | `https://text-erasmus.ngrok.dev/v1/chat/completions` |
| `ERASMUS_EMBED_URL` | `https://dev-erasmus.ngrok.dev/embed` |
README

    # Package as .plugin
    cd "$COWORK_DIR"
    python3 -c "
import zipfile, os
with zipfile.ZipFile('$PLUGIN_OUT', 'w', zipfile.ZIP_DEFLATED) as zf:
    for root, dirs, files in os.walk('.'):
        for f in files:
            zf.write(os.path.join(root, f))
"

    echo "Desktop plugin packaged: $PLUGIN_OUT"
    echo ""
    echo "To install in Claude Desktop:"
    echo "  1. Open Claude Desktop"
    echo "  2. Drag and drop $PLUGIN_OUT into a conversation"
    echo "  3. Or use /plugin to install from the file"
fi

echo ""
echo "Done."

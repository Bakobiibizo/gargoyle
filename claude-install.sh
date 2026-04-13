#!/usr/bin/env bash
#
# Gargoyle Claude Extension Installer
#
# Builds the gargoyle-mcp binary (release) and installs the plugin
# to the Claude Extensions directory with correct naming convention.
#
# Usage:
#   bash claude-install.sh              # Build + install
#   bash claude-install.sh --skip-build # Copy only (skip cargo build)
#
# Requires: bash (Git Bash on Windows), cargo (for build), python3, pip
#

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_SRC="$PROJECT_DIR/plugin"
BINARY_SRC="$PROJECT_DIR/src-tauri/target/release/gargoyle-mcp.exe"

# Claude Extensions directory — uses the local.unpacked naming convention
EXTENSIONS_BASE="$APPDATA/Claude/Claude Extensions"
EXTENSIONS_DIR="$EXTENSIONS_BASE/local.unpacked.bakobiibizo.gargoyle"
SETTINGS_DIR="$APPDATA/Claude/Claude Extensions Settings"
SETTINGS_FILE="$SETTINGS_DIR/local.unpacked.bakobiibizo.gargoyle.json"

SKIP_BUILD=false
if [[ "${1:-}" == "--skip-build" ]]; then
    SKIP_BUILD=true
fi

echo "Gargoyle Claude Extension Installer"
echo "===================================="
echo "Project:    $PROJECT_DIR"
echo "Target:     $EXTENSIONS_DIR"
echo ""

# --- Check dependencies ---
if ! command -v python3 &>/dev/null; then
    echo "WARNING: python3 not found. The auto-memory hooks require Python 3.10+."
    echo "         Install Python and re-run, or install httpx manually later."
    echo ""
fi

if command -v python3 &>/dev/null && ! python3 -c "import httpx" 2>/dev/null; then
    echo "Installing httpx (required by hook scripts)..."
    pip install httpx || pip3 install httpx || echo "WARNING: Could not install httpx. Install manually: pip install httpx"
    echo ""
fi

# --- Build ---
if [[ "$SKIP_BUILD" == false ]]; then
    echo "Building gargoyle-mcp (release)..."
    cd "$PROJECT_DIR/src-tauri"
    cargo build --bin gargoyle-mcp --release
    cd "$PROJECT_DIR"
    echo "Build complete."
    echo ""
fi

# --- Verify binary exists ---
if [[ ! -f "$BINARY_SRC" ]]; then
    echo "ERROR: Binary not found at $BINARY_SRC"
    echo "Run without --skip-build to build first."
    exit 1
fi

# --- Verify plugin source exists ---
if [[ ! -d "$PLUGIN_SRC" ]]; then
    echo "ERROR: Plugin directory not found at $PLUGIN_SRC"
    exit 1
fi

# --- Install plugin files ---
echo "Installing plugin to $EXTENSIONS_DIR ..."

mkdir -p "$EXTENSIONS_DIR"

# Copy plugin structure
cp -r "$PLUGIN_SRC/.claude-plugin" "$EXTENSIONS_DIR/"
cp -r "$PLUGIN_SRC/agents"         "$EXTENSIONS_DIR/"
cp -r "$PLUGIN_SRC/hooks"          "$EXTENSIONS_DIR/"
cp -r "$PLUGIN_SRC/scripts"        "$EXTENSIONS_DIR/"
cp -r "$PLUGIN_SRC/skills"         "$EXTENSIONS_DIR/"

# Copy metadata files (always overwrite to stay in sync with source)
for f in manifest.json server.json pyproject.toml CLAUDE.md README.md LICENSE.md .python-version; do
    if [[ -f "$PLUGIN_SRC/$f" ]]; then
        cp -f "$PLUGIN_SRC/$f" "$EXTENSIONS_DIR/"
    fi
done

# Copy install.sh for reference
cp -f "$PLUGIN_SRC/install.sh" "$EXTENSIONS_DIR/" 2>/dev/null || true

# Copy assets
mkdir -p "$EXTENSIONS_DIR/assets"
if [[ -f "$PROJECT_DIR/assets/icon.png" ]]; then
    cp -f "$PROJECT_DIR/assets/icon.png" "$EXTENSIONS_DIR/assets/logo.png"
fi

# --- Resolve paths ---
# Convert to Windows-style paths for the settings file
BINARY_WIN=$(cygpath -w "$BINARY_SRC" 2>/dev/null || echo "$BINARY_SRC")
DB_PATH="$PROJECT_DIR/src-tauri/gargoyle.db"
DB_WIN=$(cygpath -m "$DB_PATH" 2>/dev/null || echo "$DB_PATH")

# --- Create settings file ---
mkdir -p "$SETTINGS_DIR"

if [[ -f "$SETTINGS_FILE" ]]; then
    echo "Settings file already exists, preserving: $SETTINGS_FILE"
else
    echo "Creating settings file..."
    cat > "$SETTINGS_FILE" << SETTINGS
{
  "isEnabled": true,
  "userConfig": {
    "db_path": "$DB_WIN",
    "gargoyle_mcp_path": "$BINARY_WIN",
    "erasmus_text_url": "https://text-erasmus.ngrok.dev/v1/chat/completions",
    "erasmus_embed_url": "https://erasmus.ngrok.dev/embed"
  }
}
SETTINGS
    echo "Created: $SETTINGS_FILE"
fi

# --- Summary ---
BINARY_SIZE=$(du -h "$BINARY_SRC" | cut -f1)

echo ""
echo "Installation complete!"
echo ""
echo "  Plugin:   $EXTENSIONS_DIR"
echo "  Settings: $SETTINGS_FILE"
echo "  Binary:   $BINARY_SRC ($BINARY_SIZE)"
echo ""
echo "The plugin is configured with:"
echo "  Database:  $DB_WIN"
echo "  MCP path:  $BINARY_WIN"
echo ""
echo "To customize paths, edit:"
echo "  $(cygpath -w "$SETTINGS_FILE" 2>/dev/null || echo "$SETTINGS_FILE")"
echo ""
echo "Restart Claude Desktop or Claude Code to pick up the plugin."

# Gargoyle Memory Plugin

A Claude Code plugin that provides automatic memory surfacing and accumulation via the gargoyle knowledge graph.

## What It Does

- **On every prompt**: Searches gargoyle for relevant entities (FTS + semantic embeddings) and injects them as context
- **After every response**: Extracts noteworthy observations and writes them as typed entities
- **Manual deep search**: `/gargoyle:search-memory <query>` for targeted knowledge graph exploration
- **Memory searcher agent**: Specialized haiku agent for multi-step retrieval

## Features

- 20 MCP tools for full knowledge graph management (entities, relations, claims, templates)
- Full-text search (FTS5) and embedding-based semantic similarity search
- BFS graph traversal for exploring entity neighborhoods
- Typed entity system with canonical field schemas per type
- Auto-deduplication within sessions to avoid surfacing repeats
- Provenance tracking with run IDs

## Requirements

- **Gargoyle MCP binary** (Rust) — built from the gargoyle repo
- **Python 3.10+** with `httpx` (`pip install httpx`)
- **Gargoyle database** — SQLite, created by the MCP server on first run
- **Erasmus inference endpoint** (optional) — for keyword extraction and embedding search

## Installation

### Quick Install (Windows)

From the gargoyle repo root:

```bash
bash claude-install.sh
```

This will:
1. Build the `gargoyle-mcp` binary (release)
2. Copy the plugin to `%APPDATA%/Claude/Claude Extensions/local.unpacked.bakobiibizo.gargoyle/`
3. Create a settings file at `%APPDATA%/Claude/Claude Extensions Settings/local.unpacked.bakobiibizo.gargoyle.json`

To skip the build step (if you already have the binary):

```bash
bash claude-install.sh --skip-build
```

### Manual Install

1. Build the MCP server:
   ```bash
   cd src-tauri
   cargo build --bin gargoyle-mcp --release
   ```

2. Copy the `plugin/` directory contents to:
   ```
   %APPDATA%/Claude/Claude Extensions/local.unpacked.bakobiibizo.gargoyle/
   ```

3. Create the settings file at:
   ```
   %APPDATA%/Claude/Claude Extensions Settings/local.unpacked.bakobiibizo.gargoyle.json
   ```
   ```json
   {
     "isEnabled": true,
     "userConfig": {
       "db_path": "D:/apps/gargoyle/src-tauri/gargoyle.db",
       "gargoyle_mcp_path": "D:\\apps\\gargoyle\\src-tauri\\target\\release\\gargoyle-mcp.exe",
       "erasmus_text_url": "https://text-erasmus.ngrok.dev/v1/chat/completions",
       "erasmus_embed_url": "https://erasmus.ngrok.dev/embed"
     }
   }
   ```

4. Restart Claude Desktop or Claude Code.

### Claude Code CLI (without Desktop)

You can also point Claude Code directly at the plugin:

```bash
claude --plugin-dir /path/to/gargoyle/plugin
```

Or add the MCP server to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "gargoyle": {
      "command": "/path/to/gargoyle-mcp.exe",
      "args": ["--db", "/path/to/gargoyle.db"]
    }
  }
}
```

## Configuration

Settings are stored in the Claude Extensions Settings JSON file. All paths must be absolute.

| Setting | Required | Description |
|---|---|---|
| `db_path` | Yes | Path to gargoyle SQLite database |
| `gargoyle_mcp_path` | Yes | Path to gargoyle-mcp executable |
| `erasmus_text_url` | No | OpenAI-compatible chat completions endpoint (keyword extraction) |
| `erasmus_embed_url` | No | Embedding endpoint (semantic search) |

## Plugin Structure

```
gargoyle/
  .claude-plugin/
    plugin.json          Plugin metadata, MCP config, user config schema
  agents/
    memory-searcher.md   Haiku agent for multi-step memory retrieval
  assets/
    logo.png             Plugin icon
  hooks/
    hooks.json           Hook event bindings (UserPromptSubmit, Stop)
  scripts/
    memory-search.py     Prompt hook: search graph, inject context
    memory-writer.py     Stop hook: extract observations, write entities
    requirements.txt     Python dependencies (httpx)
  skills/
    search-memory/
      SKILL.md           Manual /gargoyle:search-memory skill
  manifest.json          Extension registry metadata
  server.json            MCP server registry entry
  CLAUDE.md              Plugin-specific Claude instructions
  README.md              This file
  LICENSE.md             MIT license
  pyproject.toml         Python project metadata
  .python-version        Python version pin
```

## How the Hooks Work

### Search Hook (`memory-search.py`, on `UserPromptSubmit`)

1. Reads the user's prompt from `CLAUDE_USER_PROMPT`
2. Extracts keywords via Erasmus LLM (falls back to local stop-word filtering if unavailable)
3. Queries gargoyle SQLite directly — FTS for keyword matches, embedding similarity for semantic matches
4. Session dedup (temp file) prevents surfacing the same entity twice per session
5. Returns up to 3000 chars of relevant entity context as `additionalContext`

### Writer Hook (`memory-writer.py`, on `Stop`)

1. Reads the conversation transcript tail from `CLAUDE_TRANSCRIPT`
2. Asks Erasmus to extract noteworthy observations (falls back gracefully)
3. Validates extracted entities against type schemas
4. Writes new entities to gargoyle SQLite with source `"agent"`
5. Writer dedup prevents writing the same observation title twice per session

## Entity Types

| Type | Description | Key Fields |
|---|---|---|
| `note` | Observations, preferences, patterns | context, tags |
| `idea` | Concepts worth exploring | stage, tags, potential_value |
| `task` | Action items and TODOs | assignee, due_date, effort, tags |
| `person` | People with meaningful context | full_name, role, company, relationship |
| `project` | Projects and significant updates | owner, priority, tags |
| `commitment` | Deadlines, promises, deliverables | commitment_type, due_date, stakeholders |
| `decision` | Architectural/business decisions | decision_type, rationale, alternatives |
| `tool` | Software tools and services | tool_type, url, status |
| `concept` | Mental models and frameworks | domain, tags |

## Troubleshooting

- **Plugin not loading**: Check that the directory name is exactly `local.unpacked.bakobiibizo.gargoyle` under `Claude Extensions/`
- **MCP server not connecting**: Verify `gargoyle_mcp_path` in settings points to the correct binary
- **No memories surfacing**: Check `db_path` points to a database with entities. Run `/gargoyle:search-memory test` to verify
- **Hook timeouts**: Search hook has 8s timeout, writer has 12s. If Erasmus is slow, hooks fall back to local processing
- **Python errors**: Ensure `httpx` is installed: `pip install httpx`

## License

MIT

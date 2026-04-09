# Gargoyle Memory Plugin

A Claude Code plugin that provides automatic memory surfacing and accumulation via the gargoyle knowledge graph.

## What it does

- **On every prompt**: Searches gargoyle for relevant entities (FTS + semantic embeddings) and injects them as context
- **After every response**: Extracts noteworthy observations and writes them as typed entities
- **Manual deep search**: `/gargoyle:search-memory <query>` for targeted knowledge graph exploration

## Features

- 21 MCP tools for full knowledge graph management (entities, relations, claims, templates)
- Full-text search (FTS5) and embedding-based semantic similarity search
- BFS graph traversal for exploring entity neighborhoods
- Typed entity system with canonical field schemas per type
- Auto-deduplication within sessions to avoid surfacing repeats
- Provenance tracking with run IDs

## Requirements

- **Gargoyle MCP binary** (Rust) — the core MCP server
- **Python 3.10+** with `httpx` (`pip install httpx`)
- **Gargoyle database** — set `GARGOYLE_DB_PATH` or use default
- **Erasmus inference endpoint** — set `ERASMUS_TEXT_URL` / `ERASMUS_EMBED_URL` or use defaults

## Installation

### Claude Code CLI

```bash
claude --plugin-dir /path/to/this/directory
```

Or add to your `settings.json` `enabledPlugins`.

### Claude Desktop (Cowork)

```bash
bash install.sh --desktop
```

Then drag and drop the generated `.plugin` file into a Claude Desktop conversation.

## Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `GARGOYLE_DB_PATH` | `D:/apps/gargoyle/src-tauri/gargoyle.db` | Path to gargoyle SQLite database |
| `ERASMUS_TEXT_URL` | `https://text-erasmus.ngrok.dev/v1/chat/completions` | Chat completions endpoint |
| `ERASMUS_EMBED_URL` | `https://erasmus.ngrok.dev/embed` | Embedding endpoint |

## Project Structure

```
gargoyle/
  .claude-plugin/     Plugin metadata
    plugin.json
  agents/             Custom agent definitions
    memory-searcher.md
  assets/             Icons and screenshots
  hooks/              Hook event bindings
    hooks.json
  scripts/            Python hook scripts
    memory-search.py
    memory-writer.py
    requirements.txt
  skills/             Skill definitions
    search-memory/
      SKILL.md
  .python-version
  CLAUDE.md
  LICENSE.md
  README.md
  install.sh
  manifest.json
  pyproject.toml
  server.json
```

## Entity Types

| Type | Description | Key Fields |
|---|---|---|
| `note` | Observations, preferences, patterns | context, tags |
| `idea` | Concepts worth exploring | stage, tags, potential_value |
| `task` | Action items and TODOs | assignee, due_date, effort, tags |
| `person` | People with meaningful context | full_name, role, company, relationship |
| `project` | Projects and significant updates | owner, priority, tags |
| `commitment` | Deadlines, promises, deliverables | commitment_type, due_date, stakeholders |

## License

MIT

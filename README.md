# Gargoyle

A personal knowledge graph and AI memory system. Tauri 2 desktop app + MCP server + Claude plugin. Built with Rust, React, and SQLite.

## What It Does

Gargoyle stores structured knowledge as a typed entity graph — people, projects, decisions, notes, ideas, tasks, commitments — connected by typed relations, searchable via full-text and semantic (embedding) search, and accessible to AI agents via the MCP protocol.

Two ways to use it:

1. **Desktop App** — Tauri 2 GUI for browsing, creating, and managing your knowledge graph
2. **Claude Plugin** — Auto-memory hooks that search your graph on every prompt and extract observations after every response

## Features

- **Typed Entity Graph** — Entities with schema-validated canonical fields, status state machines, and optimistic locking
- **20 MCP Tools** — Full entity/relation/claim CRUD, search, graph traversal, templates, batch ops
- **Full-Text + Semantic Search** — FTS5 keyword search and 384-dim BAAI/bge-small-en-v1.5 embeddings
- **Deduplication Pipeline** — Exact title, fuzzy match, and embedding proximity detection
- **Claim Grounding** — Subject-predicate-object triples with evidence entity linking
- **Batch Patch Protocol** — Atomic operations with schema, status, lock, and referential integrity validation
- **Claude Plugin** — Auto-search on every prompt, auto-extract observations after every response
- **Template System** — Markdown templates with frontmatter for guided workflows

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (v20+)
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)
- Python 3.10+ (for Claude plugin hooks)

### Build & Run

```bash
# Install frontend dependencies
npm install

# Run the desktop app in dev mode
npm run tauri dev

# Build the MCP server binary (standalone, no GUI)
cd src-tauri
cargo build --bin gargoyle-mcp --release

# Run tests
cargo test
```

### MCP Server (Claude Code)

Point Claude Code at the MCP server by adding to your project's `.mcp.json`:

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

## Claude Plugin Setup

The Claude plugin adds automatic memory to any Claude Code or Claude Desktop session. It provides:

- **Auto-search hook** — On every prompt, searches your knowledge graph for relevant entities and injects them as context
- **Auto-write hook** — After every response, extracts noteworthy observations and writes them as entities
- **`/gargoyle:search-memory <query>`** — Manual deep search skill
- **Memory searcher agent** — Specialized haiku agent for targeted retrieval

### Install the Plugin (Windows)

The installer builds the MCP binary and copies the plugin to Claude's extensions directory.

```bash
# From the gargoyle repo root:
bash claude-install.sh

# Or skip the cargo build if you already have the binary:
bash claude-install.sh --skip-build
```

This installs to `%APPDATA%/Claude/Claude Extensions/gargoyle/`.

### Configure the Plugin

After installation, configure the plugin settings at:

```
%APPDATA%/Claude/Claude Extensions Settings/local.unpacked.bakobiibizo.gargoyle.json
```

Required settings:

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

| Setting | Required | Description |
|---|---|---|
| `db_path` | Yes | Path to gargoyle SQLite database |
| `gargoyle_mcp_path` | Yes | Path to the gargoyle-mcp binary |
| `erasmus_text_url` | No | OpenAI-compatible chat completions endpoint (for keyword extraction) |
| `erasmus_embed_url` | No | Embedding endpoint (for semantic search) |

### Plugin File Structure

```
gargoyle/                         (in Claude Extensions)
  .claude-plugin/plugin.json      Plugin metadata + MCP server config
  agents/memory-searcher.md       Haiku agent for memory retrieval
  assets/logo.png                 Plugin icon
  hooks/hooks.json                Hook event bindings (UserPromptSubmit, Stop)
  scripts/
    memory-search.py              Prompt hook: search graph, inject context
    memory-writer.py              Stop hook: extract observations, write entities
    requirements.txt              Python deps (httpx)
  skills/search-memory/SKILL.md   Manual /search-memory skill
  manifest.json                   Extension registry metadata
  server.json                     MCP server registry entry
  CLAUDE.md                       Plugin-specific Claude instructions
  README.md                       Plugin documentation
```

### How the Hooks Work

**`memory-search.py`** (runs on `UserPromptSubmit`):
1. Reads the user's prompt
2. Extracts keywords via Erasmus LLM (falls back to stop-word filtering)
3. Queries gargoyle SQLite directly (FTS + embedding similarity)
4. Returns relevant entities as `additionalContext` (capped at 3000 chars)
5. Session dedup prevents surfacing the same entity twice

**`memory-writer.py`** (runs on `Stop`):
1. Reads the conversation transcript tail
2. Asks Erasmus to extract noteworthy observations
3. Validates against entity type schemas
4. Writes new entities to gargoyle SQLite
5. Writer dedup prevents duplicate observations per session

## Architecture

```
src-tauri/
  src/
    services/       Business logic (store, indexer, graph, memory, embeddings)
    models/         Data types (Entity, Relation, Claim, Template, Memory)
    mcp/            MCP server tool wrappers (20 tools)
    commands/       Tauri IPC command handlers (44+ commands)
    agents/         Agent router + 5 specialized agents
    schema/         Entity type definitions + validation
    patch/          Batch patch protocol
    validation/     4-step validation pipeline
    db/             SQLite connection + migrations
  migrations/       8 SQLite migration files
  bin/
    gargoyle_mcp.rs MCP server binary entry point

src/                React frontend (TypeScript + Vite)
plugin/             Claude plugin source files
templates/          Markdown workflow templates
config/             gargoyle.toml (dedup thresholds, indexer, LLM config)
docs/               Specs and architecture docs
```

### Entity Types

| Type | Description | Key Canonical Fields |
|---|---|---|
| `note` | Observations, preferences, patterns | context, tags |
| `idea` | Concepts worth exploring | stage, tags, potential_value |
| `task` | Action items and TODOs | assignee, due_date, effort, tags |
| `person` | People with meaningful context | full_name, role, company, relationship |
| `project` | Projects and initiatives | owner, priority, start_date, tags |
| `commitment` | Deadlines, promises, deliverables | commitment_type, due_date, stakeholders |
| `decision` | Architectural/business decisions | decision_type, rationale, alternatives |
| `tool` | Software tools and services | tool_type, url, status |
| `concept` | Mental models and frameworks | domain, tags |

### Key Design Patterns

- **Optimistic locking** — Updates require `expected_updated_at` to prevent conflicts
- **PatchSet protocol** — Atomic batch operations with provenance tracking via `run_id`
- **Source tracking** — Every entity records its origin: `manual`, `clipboard`, `web`, `import`, `agent`, `template`, `bootstrap`
- **Embeddings** — 384-dim vectors via BAAI/bge-small-en-v1.5 on a remote Erasmus endpoint, with hash-based fallback when unavailable

## Documentation

| Document | Description |
|---|---|
| [`docs/gargoyle-spec-v0.7.md`](docs/gargoyle-spec-v0.7.md) | Core specification |
| [`docs/stress-test-spec.md`](docs/stress-test-spec.md) | Stress test design spec |
| [`docs/AGENT_MEMORY_SPEC.md`](docs/AGENT_MEMORY_SPEC.md) | Memory system specification |
| [`docs/MEMORY_ARCHITECTURE.md`](docs/MEMORY_ARCHITECTURE.md) | Memory architecture design |
| [`PROJECT_VISION.md`](PROJECT_VISION.md) | High-level vision and roadmap |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contribution guidelines |
| [`AGENTS.md`](AGENTS.md) | Agent architecture notes |

## License

[MIT](LICENSE)

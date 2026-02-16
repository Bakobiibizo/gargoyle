# Gargoyle

A knowledge graph desktop application for structured decision-making and expertise management. Built with Tauri 2, Rust, React, and SQLite.

## Overview

Gargoyle organizes domain expertise into a typed entity graph backed by 160+ markdown templates. Entities (metrics, experiments, decisions, projects, etc.) are connected via typed relations, validated against schemas, and searchable via full-text and semantic search.

### Current Status

**Analytics Vertical Slice** — Proving the architecture with 3 entity types (`metric`, `experiment`, `result`) and 3 templates before scaling horizontally.

## Features

- **Typed Entity Graph** — Entities with schema-validated canonical fields, status state machines, and optimistic locking
- **Patch Protocol** — Atomic batch operations with 4-step validation (schema, status, lock, referential integrity)
- **Deduplication Pipeline** — Exact title, fuzzy match, and embedding proximity detection (non-blocking)
- **Full-Text + Semantic Search** — FTS5 for keyword search, embedding vectors for similarity
- **Claim Grounding** — Every assertion must reference an evidence entity
- **Template System** — 160+ expertise templates with prerequisite chains and provenance tracking
- **Schema Versioning** — Entity type schemas evolve with automatic version bumping and migration

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v20+)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
npm install
npm run tauri dev
```

### Testing

```bash
cd src-tauri && cargo test
```

## Architecture

See [`docs/stress-test-spec.md`](docs/stress-test-spec.md) for the full specification.

## License

[MIT](LICENSE)

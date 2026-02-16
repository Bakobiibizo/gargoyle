# Contributing to Gargoyle

Thanks for your interest in contributing to Gargoyle.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (v20+)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

### Setup

```bash
git clone https://github.com/bakobiibizo/gargoyle.git
cd gargoyle
npm install
cd src-tauri && cargo build && cd ..
npm run tauri dev
```

### Running Tests

```bash
# Rust backend tests
cd src-tauri && cargo test

# Frontend build check
npm run build
```

## Development Workflow

1. Fork the repo and create a feature branch from `main`
2. Make your changes
3. Ensure `cargo test` passes with no failures
4. Ensure `npm run build` succeeds
5. Submit a pull request

## Architecture

Gargoyle follows a phased architecture. See `docs/stress-test-spec.md` for the design specification.

- **Backend:** Rust (Tauri 2.x) with SQLite via rusqlite
- **Frontend:** React + TypeScript + Vite
- **Testing:** proptest for property-based fuzz testing, integration tests for each scenario

### Key directories

```
src-tauri/src/          Rust backend
  models/               Data structures
  schema/               Entity type definitions + validation
  patch/                Patch protocol (create/update operations)
  validation/           4-step validation pipeline
  services/             Business logic (store, indexer, dedup, etc.)
  commands/             Tauri IPC command handlers
  db/                   SQLite connection + migrations

src/                    React frontend
  api/                  Tauri invoke wrappers
  types/                TypeScript type definitions

templates/              Markdown expertise templates
tests/                  Integration + fuzz tests
```

## Code Style

- Rust: follow `cargo fmt` and `cargo clippy` defaults
- TypeScript: follow the ESLint config in the repo
- Commit messages: imperative mood, concise

## Reporting Issues

Open an issue on GitHub with:
- What you expected
- What happened instead
- Steps to reproduce

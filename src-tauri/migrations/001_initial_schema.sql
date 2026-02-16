-- Gargoyle Initial Schema (Analytics Vertical Slice)
-- Phase 1: Full DDL from stress-test-spec Section 1.1

-- Core tables
CREATE TABLE IF NOT EXISTS entities (
  id TEXT PRIMARY KEY,                    -- uuid
  entity_type TEXT NOT NULL,
  category TEXT,
  title TEXT NOT NULL,
  body_md TEXT DEFAULT '',
  status TEXT,
  priority INTEGER CHECK (priority BETWEEN 0 AND 4),
  due_at TEXT,                            -- ISO 8601
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  source TEXT NOT NULL CHECK (source IN ('manual','clipboard','web','import','agent','template','bootstrap')),
  canonical_fields TEXT NOT NULL DEFAULT '{}',  -- JSON
  _schema_version INTEGER NOT NULL DEFAULT 1,
  provenance_run_id TEXT,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS relations (
  id TEXT PRIMARY KEY,
  from_id TEXT NOT NULL REFERENCES entities(id),
  to_id TEXT NOT NULL REFERENCES entities(id),
  relation_type TEXT NOT NULL,
  weight REAL DEFAULT 1.0,
  confidence REAL CHECK (confidence IS NULL OR (confidence >= 0.0 AND confidence <= 1.0)),
  provenance_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS runs (
  run_id TEXT PRIMARY KEY,
  template_key TEXT NOT NULL,
  template_version TEXT NOT NULL,
  template_category TEXT NOT NULL,
  inputs_snapshot TEXT NOT NULL DEFAULT '{}',   -- JSON
  outputs_snapshot TEXT NOT NULL DEFAULT '{}',  -- JSON
  patch_set TEXT NOT NULL DEFAULT '[]',         -- JSON
  status TEXT NOT NULL CHECK (status IN ('pending','applied','rejected','partial')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS artifacts (
  artifact_id TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL REFERENCES entities(id),
  kind TEXT NOT NULL CHECK (kind IN ('attachment','link','export','rendered_doc')),
  uri_or_path TEXT NOT NULL,
  hash TEXT,
  mime TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS operational_context (
  context_id TEXT PRIMARY KEY,
  context_key TEXT NOT NULL UNIQUE,
  context_value TEXT NOT NULL,             -- JSON
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_by_run_id TEXT
);

CREATE TABLE IF NOT EXISTS claims (
  claim_id TEXT PRIMARY KEY,
  subject TEXT NOT NULL,
  predicate TEXT NOT NULL,
  object TEXT NOT NULL,
  confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
  evidence_entity_id TEXT NOT NULL REFERENCES entities(id),
  provenance_run_id TEXT,
  promoted_to_entity_id TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS embeddings (
  embedding_id TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL REFERENCES entities(id),
  model TEXT NOT NULL,
  vector BLOB NOT NULL,
  dimensions INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS custom_relation_types (
  type_key TEXT PRIMARY KEY CHECK (type_key LIKE 'custom:%'),
  description TEXT NOT NULL,
  expected_from_types TEXT,                -- JSON array
  expected_to_types TEXT,                  -- JSON array
  proposed_by_run_id TEXT,
  approved_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dedup_suggestions (
  suggestion_id TEXT PRIMARY KEY,
  new_entity_id TEXT NOT NULL REFERENCES entities(id),
  existing_entity_id TEXT NOT NULL REFERENCES entities(id),
  detection_method TEXT NOT NULL CHECK (detection_method IN ('exact_title','fuzzy_title','embedding_proximity')),
  confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','accepted','dismissed')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Performance indexes (Analytics slice)
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_metric ON entities(status) WHERE entity_type = 'metric' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_experiment ON entities(status) WHERE entity_type = 'experiment' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_result ON entities(status) WHERE entity_type = 'result' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_provenance ON entities(provenance_run_id) WHERE provenance_run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_id, relation_type);
CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_id, relation_type);
CREATE INDEX IF NOT EXISTS idx_relations_provenance ON relations(provenance_run_id) WHERE provenance_run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_embeddings_entity ON embeddings(entity_id);
CREATE INDEX IF NOT EXISTS idx_dedup_status ON dedup_suggestions(status) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_claims_evidence ON claims(evidence_entity_id);

-- FTS5
CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(title, body_md, content=entities, content_rowid=rowid);

-- Wave 4: New Entity Type Indexes (spec v0.7 alignment)
-- Adds filtered indexes for inbox_item, artifact_type, concept, commitment, issue

CREATE INDEX IF NOT EXISTS idx_entities_inbox_item ON entities(status) WHERE entity_type = 'inbox_item' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_artifact_type ON entities(status) WHERE entity_type = 'artifact_type' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_concept ON entities(status) WHERE entity_type = 'concept' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_commitment ON entities(status) WHERE entity_type = 'commitment' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_issue ON entities(status) WHERE entity_type = 'issue' AND deleted_at IS NULL;

-- NOTE: deleted_at column on relations is added programmatically in migrations.rs
-- because SQLite ALTER TABLE ADD COLUMN is not idempotent.

-- Allow custom_relation_types.approved_at to be NULL for pending proposals (spec §2.6.2).
-- SQLite does not support ALTER COLUMN, so we recreate the table.
CREATE TABLE IF NOT EXISTS custom_relation_types_new (
  type_key TEXT PRIMARY KEY CHECK (type_key LIKE 'custom:%'),
  description TEXT NOT NULL,
  expected_from_types TEXT,
  expected_to_types TEXT,
  proposed_by_run_id TEXT,
  approved_at TEXT
);
INSERT OR IGNORE INTO custom_relation_types_new SELECT * FROM custom_relation_types;
DROP TABLE IF EXISTS custom_relation_types;
ALTER TABLE custom_relation_types_new RENAME TO custom_relation_types;

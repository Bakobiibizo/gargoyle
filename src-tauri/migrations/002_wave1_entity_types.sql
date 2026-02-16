-- Wave 1: Core Entity Type Indexes
-- Adds filtered indexes for task, project, decision, person, note, session

CREATE INDEX IF NOT EXISTS idx_entities_task ON entities(status) WHERE entity_type = 'task' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_project ON entities(status) WHERE entity_type = 'project' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_decision ON entities(status) WHERE entity_type = 'decision' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_person ON entities(status) WHERE entity_type = 'person' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_note ON entities(status) WHERE entity_type = 'note' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_session ON entities(status) WHERE entity_type = 'session' AND deleted_at IS NULL;

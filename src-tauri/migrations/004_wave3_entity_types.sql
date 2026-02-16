-- Wave 3: Domain Entity Type Indexes
-- Adds filtered indexes for taxonomy, backlog, brief, event, policy

CREATE INDEX IF NOT EXISTS idx_entities_taxonomy ON entities(status) WHERE entity_type = 'taxonomy' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_backlog ON entities(status) WHERE entity_type = 'backlog' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_brief ON entities(status) WHERE entity_type = 'brief' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_event ON entities(status) WHERE entity_type = 'event' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_policy ON entities(status) WHERE entity_type = 'policy' AND deleted_at IS NULL;

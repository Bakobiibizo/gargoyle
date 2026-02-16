-- Wave 2: Domain Entity Type Indexes
-- Adds filtered indexes for campaign, audience, competitor, channel, spec, budget, vendor, playbook

CREATE INDEX IF NOT EXISTS idx_entities_campaign ON entities(status) WHERE entity_type = 'campaign' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_audience ON entities(status) WHERE entity_type = 'audience' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_competitor ON entities(status) WHERE entity_type = 'competitor' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_channel ON entities(status) WHERE entity_type = 'channel' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_spec ON entities(status) WHERE entity_type = 'spec' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_budget ON entities(status) WHERE entity_type = 'budget' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_vendor ON entities(status) WHERE entity_type = 'vendor' AND deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_entities_playbook ON entities(status) WHERE entity_type = 'playbook' AND deleted_at IS NULL;

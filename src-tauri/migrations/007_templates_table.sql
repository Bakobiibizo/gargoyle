-- Templates table: stores prompt templates in the database
-- Replaces the markdown-files-in-folders approach

CREATE TABLE IF NOT EXISTS templates (
  id TEXT PRIMARY KEY,                      -- uuid
  key TEXT NOT NULL UNIQUE,                 -- e.g., "initialize", "weekly-review"
  version TEXT NOT NULL DEFAULT '1.0',
  category TEXT NOT NULL,                   -- e.g., "bootstrap", "workflow", "analysis"
  description TEXT,                         -- Short description for index/search
  
  -- Template content
  content TEXT NOT NULL,                    -- The actual prompt/instructions (markdown)
  response_format TEXT,                     -- "structured", "freeform", "mixed"
  
  -- What this template produces (for validation)
  produces_entities TEXT DEFAULT '[]',      -- JSON array of entity types
  produces_relations TEXT DEFAULT '[]',     -- JSON array of relation types
  
  -- Generator config (if template creates entities)
  generator_type TEXT,                      -- "generic", "custom", or NULL for prompt-only
  generator_config TEXT DEFAULT '{}',       -- JSON config for generic generator
  
  -- Metadata
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  created_by TEXT,                          -- "system", "user", "agent"
  usage_count INTEGER NOT NULL DEFAULT 0,
  last_used_at TEXT,
  
  -- Soft delete
  deleted_at TEXT
);

-- Indexes for fast lookup
CREATE INDEX IF NOT EXISTS idx_templates_key ON templates(key) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_templates_category ON templates(category) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_templates_usage ON templates(usage_count DESC) WHERE deleted_at IS NULL;

-- FTS for template search
CREATE VIRTUAL TABLE IF NOT EXISTS templates_fts USING fts5(
  key, 
  category, 
  description, 
  content, 
  content=templates, 
  content_rowid=rowid
);

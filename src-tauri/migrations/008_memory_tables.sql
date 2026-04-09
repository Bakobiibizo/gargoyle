-- Memory System Tables
-- Supports conversations, segments, short-term and long-term memories

-- ============================================================================
-- Conversations: Track chat sessions
-- ============================================================================
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    title TEXT,
    summary TEXT,
    message_count INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_conversations_started_at ON conversations(started_at);

-- ============================================================================
-- Conversation Segments: Individual messages within conversations
-- ============================================================================
CREATE TABLE IF NOT EXISTS conversation_segments (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    token_count INTEGER,
    metadata TEXT DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_segments_conversation ON conversation_segments(conversation_id);
CREATE INDEX IF NOT EXISTS idx_segments_created_at ON conversation_segments(created_at);

-- FTS for conversation segments
CREATE VIRTUAL TABLE IF NOT EXISTS conversation_segments_fts USING fts5(
    content,
    content='conversation_segments',
    content_rowid='rowid'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS conversation_segments_ai AFTER INSERT ON conversation_segments BEGIN
    INSERT INTO conversation_segments_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

CREATE TRIGGER IF NOT EXISTS conversation_segments_ad AFTER DELETE ON conversation_segments BEGIN
    INSERT INTO conversation_segments_fts(conversation_segments_fts, rowid, content) VALUES ('delete', OLD.rowid, OLD.content);
END;

CREATE TRIGGER IF NOT EXISTS conversation_segments_au AFTER UPDATE ON conversation_segments BEGIN
    INSERT INTO conversation_segments_fts(conversation_segments_fts, rowid, content) VALUES ('delete', OLD.rowid, OLD.content);
    INSERT INTO conversation_segments_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

-- ============================================================================
-- Short-Term Memories: Recent observations with decay
-- ============================================================================
CREATE TABLE IF NOT EXISTS short_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL CHECK (memory_type IN ('observation', 'insight', 'fact', 'preference', 'task')),
    source_conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
    source_segment_id TEXT REFERENCES conversation_segments(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_accessed_at TEXT NOT NULL DEFAULT (datetime('now')),
    access_count INTEGER DEFAULT 1,
    relevance_score REAL DEFAULT 1.0,
    decay_rate REAL DEFAULT 0.1,
    expires_at TEXT,
    promoted_to_ltm_id TEXT,
    metadata TEXT DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_stm_type ON short_term_memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_stm_created ON short_term_memories(created_at);
CREATE INDEX IF NOT EXISTS idx_stm_relevance ON short_term_memories(relevance_score DESC);
CREATE INDEX IF NOT EXISTS idx_stm_expires ON short_term_memories(expires_at);

-- FTS for short-term memories
CREATE VIRTUAL TABLE IF NOT EXISTS short_term_memories_fts USING fts5(
    content,
    content='short_term_memories',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS stm_ai AFTER INSERT ON short_term_memories BEGIN
    INSERT INTO short_term_memories_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

CREATE TRIGGER IF NOT EXISTS stm_ad AFTER DELETE ON short_term_memories BEGIN
    INSERT INTO short_term_memories_fts(short_term_memories_fts, rowid, content) VALUES ('delete', OLD.rowid, OLD.content);
END;

CREATE TRIGGER IF NOT EXISTS stm_au AFTER UPDATE ON short_term_memories BEGIN
    INSERT INTO short_term_memories_fts(short_term_memories_fts, rowid, content) VALUES ('delete', OLD.rowid, OLD.content);
    INSERT INTO short_term_memories_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

-- ============================================================================
-- Long-Term Memories: Consolidated, persistent knowledge
-- ============================================================================
CREATE TABLE IF NOT EXISTS long_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL CHECK (memory_type IN ('fact', 'preference', 'pattern', 'relationship', 'skill', 'context')),
    category TEXT,
    importance REAL DEFAULT 0.5,
    confidence REAL DEFAULT 1.0,
    source_stm_ids TEXT DEFAULT '[]',  -- JSON array of STM IDs that contributed
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_accessed_at TEXT NOT NULL DEFAULT (datetime('now')),
    access_count INTEGER DEFAULT 1,
    embedding BLOB,  -- Float32 array for vector search
    embedding_model TEXT,
    metadata TEXT DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_ltm_type ON long_term_memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_ltm_category ON long_term_memories(category);
CREATE INDEX IF NOT EXISTS idx_ltm_importance ON long_term_memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_ltm_created ON long_term_memories(created_at);

-- FTS for long-term memories
CREATE VIRTUAL TABLE IF NOT EXISTS long_term_memories_fts USING fts5(
    content,
    category,
    content='long_term_memories',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS ltm_ai AFTER INSERT ON long_term_memories BEGIN
    INSERT INTO long_term_memories_fts(rowid, content, category) VALUES (NEW.rowid, NEW.content, COALESCE(NEW.category, ''));
END;

CREATE TRIGGER IF NOT EXISTS ltm_ad AFTER DELETE ON long_term_memories BEGIN
    INSERT INTO long_term_memories_fts(long_term_memories_fts, rowid, content, category) VALUES ('delete', OLD.rowid, OLD.content, COALESCE(OLD.category, ''));
END;

CREATE TRIGGER IF NOT EXISTS ltm_au AFTER UPDATE ON long_term_memories BEGIN
    INSERT INTO long_term_memories_fts(long_term_memories_fts, rowid, content, category) VALUES ('delete', OLD.rowid, OLD.content, COALESCE(OLD.category, ''));
    INSERT INTO long_term_memories_fts(rowid, content, category) VALUES (NEW.rowid, NEW.content, COALESCE(NEW.category, ''));
END;

-- ============================================================================
-- Memory Links: Connect memories to entities
-- ============================================================================
CREATE TABLE IF NOT EXISTS memory_entity_links (
    memory_id TEXT NOT NULL,
    memory_table TEXT NOT NULL CHECK (memory_table IN ('short_term_memories', 'long_term_memories')),
    entity_id TEXT NOT NULL,
    link_type TEXT DEFAULT 'related',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (memory_id, memory_table, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_mel_entity ON memory_entity_links(entity_id);
CREATE INDEX IF NOT EXISTS idx_mel_memory ON memory_entity_links(memory_id, memory_table);

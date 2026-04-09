use crate::models::memory::{
    Conversation, ConversationSegment, LongTermMemory, LongTermMemoryType, MemoryEntityLink,
    MemorySearchResult, MessageRole, ShortTermMemory, ShortTermMemoryType,
};
use crate::services::embeddings::{EmbeddingError, ErasmusEmbeddings};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct MemoryService {
    conn: Arc<Mutex<Connection>>,
    embeddings: ErasmusEmbeddings,
    enabled: bool,
}

impl MemoryService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            embeddings: ErasmusEmbeddings::new(None, None),
            enabled: true,
        }
    }

    pub fn with_embeddings(conn: Arc<Mutex<Connection>>, embeddings: ErasmusEmbeddings) -> Self {
        Self {
            conn,
            embeddings,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // =========================================================================
    // Conversation Operations
    // =========================================================================

    pub fn create_conversation(&self) -> Result<Conversation, MemoryError> {
        self.create_conversation_with_id(None)
    }

    pub fn create_conversation_with_id(
        &self,
        id: Option<String>,
    ) -> Result<Conversation, MemoryError> {
        if !self.enabled {
            return Ok(Conversation::new());
        }

        let conversation = match id {
            Some(custom_id) => Conversation {
                id: custom_id,
                started_at: Utc::now(),
                ended_at: None,
                title: None,
                summary: None,
                message_count: 0,
                metadata: std::collections::HashMap::new(),
            },
            None => Conversation::new(),
        };
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO conversations (id, started_at, message_count, metadata) VALUES (?1, ?2, ?3, ?4)",
            params![
                conversation.id,
                conversation.started_at.to_rfc3339(),
                conversation.message_count,
                serde_json::to_string(&conversation.metadata)?
            ],
        )?;

        Ok(conversation)
    }

    pub fn get_conversation(&self, id: &str) -> Result<Option<Conversation>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                "SELECT id, started_at, ended_at, title, summary, message_count, metadata 
                 FROM conversations WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        started_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        ended_at: row
                            .get::<_, Option<String>>(2)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                        title: row.get(3)?,
                        summary: row.get(4)?,
                        message_count: row.get(5)?,
                        metadata: row
                            .get::<_, Option<String>>(6)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn end_conversation(&self, id: &str, summary: Option<String>) -> Result<(), MemoryError> {
        if !self.enabled {
            return Ok(());
        }

        let conn = self.conn.lock().unwrap();
        let ended_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE conversations SET ended_at = ?1, summary = ?2 WHERE id = ?3",
            params![ended_at, summary, id],
        )?;

        Ok(())
    }

    pub fn list_recent_conversations(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, started_at, ended_at, title, summary, message_count, metadata 
             FROM conversations ORDER BY started_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                started_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                ended_at: row
                    .get::<_, Option<String>>(2)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                title: row.get(3)?,
                summary: row.get(4)?,
                message_count: row.get(5)?,
                metadata: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // Conversation Segment Operations
    // =========================================================================

    pub fn add_segment(
        &self,
        conversation_id: &str,
        role: MessageRole,
        content: String,
        token_count: Option<i32>,
    ) -> Result<ConversationSegment, MemoryError> {
        if !self.enabled {
            return Ok(ConversationSegment::new(
                conversation_id.to_string(),
                role,
                content,
            ));
        }

        let segment = ConversationSegment {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            role,
            content,
            created_at: Utc::now(),
            token_count,
            metadata: HashMap::new(),
        };

        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO conversation_segments (id, conversation_id, role, content, created_at, token_count, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                segment.id,
                segment.conversation_id,
                segment.role.to_string(),
                segment.content,
                segment.created_at.to_rfc3339(),
                segment.token_count,
                "{}"
            ],
        )?;

        conn.execute(
            "UPDATE conversations SET message_count = message_count + 1 WHERE id = ?1",
            params![conversation_id],
        )?;

        Ok(segment)
    }

    pub fn get_conversation_segments(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ConversationSegment>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, token_count, metadata
             FROM conversation_segments WHERE conversation_id = ?1 ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map(params![conversation_id], |row| {
            Ok(ConversationSegment {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row
                    .get::<_, String>(2)?
                    .parse()
                    .unwrap_or(MessageRole::User),
                content: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                token_count: row.get(5)?,
                metadata: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn search_segments(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ConversationSegment>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT cs.id, cs.conversation_id, cs.role, cs.content, cs.created_at, cs.token_count, cs.metadata
             FROM conversation_segments cs
             JOIN conversation_segments_fts fts ON cs.rowid = fts.rowid
             WHERE conversation_segments_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(ConversationSegment {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row
                    .get::<_, String>(2)?
                    .parse()
                    .unwrap_or(MessageRole::User),
                content: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                token_count: row.get(5)?,
                metadata: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // Short-Term Memory Operations
    // =========================================================================

    pub fn create_stm(&self, memory: ShortTermMemory) -> Result<ShortTermMemory, MemoryError> {
        if !self.enabled {
            return Ok(memory);
        }

        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO short_term_memories 
             (id, content, memory_type, source_conversation_id, source_segment_id, 
              created_at, last_accessed_at, access_count, relevance_score, decay_rate, expires_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                memory.id,
                memory.content,
                memory.memory_type.to_string(),
                memory.source_conversation_id,
                memory.source_segment_id,
                memory.created_at.to_rfc3339(),
                memory.last_accessed_at.to_rfc3339(),
                memory.access_count,
                memory.relevance_score,
                memory.decay_rate,
                memory.expires_at.map(|dt| dt.to_rfc3339()),
                serde_json::to_string(&memory.metadata)?
            ],
        )?;

        Ok(memory)
    }

    pub fn get_stm(&self, id: &str) -> Result<Option<ShortTermMemory>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                "SELECT id, content, memory_type, source_conversation_id, source_segment_id,
                        created_at, last_accessed_at, access_count, relevance_score, decay_rate,
                        expires_at, promoted_to_ltm_id, metadata
                 FROM short_term_memories WHERE id = ?1",
                params![id],
                |row| Self::row_to_stm(row),
            )
            .optional()?;

        Ok(result)
    }

    pub fn search_stm(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ShortTermMemory>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT stm.id, stm.content, stm.memory_type, stm.source_conversation_id, stm.source_segment_id,
                    stm.created_at, stm.last_accessed_at, stm.access_count, stm.relevance_score, stm.decay_rate,
                    stm.expires_at, stm.promoted_to_ltm_id, stm.metadata
             FROM short_term_memories stm
             JOIN short_term_memories_fts fts ON stm.rowid = fts.rowid
             WHERE short_term_memories_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit as i64], |row| Self::row_to_stm(row))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_recent_stm(
        &self,
        limit: usize,
        memory_type: Option<ShortTermMemoryType>,
    ) -> Result<Vec<ShortTermMemory>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        match memory_type {
            Some(mt) => {
                let mut stmt = conn.prepare(
                    "SELECT id, content, memory_type, source_conversation_id, source_segment_id,
                            created_at, last_accessed_at, access_count, relevance_score, decay_rate,
                            expires_at, promoted_to_ltm_id, metadata
                     FROM short_term_memories 
                     WHERE memory_type = ?1 AND promoted_to_ltm_id IS NULL
                     ORDER BY created_at DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![mt.to_string(), limit as i64], |row| {
                    Self::row_to_stm(row)
                })?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, content, memory_type, source_conversation_id, source_segment_id,
                            created_at, last_accessed_at, access_count, relevance_score, decay_rate,
                            expires_at, promoted_to_ltm_id, metadata
                     FROM short_term_memories 
                     WHERE promoted_to_ltm_id IS NULL
                     ORDER BY created_at DESC LIMIT ?1",
                )?;
                let rows = stmt.query_map(params![limit as i64], |row| Self::row_to_stm(row))?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
        }
    }

    pub fn touch_stm(&self, id: &str) -> Result<(), MemoryError> {
        if !self.enabled {
            return Ok(());
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE short_term_memories 
             SET last_accessed_at = ?1, access_count = access_count + 1 
             WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn delete_stm(&self, id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM short_term_memories WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_stm(row: &rusqlite::Row) -> rusqlite::Result<ShortTermMemory> {
        Ok(ShortTermMemory {
            id: row.get(0)?,
            content: row.get(1)?,
            memory_type: row
                .get::<_, String>(2)?
                .parse()
                .unwrap_or(ShortTermMemoryType::Observation),
            source_conversation_id: row.get(3)?,
            source_segment_id: row.get(4)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_accessed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            access_count: row.get(7)?,
            relevance_score: row.get(8)?,
            decay_rate: row.get(9)?,
            expires_at: row
                .get::<_, Option<String>>(10)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            promoted_to_ltm_id: row.get(11)?,
            metadata: row
                .get::<_, Option<String>>(12)?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        })
    }

    // =========================================================================
    // Long-Term Memory Operations
    // =========================================================================

    pub fn create_ltm(&self, memory: LongTermMemory) -> Result<LongTermMemory, MemoryError> {
        if !self.enabled {
            return Ok(memory);
        }

        let conn = self.conn.lock().unwrap();

        let embedding_blob = memory
            .embedding
            .as_ref()
            .map(|e| ErasmusEmbeddings::embedding_to_blob(e));

        conn.execute(
            "INSERT INTO long_term_memories 
             (id, content, memory_type, category, importance, confidence, source_stm_ids,
              created_at, updated_at, last_accessed_at, access_count, embedding, embedding_model, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                memory.id,
                memory.content,
                memory.memory_type.to_string(),
                memory.category,
                memory.importance,
                memory.confidence,
                serde_json::to_string(&memory.source_stm_ids)?,
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
                memory.last_accessed_at.to_rfc3339(),
                memory.access_count,
                embedding_blob,
                memory.embedding_model,
                serde_json::to_string(&memory.metadata)?
            ],
        )?;

        Ok(memory)
    }

    pub fn get_ltm(&self, id: &str) -> Result<Option<LongTermMemory>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                "SELECT id, content, memory_type, category, importance, confidence, source_stm_ids,
                        created_at, updated_at, last_accessed_at, access_count, embedding, embedding_model, metadata
                 FROM long_term_memories WHERE id = ?1",
                params![id],
                |row| Self::row_to_ltm(row),
            )
            .optional()?;

        Ok(result)
    }

    pub fn search_ltm(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LongTermMemory>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ltm.id, ltm.content, ltm.memory_type, ltm.category, ltm.importance, ltm.confidence, 
                    ltm.source_stm_ids, ltm.created_at, ltm.updated_at, ltm.last_accessed_at, 
                    ltm.access_count, ltm.embedding, ltm.embedding_model, ltm.metadata
             FROM long_term_memories ltm
             JOIN long_term_memories_fts fts ON ltm.rowid = fts.rowid
             WHERE long_term_memories_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit as i64], |row| Self::row_to_ltm(row))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_ltm_by_category(
        &self,
        category: &str,
        limit: usize,
    ) -> Result<Vec<LongTermMemory>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, memory_type, category, importance, confidence, source_stm_ids,
                    created_at, updated_at, last_accessed_at, access_count, embedding, embedding_model, metadata
             FROM long_term_memories 
             WHERE category = ?1
             ORDER BY importance DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![category, limit as i64], |row| Self::row_to_ltm(row))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn touch_ltm(&self, id: &str) -> Result<(), MemoryError> {
        if !self.enabled {
            return Ok(());
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE long_term_memories 
             SET last_accessed_at = ?1, access_count = access_count + 1 
             WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub async fn embed_ltm(&self, id: &str) -> Result<(), MemoryError> {
        if !self.enabled {
            return Ok(());
        }

        let content = {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT content FROM long_term_memories WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )?
        };

        let result = self.embeddings.embed(&content).await?;
        let blob = ErasmusEmbeddings::embedding_to_blob(&result.embedding);

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE long_term_memories SET embedding = ?1, embedding_model = ?2 WHERE id = ?3",
            params![blob, result.model, id],
        )?;

        Ok(())
    }

    fn row_to_ltm(row: &rusqlite::Row) -> rusqlite::Result<LongTermMemory> {
        let embedding_blob: Option<Vec<u8>> = row.get(11)?;
        let embedding = embedding_blob.map(|b| ErasmusEmbeddings::blob_to_embedding(&b));

        Ok(LongTermMemory {
            id: row.get(0)?,
            content: row.get(1)?,
            memory_type: row
                .get::<_, String>(2)?
                .parse()
                .unwrap_or(LongTermMemoryType::Fact),
            category: row.get(3)?,
            importance: row.get(4)?,
            confidence: row.get(5)?,
            source_stm_ids: row
                .get::<_, Option<String>>(6)?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_accessed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            access_count: row.get(10)?,
            embedding,
            embedding_model: row.get(12)?,
            metadata: row
                .get::<_, Option<String>>(13)?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        })
    }

    // =========================================================================
    // Unified Search
    // =========================================================================

    pub fn search_all(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, MemoryError> {
        let mut results = Vec::new();

        let stm_results = self.search_stm(query, limit)?;
        for stm in stm_results {
            results.push(MemorySearchResult {
                memory_id: stm.id.clone(),
                content: stm.content.clone(),
                memory_type: stm.memory_type.to_string(),
                score: stm.current_relevance(),
                source: "short_term".to_string(),
            });
        }

        let ltm_results = self.search_ltm(query, limit)?;
        for ltm in ltm_results {
            results.push(MemorySearchResult {
                memory_id: ltm.id.clone(),
                content: ltm.content.clone(),
                memory_type: ltm.memory_type.to_string(),
                score: ltm.importance,
                source: "long_term".to_string(),
            });
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        Ok(results)
    }

    // =========================================================================
    // Memory-Entity Links
    // =========================================================================

    pub fn link_memory_to_entity(
        &self,
        memory_id: &str,
        memory_table: &str,
        entity_id: &str,
        link_type: &str,
    ) -> Result<(), MemoryError> {
        if !self.enabled {
            return Ok(());
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memory_entity_links (memory_id, memory_table, entity_id, link_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![memory_id, memory_table, entity_id, link_type, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_entity_memories(
        &self,
        entity_id: &str,
    ) -> Result<Vec<MemoryEntityLink>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT memory_id, memory_table, entity_id, link_type, created_at
             FROM memory_entity_links WHERE entity_id = ?1",
        )?;

        let rows = stmt.query_map(params![entity_id], |row| {
            Ok(MemoryEntityLink {
                memory_id: row.get(0)?,
                memory_table: row.get(1)?,
                entity_id: row.get(2)?,
                link_type: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // Promotion: STM -> LTM
    // =========================================================================

    pub fn promote_to_ltm(
        &self,
        stm_id: &str,
        ltm_type: LongTermMemoryType,
        category: Option<String>,
    ) -> Result<LongTermMemory, MemoryError> {
        let stm = self
            .get_stm(stm_id)?
            .ok_or_else(|| MemoryError::NotFound(format!("STM not found: {}", stm_id)))?;

        let mut ltm = LongTermMemory::from_stm(&stm, ltm_type);
        ltm.category = category;

        let ltm = self.create_ltm(ltm)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE short_term_memories SET promoted_to_ltm_id = ?1 WHERE id = ?2",
            params![ltm.id, stm_id],
        )?;

        Ok(ltm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(include_str!("../../migrations/008_memory_tables.sql"))
            .unwrap();

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_conversation_lifecycle() {
        let conn = setup_test_db();
        let service = MemoryService::new(conn);

        let conv = service.create_conversation().unwrap();
        assert!(!conv.id.is_empty());

        let retrieved = service.get_conversation(&conv.id).unwrap().unwrap();
        assert_eq!(retrieved.id, conv.id);

        service
            .end_conversation(&conv.id, Some("Test summary".to_string()))
            .unwrap();
        let ended = service.get_conversation(&conv.id).unwrap().unwrap();
        assert!(ended.ended_at.is_some());
        assert_eq!(ended.summary, Some("Test summary".to_string()));
    }

    #[test]
    fn test_segment_operations() {
        let conn = setup_test_db();
        let service = MemoryService::new(conn);

        let conv = service.create_conversation().unwrap();

        service
            .add_segment(&conv.id, MessageRole::User, "Hello".to_string(), None)
            .unwrap();
        service
            .add_segment(
                &conv.id,
                MessageRole::Assistant,
                "Hi there!".to_string(),
                None,
            )
            .unwrap();

        let segments = service.get_conversation_segments(&conv.id).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].role, MessageRole::User);
        assert_eq!(segments[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_stm_crud() {
        let conn = setup_test_db();
        let service = MemoryService::new(conn);

        let stm = ShortTermMemory::new(
            "Test observation".to_string(),
            ShortTermMemoryType::Observation,
        );
        let created = service.create_stm(stm.clone()).unwrap();

        let retrieved = service.get_stm(&created.id).unwrap().unwrap();
        assert_eq!(retrieved.content, "Test observation");

        service.touch_stm(&created.id).unwrap();
        let touched = service.get_stm(&created.id).unwrap().unwrap();
        assert_eq!(touched.access_count, 2);

        service.delete_stm(&created.id).unwrap();
        assert!(service.get_stm(&created.id).unwrap().is_none());
    }

    #[test]
    fn test_ltm_crud() {
        let conn = setup_test_db();
        let service = MemoryService::new(conn);

        let ltm = LongTermMemory::new("Important fact".to_string(), LongTermMemoryType::Fact);
        let created = service.create_ltm(ltm).unwrap();

        let retrieved = service.get_ltm(&created.id).unwrap().unwrap();
        assert_eq!(retrieved.content, "Important fact");
    }

    #[test]
    fn test_stm_to_ltm_promotion() {
        let conn = setup_test_db();
        let service = MemoryService::new(conn);

        let stm = ShortTermMemory::new(
            "Observed pattern".to_string(),
            ShortTermMemoryType::Observation,
        );
        let stm = service.create_stm(stm).unwrap();

        let ltm = service
            .promote_to_ltm(
                &stm.id,
                LongTermMemoryType::Pattern,
                Some("behavior".to_string()),
            )
            .unwrap();

        assert_eq!(ltm.content, "Observed pattern");
        assert_eq!(ltm.category, Some("behavior".to_string()));

        let updated_stm = service.get_stm(&stm.id).unwrap().unwrap();
        assert_eq!(updated_stm.promoted_to_ltm_id, Some(ltm.id));
    }
}

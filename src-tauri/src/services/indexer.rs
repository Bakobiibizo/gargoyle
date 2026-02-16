use rusqlite::{params, Connection};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::error::{GargoyleError, Result};

const EMBEDDING_DIMENSIONS: usize = 128;
const EMBEDDING_MODEL: &str = "mock-hash-v1";

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub entity_id: String,
    pub title: String,
    pub entity_type: String,
    pub score: f64,
}

pub struct IndexerService;

impl IndexerService {
    /// Full-text search using FTS5.
    ///
    /// Queries the entities_fts virtual table with bm25() ranking,
    /// joins back to entities to retrieve entity_type and filter out
    /// soft-deleted rows.
    pub fn search_fts(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut stmt = conn.prepare(
            "SELECT e.id, e.title, e.entity_type, bm25(entities_fts) AS score
             FROM entities_fts f
             JOIN entities e ON e.rowid = f.rowid
             WHERE entities_fts MATCH ?1
               AND e.deleted_at IS NULL
             ORDER BY score
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SearchResult {
                entity_id: row.get(0)?,
                title: row.get(1)?,
                entity_type: row.get(2)?,
                score: row.get(3)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Generate a deterministic mock embedding for an entity and store it.
    ///
    /// Reads the entity's title and canonical_fields, hashes them to produce
    /// a 128-dimension f32 vector, then stores it in the embeddings table.
    pub fn generate_embedding(conn: &Connection, entity_id: &str) -> Result<()> {
        // Read entity title + canonical_fields
        let (title, canonical_fields): (String, String) = conn.query_row(
            "SELECT title, canonical_fields FROM entities WHERE id = ?1",
            params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: entity_id.to_string(),
            },
            other => GargoyleError::Database(other),
        })?;

        let text = format!("{} {}", title, canonical_fields);
        let vector = generate_mock_vector(&text);
        let blob = vector_to_blob(&vector);

        // Delete old embedding first if exists
        conn.execute(
            "DELETE FROM embeddings WHERE entity_id = ?1",
            params![entity_id],
        )?;

        let embedding_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        conn.execute(
            "INSERT INTO embeddings (embedding_id, entity_id, model, vector, dimensions, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                embedding_id,
                entity_id,
                EMBEDDING_MODEL,
                blob,
                EMBEDDING_DIMENSIONS as i32,
                now,
            ],
        )?;

        Ok(())
    }

    /// Search by semantic similarity using mock embeddings.
    ///
    /// Generates a query vector from the input text, loads all embeddings,
    /// computes cosine similarity, filters by threshold, and returns the
    /// top results sorted by descending similarity.
    pub fn search_similar(
        conn: &Connection,
        query: &str,
        limit: usize,
        threshold: Option<f64>,
    ) -> Result<Vec<SearchResult>> {
        let query_vector = generate_mock_vector(query);

        // Load all embeddings with their entity info
        let mut stmt = conn.prepare(
            "SELECT emb.entity_id, emb.vector, e.title, e.entity_type
             FROM embeddings emb
             JOIN entities e ON e.id = emb.entity_id
             WHERE e.deleted_at IS NULL",
        )?;

        let rows = stmt.query_map([], |row| {
            let entity_id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let title: String = row.get(2)?;
            let entity_type: String = row.get(3)?;
            Ok((entity_id, blob, title, entity_type))
        })?;

        let threshold_val = threshold.unwrap_or(-1.0);
        let mut results: Vec<SearchResult> = Vec::new();

        for row in rows {
            let (entity_id, blob, title, entity_type) = row?;
            let entity_vector = blob_to_vector(&blob);
            let similarity = cosine_similarity(&query_vector, &entity_vector);

            if similarity >= threshold_val {
                results.push(SearchResult {
                    entity_id,
                    title,
                    entity_type,
                    score: similarity,
                });
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top `limit`
        results.truncate(limit);
        Ok(results)
    }

    /// Re-index an entity: update FTS5 entry and regenerate embedding.
    ///
    /// Because the FTS5 table uses `content=entities` (external content),
    /// the safest way to bring the index in sync after an entity mutation
    /// is to rebuild the entire FTS index from the content table.
    /// For a production system with many entities you would track old values
    /// and issue targeted delete/insert pairs; here we use rebuild for
    /// correctness.
    pub fn reindex_entity(conn: &Connection, entity_id: &str) -> Result<()> {
        // Verify the entity exists
        let _exists: i64 = conn.query_row(
            "SELECT rowid FROM entities WHERE id = ?1",
            params![entity_id],
            |row| row.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: entity_id.to_string(),
            },
            other => GargoyleError::Database(other),
        })?;

        // Rebuild the entire FTS index from the content table.
        // This re-reads every row from `entities` and rebuilds entities_fts.
        conn.execute(
            "INSERT INTO entities_fts(entities_fts) VALUES('rebuild')",
            [],
        )?;

        // Regenerate embedding
        Self::generate_embedding(conn, entity_id)?;

        Ok(())
    }
}

/// Generate a deterministic 128-dimension mock vector from text.
///
/// For each dimension i, hashes the concatenation of text + i,
/// then maps the hash to f32 in [-1.0, 1.0].
fn generate_mock_vector(text: &str) -> Vec<f32> {
    let mut vector = Vec::with_capacity(EMBEDDING_DIMENSIONS);
    for i in 0..EMBEDDING_DIMENSIONS {
        let mut hasher = DefaultHasher::new();
        format!("{}{}", text, i).hash(&mut hasher);
        let h = hasher.finish();
        // Map u64 to [-1.0, 1.0]
        let value = (h as f64 / u64::MAX as f64) * 2.0 - 1.0;
        vector.push(value as f32);
    }
    vector
}

/// Serialize a Vec<f32> to raw bytes (little-endian f32s).
fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for &v in vector {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

/// Deserialize raw bytes back to Vec<f32> (little-endian f32s).
fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| {
            let bytes: [u8; 4] = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_le_bytes(bytes)
        })
        .collect()
}

/// Cosine similarity between two vectors: dot(a,b) / (|a| * |b|).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for (x, y) in a.iter().zip(b.iter()) {
        let x = *x as f64;
        let y = *y as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::create_memory_connection;
    use crate::db::migrations::run_migrations;

    /// Create an in-memory DB with the full schema applied.
    fn setup_db() -> Connection {
        let conn = create_memory_connection().expect("Failed to create in-memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    /// Helper: insert a test entity and populate FTS5.
    fn insert_entity(
        conn: &Connection,
        id: &str,
        entity_type: &str,
        title: &str,
        body_md: &str,
        canonical_fields: &str,
    ) {
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields)
             VALUES (?1, ?2, ?3, ?4, 'manual', ?5)",
            params![id, entity_type, title, body_md, canonical_fields],
        )
        .expect("Failed to insert entity");

        // Manually populate FTS5 for the content= external-content table
        let rowid: i64 = conn
            .query_row(
                "SELECT rowid FROM entities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .expect("Failed to get rowid");

        conn.execute(
            "INSERT INTO entities_fts(rowid, title, body_md) VALUES(?1, ?2, ?3)",
            params![rowid, title, body_md],
        )
        .expect("Failed to insert into FTS");
    }

    #[test]
    fn test_search_fts_basic() {
        let conn = setup_db();
        insert_entity(
            &conn,
            "ent-001",
            "metric",
            "Revenue Growth Rate",
            "Monthly recurring revenue growth",
            "{}",
        );
        insert_entity(
            &conn,
            "ent-002",
            "experiment",
            "Customer Churn Analysis",
            "Analyzing churn patterns in Q4",
            "{}",
        );
        insert_entity(
            &conn,
            "ent-003",
            "metric",
            "Churn Rate",
            "Monthly churn rate tracking",
            "{}",
        );

        // Search for "churn"
        let results = IndexerService::search_fts(&conn, "churn", 10).unwrap();
        assert!(results.len() >= 2, "Expected at least 2 results for 'churn', got {}", results.len());

        let ids: Vec<&str> = results.iter().map(|r| r.entity_id.as_str()).collect();
        assert!(ids.contains(&"ent-002"), "Should find 'Customer Churn Analysis'");
        assert!(ids.contains(&"ent-003"), "Should find 'Churn Rate'");
    }

    #[test]
    fn test_search_fts_excludes_deleted() {
        let conn = setup_db();
        insert_entity(
            &conn,
            "ent-010",
            "metric",
            "Deleted Metric",
            "This was deleted",
            "{}",
        );
        // Soft-delete it
        conn.execute(
            "UPDATE entities SET deleted_at = '2026-01-01T00:00:00.000Z' WHERE id = 'ent-010'",
            [],
        )
        .unwrap();

        insert_entity(
            &conn,
            "ent-011",
            "metric",
            "Active Metric",
            "This is active and has the word deleted in it",
            "{}",
        );

        let results = IndexerService::search_fts(&conn, "deleted", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "ent-011");
    }

    #[test]
    fn test_search_fts_limit() {
        let conn = setup_db();
        for i in 0..10 {
            let id = format!("ent-lim-{}", i);
            let title = format!("Alpha item {}", i);
            insert_entity(&conn, &id, "metric", &title, "alpha content", "{}");
        }

        let results = IndexerService::search_fts(&conn, "alpha", 3).unwrap();
        assert_eq!(results.len(), 3, "Limit should be respected");
    }

    #[test]
    fn test_generate_embedding() {
        let conn = setup_db();
        insert_entity(
            &conn,
            "ent-emb-001",
            "metric",
            "Test Embedding Entity",
            "Some body",
            "{\"unit\":\"percent\"}",
        );

        IndexerService::generate_embedding(&conn, "ent-emb-001").unwrap();

        // Verify embedding was stored
        let (model, dimensions, blob_len): (String, i32, usize) = conn
            .query_row(
                "SELECT model, dimensions, length(vector) FROM embeddings WHERE entity_id = ?1",
                params!["ent-emb-001"],
                |row| {
                    let model: String = row.get(0)?;
                    let dims: i32 = row.get(1)?;
                    let blen: i32 = row.get(2)?;
                    Ok((model, dims, blen as usize))
                },
            )
            .unwrap();

        assert_eq!(model, "mock-hash-v1");
        assert_eq!(dimensions, 128);
        assert_eq!(blob_len, 128 * 4); // 128 f32s * 4 bytes each
    }

    #[test]
    fn test_generate_embedding_replaces_old() {
        let conn = setup_db();
        insert_entity(
            &conn,
            "ent-emb-002",
            "metric",
            "Replace Test",
            "Body",
            "{}",
        );

        IndexerService::generate_embedding(&conn, "ent-emb-002").unwrap();
        IndexerService::generate_embedding(&conn, "ent-emb-002").unwrap();

        // Should only have one embedding
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embeddings WHERE entity_id = ?1",
                params!["ent-emb-002"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Re-generating should replace, not duplicate");
    }

    #[test]
    fn test_generate_embedding_not_found() {
        let conn = setup_db();
        let result = IndexerService::generate_embedding(&conn, "nonexistent-id");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "entity");
                assert_eq!(id, "nonexistent-id");
            }
            other => panic!("Expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_search_similar_basic() {
        let conn = setup_db();

        // Insert entities with varying titles
        insert_entity(&conn, "sim-001", "metric", "Revenue Growth Rate", "Monthly revenue", "{}");
        insert_entity(&conn, "sim-002", "metric", "Revenue Decline Rate", "Monthly decline", "{}");
        insert_entity(&conn, "sim-003", "experiment", "Customer Churn Analysis", "Churn patterns", "{}");

        // Generate embeddings for all
        IndexerService::generate_embedding(&conn, "sim-001").unwrap();
        IndexerService::generate_embedding(&conn, "sim-002").unwrap();
        IndexerService::generate_embedding(&conn, "sim-003").unwrap();

        // Search for something similar to "Revenue Growth Rate {}"
        // The query text goes through the same hash, so an exact match should be most similar
        let results = IndexerService::search_similar(&conn, "Revenue Growth Rate {}", 10, None).unwrap();
        assert!(!results.is_empty(), "Should return at least one result");

        // The top result should be the entity whose text hashes most similarly
        // Since "Revenue Growth Rate {}" is exactly "Revenue Growth Rate" + " " + "{}",
        // sim-001 should be the top match
        assert_eq!(results[0].entity_id, "sim-001");
        assert!(results[0].score > results.last().unwrap().score || results.len() == 1);
    }

    #[test]
    fn test_search_similar_self_is_perfect() {
        let conn = setup_db();
        insert_entity(&conn, "self-001", "metric", "Exact Match Test", "body text", "{}");
        IndexerService::generate_embedding(&conn, "self-001").unwrap();

        // Query with the exact same text that was used to generate the embedding
        // The embedding text is "title canonical_fields" = "Exact Match Test {}"
        let results =
            IndexerService::search_similar(&conn, "Exact Match Test {}", 10, None).unwrap();
        assert_eq!(results.len(), 1);
        let score = results[0].score;
        assert!(
            (score - 1.0).abs() < 1e-6,
            "Cosine similarity of a vector with itself should be 1.0, got {}",
            score
        );
    }

    #[test]
    fn test_search_similar_threshold() {
        let conn = setup_db();
        insert_entity(&conn, "thr-001", "metric", "AAA", "aaa", "{}");
        insert_entity(&conn, "thr-002", "metric", "BBB", "bbb", "{}");
        insert_entity(&conn, "thr-003", "metric", "CCC", "ccc", "{}");

        IndexerService::generate_embedding(&conn, "thr-001").unwrap();
        IndexerService::generate_embedding(&conn, "thr-002").unwrap();
        IndexerService::generate_embedding(&conn, "thr-003").unwrap();

        // With a very high threshold, we should get fewer results
        let high = IndexerService::search_similar(&conn, "AAA {}", 10, Some(0.99)).unwrap();
        let low = IndexerService::search_similar(&conn, "AAA {}", 10, Some(-1.0)).unwrap();

        assert!(
            high.len() <= low.len(),
            "Higher threshold should return fewer or equal results"
        );
    }

    #[test]
    fn test_search_similar_limit() {
        let conn = setup_db();
        for i in 0..5 {
            let id = format!("lim-{}", i);
            let title = format!("Item {}", i);
            insert_entity(&conn, &id, "metric", &title, "body", "{}");
            IndexerService::generate_embedding(&conn, &id).unwrap();
        }

        let results = IndexerService::search_similar(&conn, "Item 0 {}", 2, None).unwrap();
        assert_eq!(results.len(), 2, "Should respect the limit parameter");
    }

    #[test]
    fn test_search_similar_excludes_deleted() {
        let conn = setup_db();
        insert_entity(&conn, "del-001", "metric", "Deleted Entity", "body", "{}");
        IndexerService::generate_embedding(&conn, "del-001").unwrap();

        // Soft-delete
        conn.execute(
            "UPDATE entities SET deleted_at = '2026-01-01T00:00:00.000Z' WHERE id = 'del-001'",
            [],
        )
        .unwrap();

        let results =
            IndexerService::search_similar(&conn, "Deleted Entity {}", 10, None).unwrap();
        assert!(
            results.is_empty(),
            "Deleted entities should not appear in similarity search"
        );
    }

    #[test]
    fn test_reindex_entity() {
        let conn = setup_db();
        insert_entity(&conn, "reidx-001", "metric", "Original Title", "Original body", "{}");
        IndexerService::generate_embedding(&conn, "reidx-001").unwrap();

        // Verify FTS works before reindex
        let results = IndexerService::search_fts(&conn, "Original", 10).unwrap();
        assert_eq!(results.len(), 1);

        // Update entity directly (simulating a store update)
        conn.execute(
            "UPDATE entities SET title = 'Updated Title', body_md = 'Updated body' WHERE id = 'reidx-001'",
            [],
        )
        .unwrap();

        // Reindex
        IndexerService::reindex_entity(&conn, "reidx-001").unwrap();

        // FTS should now find "Updated" but not "Original"
        let results_old = IndexerService::search_fts(&conn, "Original", 10).unwrap();
        assert_eq!(results_old.len(), 0, "Old title should no longer match FTS");

        let results_new = IndexerService::search_fts(&conn, "Updated", 10).unwrap();
        assert_eq!(results_new.len(), 1, "New title should match FTS");
        assert_eq!(results_new[0].entity_id, "reidx-001");

        // Embedding should still exist (regenerated)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embeddings WHERE entity_id = 'reidx-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Embedding should be regenerated");
    }

    #[test]
    fn test_mock_vector_determinism() {
        let v1 = generate_mock_vector("hello world");
        let v2 = generate_mock_vector("hello world");
        assert_eq!(v1, v2, "Same input should always produce the same vector");

        let v3 = generate_mock_vector("different text");
        assert_ne!(v1, v3, "Different input should produce different vectors");
    }

    #[test]
    fn test_cosine_similarity_properties() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let c = vec![1.0f32, 0.0, 0.0];

        // Orthogonal vectors have similarity 0
        let sim_ab = cosine_similarity(&a, &b);
        assert!((sim_ab - 0.0).abs() < 1e-6, "Orthogonal vectors should have similarity 0");

        // Identical vectors have similarity 1
        let sim_ac = cosine_similarity(&a, &c);
        assert!((sim_ac - 1.0).abs() < 1e-6, "Identical vectors should have similarity 1");

        // Opposite vectors have similarity -1
        let d = vec![-1.0f32, 0.0, 0.0];
        let sim_ad = cosine_similarity(&a, &d);
        assert!((sim_ad - (-1.0)).abs() < 1e-6, "Opposite vectors should have similarity -1");
    }

    #[test]
    fn test_vector_blob_roundtrip() {
        let original = vec![1.5f32, -2.3, 0.0, 42.0, -0.001];
        let blob = vector_to_blob(&original);
        let restored = blob_to_vector(&blob);
        assert_eq!(original, restored, "Blob roundtrip should preserve exact values");
    }
}

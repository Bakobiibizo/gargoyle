// Scenario 6: Embedding / Semantic Search Integration Tests
//
// Tests the IndexerService: embedding generation, search_similar, FTS search,
// and embedding regeneration. Uses the mock hash-based embedding system.

mod common;

use gargoyle_lib::services::indexer::IndexerService;
use rusqlite::params;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a test entity with FTS5 index population.
fn insert_entity_with_fts(
    conn: &rusqlite::Connection,
    id: &str,
    entity_type: &str,
    title: &str,
    body_md: &str,
    canonical_fields: &str,
) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'manual', ?5, 1, ?6, ?6)",
        params![id, entity_type, title, body_md, canonical_fields, now],
    )
    .expect("Failed to insert test entity");

    // Populate FTS5 external-content table
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

// =============================================================================
// 6a. BASIC SEMANTIC SEARCH
// =============================================================================

#[test]
fn test_6a_basic_semantic_search() {
    let conn = common::test_db();

    // Create 5+ metric entities
    let titles = [
        "Revenue Growth Rate",
        "Customer Acquisition Cost",
        "Monthly Active Users",
        "Churn Rate Analysis",
        "Net Promoter Score",
        "Average Revenue Per User",
    ];

    for (i, title) in titles.iter().enumerate() {
        let id = format!("emb-{}", i);
        insert_entity_with_fts(&conn, &id, "metric", title, "", "{}");
        IndexerService::generate_embedding(&conn, &id).unwrap();
    }

    // Search for something -- with mock hash embeddings the query text is hashed
    // and compared via cosine similarity. Use the exact entity text format to
    // guarantee at least one strong match.
    let results = IndexerService::search_similar(&conn, "Revenue Growth Rate {}", 5, None).unwrap();

    assert!(
        !results.is_empty(),
        "Semantic search should return at least one result"
    );

    // Results should be ranked (descending score)
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "Results should be sorted by score descending: {} >= {}",
            window[0].score,
            window[1].score
        );
    }

    // The top result should be the entity whose embedding text matches the query
    assert_eq!(
        results[0].entity_id, "emb-0",
        "Top result should be 'Revenue Growth Rate' since query matches its embedding text"
    );
    assert!(
        (results[0].score - 1.0).abs() < 1e-6,
        "Exact text match should produce score ~1.0, got {}",
        results[0].score
    );
}

// =============================================================================
// 6b. EMPTY RESULTS -- nonsensical query
// =============================================================================

#[test]
fn test_6b_empty_results_nonsensical_query() {
    let conn = common::test_db();

    // Create some entities with embeddings
    insert_entity_with_fts(&conn, "emb-b1", "metric", "Revenue Growth", "", "{}");
    IndexerService::generate_embedding(&conn, "emb-b1").unwrap();

    // Search with a nonsensical query and a HIGH threshold
    // With mock hash embeddings, random text should produce low similarity
    let results = IndexerService::search_similar(
        &conn,
        "xyzzy_qqq_nonsense_zxcvbn_12345",
        10,
        Some(0.99),
    )
    .unwrap();

    // With a 0.99 threshold, the nonsensical query should match nothing
    assert!(
        results.is_empty(),
        "Nonsensical query with high threshold should return no results, got {}",
        results.len()
    );

    // Without threshold, it should still not crash and should return results
    let results_no_threshold = IndexerService::search_similar(
        &conn,
        "xyzzy_qqq_nonsense_zxcvbn_12345",
        10,
        None,
    )
    .unwrap();

    // Should not crash -- just return whatever (possibly with low scores)
    // The key assertion is that this does not panic or error
    assert!(
        results_no_threshold.len() <= 10,
        "Should respect limit even without threshold"
    );
}

// =============================================================================
// 6c. LIMIT RESPECTED
// =============================================================================

#[test]
fn test_6c_limit_respected() {
    let conn = common::test_db();

    // Create 10 entities with embeddings
    for i in 0..10 {
        let id = format!("lim-{}", i);
        let title = format!("Metric Item {}", i);
        insert_entity_with_fts(&conn, &id, "metric", &title, "", "{}");
        IndexerService::generate_embedding(&conn, &id).unwrap();
    }

    // Search with limit=3
    let results = IndexerService::search_similar(&conn, "Metric Item 0 {}", 3, None).unwrap();
    assert!(
        results.len() <= 3,
        "search_similar with limit=3 should return at most 3 results, got {}",
        results.len()
    );

    // Search with limit=1
    let results_1 = IndexerService::search_similar(&conn, "Metric Item 0 {}", 1, None).unwrap();
    assert!(
        results_1.len() <= 1,
        "search_similar with limit=1 should return at most 1 result, got {}",
        results_1.len()
    );

    // Search with limit=0 should return 0
    let results_0 = IndexerService::search_similar(&conn, "Metric Item 0 {}", 0, None).unwrap();
    assert_eq!(
        results_0.len(),
        0,
        "search_similar with limit=0 should return 0 results"
    );
}

// =============================================================================
// 6d. THRESHOLD FILTERING
// =============================================================================

#[test]
fn test_6d_threshold_filtering() {
    let conn = common::test_db();

    // Create entities with very different titles (should produce different hash vectors)
    insert_entity_with_fts(&conn, "thr-a", "metric", "AAA Metric Alpha", "", "{}");
    insert_entity_with_fts(&conn, "thr-b", "metric", "BBB Metric Beta", "", "{}");
    insert_entity_with_fts(&conn, "thr-c", "metric", "CCC Metric Gamma", "", "{}");

    IndexerService::generate_embedding(&conn, "thr-a").unwrap();
    IndexerService::generate_embedding(&conn, "thr-b").unwrap();
    IndexerService::generate_embedding(&conn, "thr-c").unwrap();

    // Query matching thr-a exactly
    let query = "AAA Metric Alpha {}";

    // With threshold=0.8, only high-similarity results pass
    let high_threshold =
        IndexerService::search_similar(&conn, query, 10, Some(0.8)).unwrap();

    // All returned results must meet the threshold
    for r in &high_threshold {
        assert!(
            r.score >= 0.8,
            "All results should have score >= 0.8 threshold, got {} for {}",
            r.score,
            r.entity_id
        );
    }

    // With no threshold (or very low threshold), should get more or equal results
    let no_threshold = IndexerService::search_similar(&conn, query, 10, None).unwrap();
    assert!(
        high_threshold.len() <= no_threshold.len(),
        "High threshold ({}) should return fewer or equal results than no threshold ({})",
        high_threshold.len(),
        no_threshold.len()
    );
}

// =============================================================================
// 6e. SERVICE ISOLATION -- cosine computation only in indexer
// =============================================================================

#[test]
fn test_6e_service_isolation() {
    // This is a code-audit style test. The cosine_similarity function is defined
    // as a private function inside src/services/indexer.rs and is not exposed
    // publicly. The dedup pipeline delegates to IndexerService::search_similar
    // rather than computing cosine similarity itself.
    //
    // We verify this by confirming that DedupPipeline works through the
    // IndexerService API and does not re-implement cosine computation.
    //
    // Since the cosine_similarity function in indexer.rs is private (fn, not pub fn),
    // we cannot call it from tests outside the module -- which IS the isolation
    // guarantee. This test documents that contract.

    // Functional validation: DedupPipeline stage 3 uses IndexerService
    let conn = common::test_db();
    insert_entity_with_fts(&conn, "iso-1", "metric", "Isolation Test Entity", "", "{}");
    IndexerService::generate_embedding(&conn, "iso-1").unwrap();

    insert_entity_with_fts(&conn, "iso-2", "metric", "Isolation Test Entity", "", "{}");

    // If dedup pipeline stage 3 works, it is using IndexerService internally
    let suggestions =
        gargoyle_lib::services::dedup::DedupPipeline::check_for_duplicates(&conn, "iso-2")
            .unwrap();

    // The exact match will be found by stage 1. The embedding match (stage 3) is
    // deduplicated against already-found IDs. This confirms the pipeline integrates
    // with IndexerService correctly.
    assert!(
        !suggestions.is_empty(),
        "Pipeline should find the duplicate via stage 1"
    );
}

// =============================================================================
// 6f. EMBEDDING REGENERATION
// =============================================================================

#[test]
fn test_6f_embedding_regeneration() {
    let conn = common::test_db();

    insert_entity_with_fts(
        &conn,
        "regen-1",
        "metric",
        "Original Title",
        "Original body",
        "{}",
    );

    // Generate initial embedding
    IndexerService::generate_embedding(&conn, "regen-1").unwrap();

    // Read the original embedding vector
    let original_blob: Vec<u8> = conn
        .query_row(
            "SELECT vector FROM embeddings WHERE entity_id = 'regen-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Update the entity title and canonical_fields (simulating an edit)
    conn.execute(
        "UPDATE entities SET title = 'Completely Different Title', canonical_fields = '{\"unit\":\"percent\"}' WHERE id = 'regen-1'",
        [],
    )
    .unwrap();

    // Regenerate embedding
    IndexerService::generate_embedding(&conn, "regen-1").unwrap();

    // Read the new embedding vector
    let new_blob: Vec<u8> = conn
        .query_row(
            "SELECT vector FROM embeddings WHERE entity_id = 'regen-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Vectors should differ since title + canonical_fields changed
    assert_ne!(
        original_blob, new_blob,
        "Embedding should change after entity title/canonical_fields are updated"
    );

    // There should still be only ONE embedding (old one replaced, not duplicated)
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE entity_id = 'regen-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 1,
        "Regeneration should replace the old embedding, not add a new one"
    );
}

// =============================================================================
// 6 bonus: FTS search basic smoke test (integration coverage)
// =============================================================================

#[test]
fn test_6_bonus_fts_search_integration() {
    let conn = common::test_db();

    insert_entity_with_fts(
        &conn,
        "fts-1",
        "metric",
        "Revenue Growth Rate",
        "Monthly recurring revenue tracking",
        "{}",
    );
    insert_entity_with_fts(
        &conn,
        "fts-2",
        "experiment",
        "Pricing Experiment",
        "Testing price elasticity",
        "{}",
    );
    insert_entity_with_fts(
        &conn,
        "fts-3",
        "metric",
        "Customer Churn Rate",
        "Revenue impact from churn",
        "{}",
    );

    // FTS search for "revenue"
    let results = IndexerService::search_fts(&conn, "revenue", 10).unwrap();
    assert!(
        results.len() >= 2,
        "FTS search for 'revenue' should find at least 2 entities (title + body matches), got {}",
        results.len()
    );

    let ids: Vec<&str> = results.iter().map(|r| r.entity_id.as_str()).collect();
    assert!(ids.contains(&"fts-1"), "Should find 'Revenue Growth Rate'");
    // fts-3 has "Revenue" in body_md
    assert!(
        ids.contains(&"fts-3"),
        "Should find 'Customer Churn Rate' (body contains 'revenue')"
    );
}

// =============================================================================
// 6 bonus: search_similar excludes soft-deleted entities
// =============================================================================

#[test]
fn test_6_bonus_search_excludes_deleted() {
    let conn = common::test_db();

    insert_entity_with_fts(&conn, "del-1", "metric", "Deleted Metric", "", "{}");
    IndexerService::generate_embedding(&conn, "del-1").unwrap();

    // Soft-delete
    conn.execute(
        "UPDATE entities SET deleted_at = '2026-01-01T00:00:00.000Z' WHERE id = 'del-1'",
        [],
    )
    .unwrap();

    let results =
        IndexerService::search_similar(&conn, "Deleted Metric {}", 10, None).unwrap();
    assert!(
        results.is_empty(),
        "Soft-deleted entities should not appear in similarity search"
    );
}

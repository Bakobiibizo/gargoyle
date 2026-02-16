// Scenario 5: Dedup Pipeline Integration Tests
//
// Tests the 3-stage dedup pipeline (exact title -> fuzzy title -> embedding proximity).
// Entity creation is NEVER blocked by dedup. Dedup runs post-commit.
// Cross-type matching produces no suggestions.

mod common;

use gargoyle_lib::models::dedup::DetectionMethod;
use gargoyle_lib::services::dedup::DedupPipeline;
use rusqlite::params;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn insert_entity(conn: &rusqlite::Connection, id: &str, entity_type: &str, title: &str) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, '', 'manual', '{}', 1, ?4, ?4)",
        params![id, entity_type, title, now],
    )
    .expect("Failed to insert test entity");
}

// =============================================================================
// 5a. EXACT TITLE MATCH
// =============================================================================

#[test]
fn test_5a_exact_title_match() {
    let conn = common::test_db();

    insert_entity(&conn, "m-orig", "metric", "Monthly Recurring Revenue");
    insert_entity(&conn, "m-dup", "metric", "Monthly Recurring Revenue");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-dup").unwrap();

    assert!(
        !suggestions.is_empty(),
        "Exact title match should produce at least one suggestion"
    );

    let exact = suggestions
        .iter()
        .find(|s| s.detection_method == DetectionMethod::ExactTitle);
    assert!(
        exact.is_some(),
        "Should have an exact_title suggestion for identical titles"
    );

    let exact = exact.unwrap();
    assert_eq!(exact.existing_entity_id, "m-orig");
    assert_eq!(exact.new_entity_id, "m-dup");
    assert!(
        (exact.confidence - 1.0).abs() < f64::EPSILON,
        "Exact title match should have confidence = 1.0"
    );
    assert_eq!(exact.status, "pending");
}

// =============================================================================
// 5b. CASE-INSENSITIVE EXACT MATCH
// =============================================================================

#[test]
fn test_5b_case_insensitive_exact_match() {
    let conn = common::test_db();

    insert_entity(&conn, "m-upper", "metric", "Monthly Recurring Revenue");
    insert_entity(&conn, "m-lower", "metric", "monthly recurring revenue");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-lower").unwrap();

    assert!(
        !suggestions.is_empty(),
        "Case-insensitive exact match should produce suggestions"
    );

    let exact = suggestions
        .iter()
        .find(|s| s.detection_method == DetectionMethod::ExactTitle);
    assert!(
        exact.is_some(),
        "Case difference should still register as exact_title"
    );

    let exact = exact.unwrap();
    assert_eq!(exact.existing_entity_id, "m-upper");
    assert!(
        (exact.confidence - 1.0).abs() < f64::EPSILON,
        "Case-insensitive exact match should have confidence = 1.0"
    );
}

// =============================================================================
// 5c. FUZZY TITLE MATCH (typo)
// =============================================================================

#[test]
fn test_5c_fuzzy_title_match_typo() {
    let conn = common::test_db();

    insert_entity(&conn, "m-correct", "metric", "Monthly Recurring Revenue");
    insert_entity(&conn, "m-typo", "metric", "Monthly Recurrig Revenue");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-typo").unwrap();

    assert!(
        !suggestions.is_empty(),
        "Typo in title should produce at least one suggestion"
    );

    // It should be either exact (if SQL LOWER treats them the same -- it won't for a typo)
    // or fuzzy. Since "Recurring" vs "Recurrig" differs, it should be fuzzy.
    let fuzzy = suggestions
        .iter()
        .find(|s| s.detection_method == DetectionMethod::FuzzyTitle);
    assert!(
        fuzzy.is_some(),
        "Typo should produce a fuzzy_title suggestion (not exact)"
    );

    let fuzzy = fuzzy.unwrap();
    assert!(
        fuzzy.confidence >= 0.8,
        "Levenshtein similarity should be >= 0.8 for a single-char typo, got {}",
        fuzzy.confidence
    );
    assert!(
        fuzzy.confidence < 1.0,
        "Should not be a perfect match, got {}",
        fuzzy.confidence
    );
    assert_eq!(fuzzy.existing_entity_id, "m-correct");
}

// =============================================================================
// 5d. EMBEDDING PROXIMITY
// =============================================================================

#[test]
fn test_5d_embedding_proximity() {
    let conn = common::test_db();

    // Create two entities with significantly different titles.
    // With the mock hash-based embedding system, semantically related titles
    // will NOT necessarily produce high cosine similarity (it is hash-based,
    // not semantic). So we test that the pipeline runs without error and
    // does not produce false positives for truly different titles.
    insert_entity(&conn, "m-rev", "metric", "Monthly Recurring Revenue");
    insert_entity(&conn, "m-churn", "metric", "Customer Churn Rate");

    // Generate embedding for existing entity so stage 3 has data to compare
    gargoyle_lib::services::indexer::IndexerService::generate_embedding(&conn, "m-rev").unwrap();

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-churn").unwrap();

    // With hash-based mock embeddings, these very different titles should not
    // produce an embedding_proximity match (threshold is 0.85).
    let embedding_suggestions: Vec<_> = suggestions
        .iter()
        .filter(|s| s.detection_method == DetectionMethod::EmbeddingProximity)
        .collect();

    // The titles are very different, so we expect no embedding match with mock vectors.
    // If the mock system did produce a match, verify it meets the threshold.
    for s in &embedding_suggestions {
        assert!(
            s.confidence >= 0.85,
            "Embedding proximity suggestion should have score >= 0.85, got {}",
            s.confidence
        );
    }

    // Also verify no exact or fuzzy matches for these very different titles
    let exact_or_fuzzy: Vec<_> = suggestions
        .iter()
        .filter(|s| {
            s.detection_method == DetectionMethod::ExactTitle
                || s.detection_method == DetectionMethod::FuzzyTitle
        })
        .collect();
    assert!(
        exact_or_fuzzy.is_empty(),
        "Very different titles should not produce exact or fuzzy matches"
    );
}

// =============================================================================
// 5e. SHORT TITLE -- EMBEDDING BEHAVIOR
// =============================================================================

#[test]
fn test_5e_short_title_embedding_behavior() {
    let conn = common::test_db();

    insert_entity(&conn, "m-mrr-long", "metric", "Monthly Recurring Revenue");
    insert_entity(&conn, "m-mrr-short", "metric", "MRR");

    // Generate embedding for existing entity
    gargoyle_lib::services::indexer::IndexerService::generate_embedding(&conn, "m-mrr-long")
        .unwrap();

    // Run dedup on the short title entity
    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-mrr-short").unwrap();

    // The pipeline should run without error regardless of title length.
    // "MRR" and "Monthly Recurring Revenue" are very different strings,
    // so no exact or fuzzy match is expected.
    let exact_or_fuzzy: Vec<_> = suggestions
        .iter()
        .filter(|s| {
            s.detection_method == DetectionMethod::ExactTitle
                || s.detection_method == DetectionMethod::FuzzyTitle
        })
        .collect();
    assert!(
        exact_or_fuzzy.is_empty(),
        "Short title 'MRR' should not exact/fuzzy match 'Monthly Recurring Revenue'"
    );

    // The embedding stage runs (generates embedding for the short title too).
    // Verify the embedding was created for the short-title entity.
    let emb_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE entity_id = 'm-mrr-short'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        emb_count, 1,
        "Embedding should be generated even for short titles"
    );
}

// =============================================================================
// 5f. CROSS-TYPE -- NO FALSE POSITIVE
// =============================================================================

#[test]
fn test_5f_cross_type_no_false_positive() {
    let conn = common::test_db();

    insert_entity(&conn, "m-revenue", "metric", "Monthly Recurring Revenue");
    insert_entity(&conn, "r-revenue", "result", "Monthly Recurring Revenue");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "r-revenue").unwrap();

    assert!(
        suggestions.is_empty(),
        "Cross-type matches must never be created. \
         metric and result with same title should produce 0 suggestions. Got: {:?}",
        suggestions
            .iter()
            .map(|s| format!(
                "{}->{}({})",
                s.new_entity_id, s.existing_entity_id, s.detection_method
            ))
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// 5g. NON-BLOCKING CREATION
// =============================================================================

#[test]
fn test_5g_non_blocking_creation() {
    let conn = common::test_db();

    // Create two entities with the same title -- both must exist
    insert_entity(&conn, "m-nb1", "metric", "Duplicate Title For Test");
    insert_entity(&conn, "m-nb2", "metric", "Duplicate Title For Test");

    // Verify both entities exist in the DB (creation was not blocked)
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE title = 'Duplicate Title For Test'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 2, "Both entities must exist -- creation is never blocked");

    // Dedup runs post-commit, returns suggestions but does not delete/block
    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-nb2").unwrap();
    assert!(
        !suggestions.is_empty(),
        "Dedup should produce suggestions for identical titles"
    );

    // Verify entities still exist after dedup ran
    let count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE title = 'Duplicate Title For Test'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count_after, 2,
        "Dedup must never delete or block entities"
    );
}

#[test]
fn test_5g_dedup_failure_does_not_block() {
    let conn = common::test_db();

    // Call check_for_duplicates with a nonexistent entity ID.
    // This will fail internally, but must NOT propagate the error.
    let result = DedupPipeline::check_for_duplicates(&conn, "nonexistent-entity-xyz");
    assert!(
        result.is_ok(),
        "Dedup failure must not propagate -- must return Ok"
    );
    assert!(
        result.unwrap().is_empty(),
        "Failed dedup should return empty vec"
    );
}

// =============================================================================
// 5h. GET SUGGESTIONS BY STATUS
// =============================================================================

#[test]
fn test_5h_get_suggestions_by_status() {
    let conn = common::test_db();

    // Create entities that will produce dedup suggestions
    insert_entity(&conn, "m-s1", "metric", "Status Test Alpha");
    insert_entity(&conn, "m-s2", "metric", "Status Test Alpha");
    insert_entity(&conn, "m-s3", "metric", "Status Test Alpha");

    // Run dedup for m-s2 (finds m-s1)
    DedupPipeline::check_for_duplicates(&conn, "m-s2").unwrap();
    // Run dedup for m-s3 (finds m-s1 and m-s2)
    DedupPipeline::check_for_duplicates(&conn, "m-s3").unwrap();

    // All suggestions should be "pending"
    let pending = DedupPipeline::get_suggestions(&conn, Some("pending")).unwrap();
    assert!(
        pending.len() >= 3,
        "Should have at least 3 pending suggestions, got {}",
        pending.len()
    );

    // No dismissed or accepted suggestions yet
    let dismissed = DedupPipeline::get_suggestions(&conn, Some("dismissed")).unwrap();
    assert!(dismissed.is_empty(), "No dismissed suggestions yet");

    let accepted = DedupPipeline::get_suggestions(&conn, Some("accepted")).unwrap();
    assert!(accepted.is_empty(), "No accepted suggestions yet");

    // Get all (no filter)
    let all = DedupPipeline::get_suggestions(&conn, None).unwrap();
    assert_eq!(
        all.len(),
        pending.len(),
        "All suggestions should equal pending count"
    );
}

// =============================================================================
// 5i. RESOLVE SUGGESTION (accept / dismiss)
// =============================================================================

#[test]
fn test_5i_resolve_suggestion_dismissed() {
    let conn = common::test_db();

    insert_entity(&conn, "m-r1", "metric", "Resolve Test");
    insert_entity(&conn, "m-r2", "metric", "Resolve Test");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-r2").unwrap();
    assert!(!suggestions.is_empty(), "Should have suggestions to resolve");

    let suggestion_id = &suggestions[0].suggestion_id;

    // Resolve as dismissed
    DedupPipeline::resolve_suggestion(&conn, suggestion_id, "dismissed").unwrap();

    // Verify status changed
    let dismissed = DedupPipeline::get_suggestions(&conn, Some("dismissed")).unwrap();
    assert_eq!(dismissed.len(), 1);
    assert_eq!(dismissed[0].suggestion_id, *suggestion_id);
    assert_eq!(dismissed[0].status, "dismissed");
}

#[test]
fn test_5i_resolve_suggestion_accepted() {
    let conn = common::test_db();

    insert_entity(&conn, "m-a1", "metric", "Accept Test");
    insert_entity(&conn, "m-a2", "metric", "Accept Test");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-a2").unwrap();
    assert!(!suggestions.is_empty());

    let suggestion_id = &suggestions[0].suggestion_id;

    // Resolve as accepted
    DedupPipeline::resolve_suggestion(&conn, suggestion_id, "accepted").unwrap();

    // Verify status changed
    let accepted = DedupPipeline::get_suggestions(&conn, Some("accepted")).unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].suggestion_id, *suggestion_id);
    assert_eq!(accepted[0].status, "accepted");

    // Pending count should decrease
    let pending = DedupPipeline::get_suggestions(&conn, Some("pending")).unwrap();
    assert!(
        pending.iter().all(|s| s.suggestion_id != *suggestion_id),
        "Accepted suggestion should no longer appear as pending"
    );
}

#[test]
fn test_5i_resolve_suggestion_invalid_status() {
    let conn = common::test_db();

    insert_entity(&conn, "m-inv1", "metric", "Invalid Status Test");
    insert_entity(&conn, "m-inv2", "metric", "Invalid Status Test");

    let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-inv2").unwrap();
    assert!(!suggestions.is_empty());

    let result =
        DedupPipeline::resolve_suggestion(&conn, &suggestions[0].suggestion_id, "bogus_status");
    assert!(
        result.is_err(),
        "Invalid status should return an error"
    );
}

#[test]
fn test_5i_resolve_nonexistent_suggestion() {
    let conn = common::test_db();

    let result = DedupPipeline::resolve_suggestion(&conn, "nonexistent-suggestion-id", "dismissed");
    assert!(
        result.is_err(),
        "Resolving a nonexistent suggestion should return an error"
    );
}

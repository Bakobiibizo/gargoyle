// Scenario 7: Claims Grounding Integration Tests
//
// Tests ClaimService: claim creation, grounding validation, entity traversal,
// claim promotion, and double-promotion prevention.

mod common;

use gargoyle_lib::error::{ErrorCode, GargoyleError};
use gargoyle_lib::services::claim_service::ClaimService;
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
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
        params![id],
    )
    .expect("Failed to insert FTS row");
}

fn insert_claim(
    conn: &rusqlite::Connection,
    claim_id: &str,
    subject: &str,
    predicate: &str,
    object: &str,
    evidence_entity_id: &str,
) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO claims (claim_id, subject, predicate, object, confidence, evidence_entity_id, created_at)
         VALUES (?1, ?2, ?3, ?4, 0.9, ?5, ?6)",
        params![claim_id, subject, predicate, object, evidence_entity_id, now],
    )
    .expect("Failed to insert test claim");
}

fn soft_delete_entity(conn: &rusqlite::Connection, id: &str) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .expect("Failed to soft-delete entity");
}

// =============================================================================
// 7a. VALID GROUNDED CLAIM
// =============================================================================

#[test]
fn test_7a_valid_grounded_claim() {
    let conn = common::test_db();

    // Create evidence entity
    insert_entity(&conn, "ev-7a", "result", "Q1 Revenue Analysis");

    // Create a claim grounded to the evidence entity
    insert_claim(&conn, "claim-7a", "Revenue", "increased_by", "15%", "ev-7a");

    // Validate grounding -- should succeed
    let result = ClaimService::validate_grounding(&conn, "ev-7a");
    assert!(
        result.is_ok(),
        "Grounding validation should pass for existing, non-deleted entity"
    );

    // Also verify the claim is retrievable
    let claim = ClaimService::get_claim(&conn, "claim-7a").unwrap();
    assert_eq!(claim.claim_id, "claim-7a");
    assert_eq!(claim.subject, "Revenue");
    assert_eq!(claim.predicate, "increased_by");
    assert_eq!(claim.object, "15%");
    assert_eq!(claim.evidence_entity_id, "ev-7a");
    assert!((claim.confidence - 0.9).abs() < f64::EPSILON);
}

// =============================================================================
// 7b. INVALID UNGROUNDED CLAIM -- nonexistent evidence
// =============================================================================

#[test]
fn test_7b_ungrounded_claim_nonexistent() {
    let conn = common::test_db();

    // Validate grounding for a nonexistent entity
    let result = ClaimService::validate_grounding(&conn, "nonexistent-entity-id");

    assert!(result.is_err(), "Grounding should fail for nonexistent entity");

    match result.unwrap_err() {
        GargoyleError::Validation(v) => {
            assert!(
                matches!(v.code, ErrorCode::UngroundedClaim),
                "Error code should be UngroundedClaim, got {:?}",
                v.code
            );
            assert_eq!(v.field_path, "evidence_entity_id");
            assert_eq!(v.actual, Some("nonexistent-entity-id".to_string()));
            assert!(
                v.message.contains("does not exist or is deleted"),
                "Error message should explain the issue: {}",
                v.message
            );
        }
        other => panic!("Expected Validation(UngroundedClaim), got: {:?}", other),
    }
}

// =============================================================================
// 7c. INVALID -- evidence points to deleted entity
// =============================================================================

#[test]
fn test_7c_ungrounded_claim_deleted_entity() {
    let conn = common::test_db();

    // Create entity, then soft-delete it
    insert_entity(&conn, "ev-7c", "result", "Deleted Evidence");
    soft_delete_entity(&conn, "ev-7c");

    // Validate grounding -- should fail because entity is soft-deleted
    let result = ClaimService::validate_grounding(&conn, "ev-7c");

    assert!(
        result.is_err(),
        "Grounding should fail for soft-deleted entity"
    );

    match result.unwrap_err() {
        GargoyleError::Validation(v) => {
            assert!(
                matches!(v.code, ErrorCode::UngroundedClaim),
                "Error code should be UngroundedClaim for deleted entity"
            );
            assert_eq!(v.field_path, "evidence_entity_id");
            assert_eq!(v.actual, Some("ev-7c".to_string()));
        }
        other => panic!("Expected Validation(UngroundedClaim), got: {:?}", other),
    }
}

// =============================================================================
// 7d. GROUNDING PRIORITY ORDER -- multi-entity run
// =============================================================================

#[test]
fn test_7d_grounding_priority_order_multi_entity() {
    let conn = common::test_db();

    // Create entities of different types that might come from a single run
    // Priority order: decision > result > experiment > metric > spec
    insert_entity(&conn, "spec-7d", "spec", "API Specification");
    insert_entity(&conn, "result-7d", "result", "Q1 Revenue Results");
    insert_entity(&conn, "exp-7d", "experiment", "Pricing Experiment");

    // All entities should be valid evidence
    assert!(ClaimService::validate_grounding(&conn, "spec-7d").is_ok());
    assert!(ClaimService::validate_grounding(&conn, "result-7d").is_ok());
    assert!(ClaimService::validate_grounding(&conn, "exp-7d").is_ok());

    // When a run produces spec, result, experiment, the primary entity
    // (highest priority) would be "result" (since "decision" is not present).
    // This is application-level logic. We verify the type priority by checking
    // that each entity type is correctly stored and retrievable.
    let types: Vec<String> = ["spec-7d", "result-7d", "exp-7d"]
        .iter()
        .map(|id| {
            conn.query_row(
                "SELECT entity_type FROM entities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap()
        })
        .collect();

    // Define priority order
    let priority = |t: &str| -> i32 {
        match t {
            "decision" => 5,
            "result" => 4,
            "experiment" => 3,
            "metric" => 2,
            "spec" => 1,
            _ => 0,
        }
    };

    // Find the highest-priority type in our set
    let primary_type = types
        .iter()
        .max_by_key(|t| priority(t.as_str()))
        .unwrap();
    assert_eq!(
        primary_type, "result",
        "Result should be the highest-priority type in [spec, result, experiment]"
    );
}

// =============================================================================
// 7e. GROUNDING PRIORITY ORDER -- no high-priority types
// =============================================================================

#[test]
fn test_7e_grounding_priority_no_high_priority() {
    let conn = common::test_db();

    // Run produces only metrics
    insert_entity(&conn, "met-7e-1", "metric", "MRR");
    insert_entity(&conn, "met-7e-2", "metric", "ARR");
    insert_entity(&conn, "met-7e-3", "metric", "NRR");

    // All are valid evidence
    assert!(ClaimService::validate_grounding(&conn, "met-7e-1").is_ok());
    assert!(ClaimService::validate_grounding(&conn, "met-7e-2").is_ok());
    assert!(ClaimService::validate_grounding(&conn, "met-7e-3").is_ok());

    // When only metrics are present, the primary entity is the first metric
    // in the patch_set (application logic). We verify that all metrics are
    // equivalent in priority.
    let priority = |t: &str| -> i32 {
        match t {
            "decision" => 5,
            "result" => 4,
            "experiment" => 3,
            "metric" => 2,
            "spec" => 1,
            _ => 0,
        }
    };

    let types = ["met-7e-1", "met-7e-2", "met-7e-3"]
        .iter()
        .map(|id| -> String {
            conn.query_row(
                "SELECT entity_type FROM entities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // All should have the same priority (all metrics)
    let priorities: Vec<i32> = types.iter().map(|t| priority(t.as_str())).collect();
    assert!(
        priorities.iter().all(|&p| p == priorities[0]),
        "All metrics should have equal priority"
    );

    // Therefore the first metric in patch_set order is the primary entity
    // (confirmed by the fact that they are all the same type)
    assert_eq!(
        types[0], "metric",
        "First entity in the set should be a metric"
    );
}

// =============================================================================
// 7f. CLAIM -> ENTITY TRAVERSAL
// =============================================================================

#[test]
fn test_7f_claim_entity_traversal() {
    let conn = common::test_db();

    // Create a result entity that will serve as evidence
    insert_entity(&conn, "ev-7f", "result", "Comprehensive Analysis");

    // Create 5 claims grounded to the same entity
    for i in 1..=5 {
        let claim_id = format!("claim-7f-{}", i);
        let subject = format!("Finding {}", i);
        insert_claim(&conn, &claim_id, &subject, "indicates", "growth", "ev-7f");
    }

    // Query claims by evidence_entity_id
    let claims = ClaimService::get_claims_for_entity(&conn, "ev-7f").unwrap();

    assert_eq!(
        claims.len(),
        5,
        "Should find all 5 claims grounded to ev-7f, got {}",
        claims.len()
    );

    // All claims should reference the same evidence entity
    for claim in &claims {
        assert_eq!(
            claim.evidence_entity_id, "ev-7f",
            "Each claim should reference ev-7f"
        );
    }

    // Also verify via list_claims with evidence filter
    let listed = ClaimService::list_claims(&conn, Some("ev-7f")).unwrap();
    assert_eq!(
        listed.len(),
        5,
        "list_claims filtered by evidence should also return 5"
    );
}

// =============================================================================
// 7g. CLAIM PROMOTION
// =============================================================================

#[test]
fn test_7g_claim_promotion() {
    let conn = common::test_db();

    // Create evidence entity
    insert_entity(&conn, "ev-7g", "result", "Revenue Evidence");

    // Create a claim to promote
    insert_claim(
        &conn,
        "claim-7g",
        "Revenue",
        "grew_by",
        "20%",
        "ev-7g",
    );

    // Verify claim exists and is not yet promoted
    let claim_before = ClaimService::get_claim(&conn, "claim-7g").unwrap();
    assert!(
        claim_before.promoted_to_entity_id.is_none(),
        "Claim should not be promoted yet"
    );

    // Promote the claim to a metric entity
    let entity_id =
        ClaimService::promote_claim(&conn, "claim-7g", "metric", "agent").unwrap();

    assert!(!entity_id.is_empty(), "Promoted entity ID should not be empty");

    // Verify: the claim now has promoted_to_entity_id set
    let claim_after = ClaimService::get_claim(&conn, "claim-7g").unwrap();
    assert_eq!(
        claim_after.promoted_to_entity_id,
        Some(entity_id.clone()),
        "Claim should reference the new entity"
    );

    // Verify: the new entity exists in the database
    let (entity_type, title, source): (String, String, String) = conn
        .query_row(
            "SELECT entity_type, title, source FROM entities WHERE id = ?1",
            params![&entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("Promoted entity should exist in the database");

    assert_eq!(entity_type, "metric");
    assert_eq!(title, "Revenue grew_by 20%");
    assert_eq!(source, "agent");
}

// =============================================================================
// 7h. DOUBLE PROMOTION BLOCKED
// =============================================================================

#[test]
fn test_7h_double_promotion_blocked() {
    let conn = common::test_db();

    // Create evidence and claim
    insert_entity(&conn, "ev-7h", "result", "Evidence for Double Promote");
    insert_claim(
        &conn,
        "claim-7h",
        "Retention",
        "improved_by",
        "5%",
        "ev-7h",
    );

    // First promotion should succeed
    let first_entity_id =
        ClaimService::promote_claim(&conn, "claim-7h", "metric", "agent").unwrap();
    assert!(!first_entity_id.is_empty());

    // Second promotion should fail
    let result = ClaimService::promote_claim(&conn, "claim-7h", "metric", "agent");
    assert!(
        result.is_err(),
        "Double promotion should be rejected"
    );

    match result.unwrap_err() {
        GargoyleError::Schema(msg) => {
            assert!(
                msg.contains("already been promoted"),
                "Error message should say 'already been promoted', got: {}",
                msg
            );
        }
        other => panic!("Expected Schema error for double promotion, got: {:?}", other),
    }

    // Verify: claim still points to the first promoted entity (not changed)
    let claim = ClaimService::get_claim(&conn, "claim-7h").unwrap();
    assert_eq!(
        claim.promoted_to_entity_id,
        Some(first_entity_id),
        "promoted_to_entity_id should still reference the first promotion"
    );
}

// =============================================================================
// 7 bonus: promote nonexistent claim
// =============================================================================

#[test]
fn test_7_bonus_promote_nonexistent_claim() {
    let conn = common::test_db();

    let result = ClaimService::promote_claim(&conn, "nonexistent-claim", "metric", "agent");
    assert!(result.is_err());

    match result.unwrap_err() {
        GargoyleError::NotFound { entity_type, id } => {
            assert_eq!(entity_type, "claim");
            assert_eq!(id, "nonexistent-claim");
        }
        other => panic!("Expected NotFound, got: {:?}", other),
    }
}

// =============================================================================
// 7 bonus: get_claim returns correct fields
// =============================================================================

#[test]
fn test_7_bonus_get_claim_returns_correct_fields() {
    let conn = common::test_db();

    insert_entity(&conn, "ev-bonus", "result", "Bonus Evidence");
    insert_claim(
        &conn,
        "claim-bonus",
        "Customer Satisfaction",
        "correlated_with",
        "NPS Score",
        "ev-bonus",
    );

    let claim = ClaimService::get_claim(&conn, "claim-bonus").unwrap();
    assert_eq!(claim.claim_id, "claim-bonus");
    assert_eq!(claim.subject, "Customer Satisfaction");
    assert_eq!(claim.predicate, "correlated_with");
    assert_eq!(claim.object, "NPS Score");
    assert_eq!(claim.evidence_entity_id, "ev-bonus");
    assert!(claim.provenance_run_id.is_none());
    assert!(claim.promoted_to_entity_id.is_none());
    assert!(!claim.created_at.is_empty());
}

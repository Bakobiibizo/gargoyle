// Scenario 2: Optimistic Locking Under Concurrency
//
// Tests the optimistic locking protocol:
//   2a. Valid sequential update
//   2b. Valid second sequential update
//   2c. Conflict -- stale expected_updated_at
//   2d. Conflict -- parallel agent simulation
//   2e. Recovery -- rebase after conflict

mod common;

use gargoyle_lib::error::GargoyleError;
use gargoyle_lib::models::patch::*;
use gargoyle_lib::patch::apply::apply_patch_set;
use gargoyle_lib::services::store::StoreService;

// =============================================================================
// 2a. VALID sequential update
// =============================================================================

#[test]
fn test_2a_valid_sequential_update() {
    let conn = common::test_db();

    // Create entity using direct SQL insert (with FTS) for reliable test setup
    let entity_id = "seq-update-2a";
    let t1 = common::insert_test_metric(&conn, entity_id, "MRR");

    // Small delay to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update with correct expected_updated_at = T1
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("MRR Updated".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 210000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let update_result = apply_patch_set(&conn, &update_set).expect("update should succeed");
    assert_eq!(update_result.applied.len(), 1);

    // Verify updated_at changed
    let updated_entity = StoreService::get_entity(&conn, entity_id).unwrap();
    let t2 = updated_entity.updated_at.clone();
    assert_ne!(t1, t2, "updated_at should change after update");
    assert_eq!(updated_entity.title, "MRR Updated");
    assert_eq!(updated_entity.canonical_fields["current_value"], 210000);
}

// =============================================================================
// 2b. VALID second sequential update
// =============================================================================

#[test]
fn test_2b_valid_second_sequential_update() {
    let conn = common::test_db();

    // Create entity using direct SQL insert
    let entity_id = "seq-update-2b";
    let t1 = common::insert_test_metric(&conn, entity_id, "MRR");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // First update
    let update1 = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("MRR v2".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 210000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };
    apply_patch_set(&conn, &update1).expect("first update should succeed");

    // Read again to get T2
    let entity_v2 = StoreService::get_entity(&conn, entity_id).unwrap();
    let t2 = entity_v2.updated_at.clone();
    assert_ne!(t1, t2, "T2 should differ from T1");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Second update with T2
    let update2 = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t2.clone(),
            title: Some("MRR v3".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 220000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };
    let result2 = apply_patch_set(&conn, &update2).expect("second update should succeed");
    assert_eq!(result2.applied.len(), 1);

    // Verify final state
    let entity_v3 = StoreService::get_entity(&conn, entity_id).unwrap();
    let t3 = entity_v3.updated_at.clone();
    assert_ne!(t2, t3, "T3 should differ from T2");
    assert_eq!(entity_v3.title, "MRR v3");
    assert_eq!(entity_v3.canonical_fields["current_value"], 220000);
}

// =============================================================================
// 2c. CONFLICT -- stale expected_updated_at
// =============================================================================

#[test]
fn test_2c_conflict_stale_expected_updated_at() {
    let conn = common::test_db();

    // Create entity
    let entity_id = "lock-conflict-2c";
    let t1 = common::insert_test_metric(&conn, entity_id, "MRR");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Simulate concurrent write: update directly to set updated_at = T2
    let t2 = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "UPDATE entities SET updated_at = ?1, title = 'MRR Concurrent' WHERE id = ?2",
        rusqlite::params![t2, entity_id],
    )
    .expect("Direct update should succeed");

    // Attempt update with stale T1
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("MRR Stale".to_string()),
            body_md: None,
            status: None,
            canonical_fields: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_err(), "Should reject stale expected_updated_at");

    match result.unwrap_err() {
        GargoyleError::LockConflict { expected, found } => {
            assert_eq!(expected, t1, "Expected should be the stale T1");
            assert_eq!(found, t2, "Found should be the current T2");
        }
        other => panic!("Expected LockConflict, got: {:?}", other),
    }

    // Verify the entity was NOT modified by the failed update
    let entity = common::get_entity_row(&conn, entity_id).unwrap();
    assert_eq!(entity.2, "MRR Concurrent", "Title should remain from concurrent write");
}

// =============================================================================
// 2d. CONFLICT -- parallel agent simulation
// =============================================================================

#[test]
fn test_2d_conflict_parallel_agent_simulation() {
    let conn = common::test_db();

    // Create entity using direct SQL insert
    let entity_id = "parallel-2d";
    let t1 = common::insert_test_metric(&conn, entity_id, "MRR");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Both agents read T1 and prepare updates
    let update1 = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("Agent 1 Update".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 210000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let update2 = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("Agent 2 Update".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 220000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    // Apply first update: should succeed (T1 -> T2)
    let result1 = apply_patch_set(&conn, &update1);
    assert!(result1.is_ok(), "First update should succeed: {:?}", result1.err());

    // Apply second update: should fail (expected T1, but now T2)
    let result2 = apply_patch_set(&conn, &update2);
    assert!(result2.is_err(), "Second update should fail with LockConflict");

    match result2.unwrap_err() {
        GargoyleError::LockConflict { expected, found } => {
            assert_eq!(expected, t1, "Expected should be the stale T1");
            assert_ne!(found, t1, "Found should be the new timestamp T2");
        }
        other => panic!("Expected LockConflict, got: {:?}", other),
    }

    // Verify the winning update
    let final_entity = StoreService::get_entity(&conn, entity_id).unwrap();
    assert_eq!(
        final_entity.title, "Agent 1 Update",
        "First agent's update should win"
    );
    assert_eq!(
        final_entity.canonical_fields["current_value"], 210000,
        "First agent's value should persist"
    );
}

// =============================================================================
// 2e. RECOVERY -- rebase after conflict
// =============================================================================

#[test]
fn test_2e_recovery_rebase_after_conflict() {
    let conn = common::test_db();

    // Create entity using direct SQL insert
    let entity_id = "rebase-2e";
    let t1 = common::insert_test_metric(&conn, entity_id, "MRR");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Agent 1 succeeds
    let update1 = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("Agent 1 Wins".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 210000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };
    apply_patch_set(&conn, &update1).expect("Agent 1 should succeed");

    // Agent 2 fails
    let update2_stale = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t1.clone(),
            title: Some("Agent 2 Stale".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 220000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };
    let conflict = apply_patch_set(&conn, &update2_stale);
    assert!(
        conflict.is_err(),
        "Agent 2 should fail with stale timestamp"
    );
    assert!(
        matches!(conflict.unwrap_err(), GargoyleError::LockConflict { .. }),
        "Should be LockConflict"
    );

    // RECOVERY: Agent 2 re-reads the entity to get current T2
    let entity_after_conflict = StoreService::get_entity(&conn, entity_id).unwrap();
    let t2 = entity_after_conflict.updated_at.clone();
    assert_ne!(t2, t1, "T2 should be different from T1");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Agent 2 retries with correct T2
    let update2_rebased = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.to_string(),
            expected_updated_at: t2.clone(),
            title: Some("Agent 2 Rebased".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 220000})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };
    let rebase_result = apply_patch_set(&conn, &update2_rebased);
    assert!(
        rebase_result.is_ok(),
        "Rebased update should succeed, got: {:?}",
        rebase_result.err()
    );

    // Verify final state
    let final_entity = StoreService::get_entity(&conn, entity_id).unwrap();
    assert_eq!(final_entity.title, "Agent 2 Rebased");
    assert_eq!(final_entity.canonical_fields["current_value"], 220000);
    assert_ne!(
        final_entity.updated_at, t2,
        "updated_at should advance past T2"
    );
}

// Scenario 4: Status State Machine
//
// Tests the status transition rules for experiments:
//   4a. Valid forward transition: draft -> running
//   4b. Valid skip transition: running -> archived
//   4c. Valid backward transition WITH reason: archived -> running
//   4d. Invalid backward transition WITHOUT reason: concluded -> running
//   4e. Invalid status value: status="completed" on experiment
//   4f. Idempotent same-status: running -> running
//   4g. NULL -> first status: null -> "draft"

mod common;

use gargoyle_lib::error::ErrorCode;
use gargoyle_lib::models::patch::*;
use gargoyle_lib::patch::apply::apply_patch_set;
use gargoyle_lib::services::store::StoreService;
use gargoyle_lib::validation::status_validator::validate_status_transition;

// =============================================================================
// Helpers
// =============================================================================

/// Create an experiment entity and return its ID.
fn create_experiment(conn: &rusqlite::Connection, title: &str, status: Option<&str>) -> String {
    let patch_set = PatchSet {
        ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: "experiment".to_string(),
            title: title.to_string(),
            source: "manual".to_string(),
            canonical_fields: serde_json::json!({"hypothesis": "test hypothesis"}),
            body_md: None,
            status: status.map(|s| s.to_string()),
            category: None,
            priority: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(conn, &patch_set).expect("experiment creation should succeed");
    result.applied[0]
        .entity_id
        .as_ref()
        .expect("Should have entity_id")
        .clone()
}

/// Set status on an entity directly in the database (bypasses validation for test setup).
fn set_status_directly(conn: &rusqlite::Connection, entity_id: &str, status: &str) {
    conn.execute(
        "UPDATE entities SET status = ?1 WHERE id = ?2",
        rusqlite::params![status, entity_id],
    )
    .expect("Failed to set status directly");
}

// =============================================================================
// 4a. VALID forward transition: draft -> running
// =============================================================================

#[test]
fn test_4a_valid_forward_transition_draft_to_running() {
    // Test the status validator directly
    let errors = validate_status_transition("experiment", Some("draft"), "running", None);
    assert!(
        errors.is_empty(),
        "draft -> running should be valid (forward, no reason needed): {:?}",
        errors
    );
}

#[test]
fn test_4a_forward_transition_via_patch() {
    let conn = common::test_db();

    // Create experiment at "draft" status
    let entity_id = create_experiment(&conn, "Forward Test", Some("draft"));
    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(entity.status, Some("draft".to_string()));

    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update status: draft -> running
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.clone(),
            expected_updated_at: entity.updated_at.clone(),
            title: None,
            body_md: None,
            status: Some("running".to_string()),
            canonical_fields: None,
            category: None,
            priority: None,
            reason: None, // No reason needed for forward transition
        })],
        run_id: None,
    };

    // Note: Status validation is not wired into apply_patch_set, so this succeeds
    // because the DB accepts any string for status. We test the validator separately.
    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Forward transition should succeed");

    let updated = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(updated.status, Some("running".to_string()));
}

// =============================================================================
// 4b. VALID skip transition: running -> archived
// =============================================================================

#[test]
fn test_4b_valid_skip_transition_running_to_archived() {
    // Skipping "concluded" and going directly to "archived"
    let errors = validate_status_transition("experiment", Some("running"), "archived", None);
    assert!(
        errors.is_empty(),
        "running -> archived should be valid (forward skip): {:?}",
        errors
    );
}

#[test]
fn test_4b_skip_transition_via_patch() {
    let conn = common::test_db();

    let entity_id = create_experiment(&conn, "Skip Test", Some("draft"));
    set_status_directly(&conn, &entity_id, "running");

    let updated_at = common::get_updated_at(&conn, &entity_id);
    std::thread::sleep(std::time::Duration::from_millis(10));

    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.clone(),
            expected_updated_at: updated_at,
            title: None,
            body_md: None,
            status: Some("archived".to_string()),
            canonical_fields: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Skip transition should succeed");

    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(entity.status, Some("archived".to_string()));
}

// =============================================================================
// 4c. VALID backward transition WITH reason: archived -> running
// =============================================================================

#[test]
fn test_4c_valid_backward_transition_with_reason() {
    let errors = validate_status_transition(
        "experiment",
        Some("archived"),
        "running",
        Some("Re-opened due to new data"),
    );
    assert!(
        errors.is_empty(),
        "archived -> running with reason should be valid: {:?}",
        errors
    );
}

#[test]
fn test_4c_backward_transition_via_patch_with_reason() {
    let conn = common::test_db();

    let entity_id = create_experiment(&conn, "Backward Test", Some("draft"));
    set_status_directly(&conn, &entity_id, "archived");

    let updated_at = common::get_updated_at(&conn, &entity_id);
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Note: The reason field is on the UpdateEntityPayload but status validation
    // is not wired into apply_patch_set. The update succeeds regardless.
    // We verify the validator would accept this.
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.clone(),
            expected_updated_at: updated_at,
            title: None,
            body_md: None,
            status: Some("running".to_string()),
            canonical_fields: None,
            category: None,
            priority: None,
            reason: Some("Re-opened due to new data".to_string()),
        })],
        run_id: None,
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Backward transition with reason should succeed");

    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(entity.status, Some("running".to_string()));
}

// =============================================================================
// 4d. INVALID backward transition WITHOUT reason: concluded -> running
// =============================================================================

#[test]
fn test_4d_invalid_backward_transition_without_reason() {
    let errors = validate_status_transition(
        "experiment",
        Some("concluded"),
        "running",
        None, // No reason provided
    );

    assert!(
        !errors.is_empty(),
        "concluded -> running without reason should be rejected"
    );
    assert_eq!(errors.len(), 1, "Should have exactly one error");
    assert!(
        matches!(errors[0].code, ErrorCode::InvalidStatusTransition),
        "Should be InvalidStatusTransition, got: {:?}",
        errors[0].code
    );
    assert!(
        errors[0].message.contains("requires a reason"),
        "Error should mention reason requirement: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("concluded"),
        "Error should mention current status: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("running"),
        "Error should mention target status: {}",
        errors[0].message
    );
}

#[test]
fn test_4d_backward_transition_empty_reason_also_rejected() {
    let errors = validate_status_transition(
        "experiment",
        Some("concluded"),
        "running",
        Some(""), // Empty reason
    );

    assert!(
        !errors.is_empty(),
        "Empty reason should also be rejected for backward transition"
    );
    assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
}

#[test]
fn test_4d_backward_transition_whitespace_reason_rejected() {
    let errors = validate_status_transition(
        "experiment",
        Some("concluded"),
        "running",
        Some("   "), // Whitespace-only reason
    );

    assert!(
        !errors.is_empty(),
        "Whitespace-only reason should be rejected for backward transition"
    );
    assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
}

// =============================================================================
// 4e. INVALID status value: status="completed" on experiment
// =============================================================================

#[test]
fn test_4e_invalid_status_value_for_entity_type() {
    // "completed" is not a valid experiment status
    // (valid: draft, running, concluded, archived)
    let errors = validate_status_transition(
        "experiment",
        Some("draft"),
        "completed",
        None,
    );

    assert!(
        !errors.is_empty(),
        "Should reject invalid status 'completed' for experiment"
    );
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(errors[0].code, ErrorCode::InvalidStatusTransition),
        "Should be InvalidStatusTransition, got: {:?}",
        errors[0].code
    );

    // Verify the error contains the valid status list
    assert!(
        errors[0].message.contains("completed"),
        "Error should mention the invalid status: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("draft"),
        "Error should list valid statuses: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("running"),
        "Error should list valid statuses: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("concluded"),
        "Error should list valid statuses: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("archived"),
        "Error should list valid statuses: {}",
        errors[0].message
    );
}

#[test]
fn test_4e_invalid_status_from_null() {
    // "completed" is also invalid when transitioning from null
    let errors = validate_status_transition(
        "experiment",
        None,
        "completed",
        None,
    );

    assert!(
        !errors.is_empty(),
        "Should reject invalid status 'completed' even from null"
    );
    assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
}

// =============================================================================
// 4f. IDEMPOTENT same-status: running -> running
// =============================================================================

#[test]
fn test_4f_idempotent_same_status() {
    let errors = validate_status_transition(
        "experiment",
        Some("running"),
        "running",
        None,
    );

    assert!(
        errors.is_empty(),
        "running -> running should be valid (idempotent no-op): {:?}",
        errors
    );
}

#[test]
fn test_4f_idempotent_all_experiment_statuses() {
    // Verify idempotent transitions for all valid experiment statuses
    for status in &["draft", "running", "concluded", "archived"] {
        let errors = validate_status_transition(
            "experiment",
            Some(status),
            status,
            None,
        );
        assert!(
            errors.is_empty(),
            "{} -> {} should be idempotent: {:?}",
            status,
            status,
            errors
        );
    }
}

#[test]
fn test_4f_idempotent_via_patch() {
    let conn = common::test_db();

    let entity_id = create_experiment(&conn, "Idempotent Test", Some("draft"));
    set_status_directly(&conn, &entity_id, "running");

    let updated_at = common::get_updated_at(&conn, &entity_id);
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Set running -> running (same status)
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.clone(),
            expected_updated_at: updated_at,
            title: None,
            body_md: None,
            status: Some("running".to_string()),
            canonical_fields: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Same-status update should succeed");

    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(entity.status, Some("running".to_string()));
}

// =============================================================================
// 4g. NULL -> first status: null -> "draft"
// =============================================================================

#[test]
fn test_4g_null_to_first_status() {
    // null -> "draft" should be valid for any entity type
    let errors = validate_status_transition(
        "experiment",
        None,
        "draft",
        None,
    );

    assert!(
        errors.is_empty(),
        "null -> draft should be valid: {:?}",
        errors
    );
}

#[test]
fn test_4g_null_to_any_valid_status() {
    // null -> any valid status should succeed for experiment
    for status in &["draft", "running", "concluded", "archived"] {
        let errors = validate_status_transition(
            "experiment",
            None,
            status,
            None,
        );
        assert!(
            errors.is_empty(),
            "null -> {} should be valid for experiment: {:?}",
            status,
            errors
        );
    }
}

#[test]
fn test_4g_null_to_first_status_via_patch() {
    let conn = common::test_db();

    // Create experiment with no status
    let entity_id = create_experiment(&conn, "Null Status Test", None);
    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert!(entity.status.is_none(), "Status should initially be null");

    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update: null -> draft
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.clone(),
            expected_updated_at: entity.updated_at.clone(),
            title: None,
            body_md: None,
            status: Some("draft".to_string()),
            canonical_fields: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "null -> draft should succeed");

    let updated = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(updated.status, Some("draft".to_string()));
}

// =============================================================================
// Additional: metric status transitions
// =============================================================================

#[test]
fn test_metric_forward_active_to_paused() {
    let errors = validate_status_transition("metric", Some("active"), "paused", None);
    assert!(errors.is_empty(), "active -> paused should be valid");
}

#[test]
fn test_metric_backward_paused_to_active_without_reason() {
    let errors = validate_status_transition("metric", Some("paused"), "active", None);
    assert!(
        !errors.is_empty(),
        "paused -> active without reason should be rejected"
    );
    assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
}

#[test]
fn test_metric_backward_paused_to_active_with_reason() {
    let errors = validate_status_transition(
        "metric",
        Some("paused"),
        "active",
        Some("Reactivating metric after review"),
    );
    assert!(
        errors.is_empty(),
        "paused -> active with reason should be valid: {:?}",
        errors
    );
}

// =============================================================================
// Additional: result status transitions
// =============================================================================

#[test]
fn test_result_forward_draft_to_final() {
    let errors = validate_status_transition("result", Some("draft"), "final", None);
    assert!(errors.is_empty(), "draft -> final should be valid");
}

#[test]
fn test_result_backward_final_to_draft_without_reason() {
    let errors = validate_status_transition("result", Some("final"), "draft", None);
    assert!(
        !errors.is_empty(),
        "final -> draft without reason should be rejected"
    );
}

#[test]
fn test_result_backward_final_to_draft_with_reason() {
    let errors = validate_status_transition(
        "result",
        Some("final"),
        "draft",
        Some("Corrections needed"),
    );
    assert!(
        errors.is_empty(),
        "final -> draft with reason should be valid: {:?}",
        errors
    );
}

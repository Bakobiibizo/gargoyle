// Scenario 4: Status State Machine
//
// Tests the status transition rules for experiments:
//   4a. Valid forward transition: draft -> running
//   4b. Valid skip transition: running -> archived (with skip warning)
//   4c. Valid backward transition WITH reason: archived -> running (with info warning)
//   4d. Backward transition WITHOUT reason: concluded -> running (soft constraint -- succeeds with warning)
//   4e. Invalid status value: status="completed" on experiment (hard error)
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
            canonical_fields: serde_json::json!({"hypothesis": "test hypothesis", "primary_metric": "conversion_rate"}),
            body_md: None,
            status: status.map(|s| s.to_string()),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: "test-run".to_string(),
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
    let result = validate_status_transition("experiment", Some("draft"), "running", None);
    assert!(
        result.errors.is_empty(),
        "draft -> running should be valid (forward, no reason needed): {:?}",
        result.errors
    );
    assert!(
        result.warnings.is_empty(),
        "Adjacent forward transition should produce no warnings"
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
        run_id: "test-run".to_string(),
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Forward transition should succeed");

    let updated = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(updated.status, Some("running".to_string()));
}

// =============================================================================
// 4b. VALID skip transition: running -> archived (with skip warning)
// =============================================================================

#[test]
fn test_4b_valid_skip_transition_running_to_archived() {
    // Skipping "concluded" and going directly to "archived"
    let result = validate_status_transition("experiment", Some("running"), "archived", None);
    assert!(
        result.errors.is_empty(),
        "running -> archived should be valid (forward skip): {:?}",
        result.errors
    );
    // Skip transitions generate a warning
    assert_eq!(
        result.warnings.len(),
        1,
        "Skip transition should produce a warning"
    );
    assert!(
        result.warnings[0].contains("skip"),
        "Warning should mention 'skip': {}",
        result.warnings[0]
    );
    assert!(
        result.warnings[0].contains("concluded"),
        "Warning should mention skipped status 'concluded': {}",
        result.warnings[0]
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
        run_id: "test-run".to_string(),
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Skip transition should succeed");

    let patch_result = result.unwrap();
    // Skip transition should produce a warning in PatchResult
    assert!(
        !patch_result.warnings.is_empty(),
        "Skip transition should surface warning in PatchResult"
    );

    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(entity.status, Some("archived".to_string()));
}

// =============================================================================
// 4c. VALID backward transition WITH reason: archived -> running
// =============================================================================

#[test]
fn test_4c_valid_backward_transition_with_reason() {
    let result = validate_status_transition(
        "experiment",
        Some("archived"),
        "running",
        Some("Re-opened due to new data"),
    );
    assert!(
        result.errors.is_empty(),
        "archived -> running with reason should be valid: {:?}",
        result.errors
    );
    // Backward transition with reason produces an informational warning
    assert_eq!(result.warnings.len(), 1);
    assert!(
        result.warnings[0].contains("Re-opened due to new data"),
        "Warning should include the reason: {}",
        result.warnings[0]
    );
}

#[test]
fn test_4c_backward_transition_via_patch_with_reason() {
    let conn = common::test_db();

    let entity_id = create_experiment(&conn, "Backward Test", Some("draft"));
    set_status_directly(&conn, &entity_id, "archived");

    let updated_at = common::get_updated_at(&conn, &entity_id);
    std::thread::sleep(std::time::Duration::from_millis(10));

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
        run_id: "test-run".to_string(),
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(result.is_ok(), "Backward transition with reason should succeed");

    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(entity.status, Some("running".to_string()));
}

// =============================================================================
// 4d. BACKWARD transition WITHOUT reason: concluded -> running
//     (soft constraint -- succeeds with warning, NOT a hard error)
// =============================================================================

#[test]
fn test_4d_backward_transition_without_reason_succeeds_with_warning() {
    let result = validate_status_transition(
        "experiment",
        Some("concluded"),
        "running",
        None, // No reason provided
    );

    // Backward transitions are now soft constraints -- NO errors
    assert!(
        result.errors.is_empty(),
        "concluded -> running without reason should succeed (soft constraint): {:?}",
        result.errors
    );
    // But there should be a warning
    assert_eq!(
        result.warnings.len(),
        1,
        "Should have exactly one warning"
    );
    assert!(
        result.warnings[0].contains("Backward"),
        "Warning should mention backward transition: {}",
        result.warnings[0]
    );
    assert!(
        result.warnings[0].contains("without reason"),
        "Warning should mention missing reason: {}",
        result.warnings[0]
    );
    assert!(
        result.warnings[0].contains("concluded"),
        "Warning should mention current status: {}",
        result.warnings[0]
    );
    assert!(
        result.warnings[0].contains("running"),
        "Warning should mention target status: {}",
        result.warnings[0]
    );
}

#[test]
fn test_4d_backward_transition_empty_reason_warns() {
    let result = validate_status_transition(
        "experiment",
        Some("concluded"),
        "running",
        Some(""), // Empty reason
    );

    // Empty reason is treated the same as no reason -- soft constraint
    assert!(
        result.errors.is_empty(),
        "Empty reason should not produce errors"
    );
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("without reason"));
}

#[test]
fn test_4d_backward_transition_whitespace_reason_warns() {
    let result = validate_status_transition(
        "experiment",
        Some("concluded"),
        "running",
        Some("   "), // Whitespace-only reason
    );

    // Whitespace-only reason is treated the same as no reason -- soft constraint
    assert!(
        result.errors.is_empty(),
        "Whitespace-only reason should not produce errors"
    );
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("without reason"));
}

// =============================================================================
// 4e. INVALID status value: status="completed" on experiment (hard error)
// =============================================================================

#[test]
fn test_4e_invalid_status_value_for_entity_type() {
    // "completed" is not a valid experiment status
    // (valid: draft, running, concluded, archived)
    let result = validate_status_transition(
        "experiment",
        Some("draft"),
        "completed",
        None,
    );

    assert!(
        !result.errors.is_empty(),
        "Should reject invalid status 'completed' for experiment"
    );
    assert_eq!(result.errors.len(), 1);
    assert!(
        matches!(result.errors[0].code, ErrorCode::InvalidStatusTransition),
        "Should be InvalidStatusTransition, got: {:?}",
        result.errors[0].code
    );

    // Verify the error contains the valid status list
    assert!(
        result.errors[0].message.contains("completed"),
        "Error should mention the invalid status: {}",
        result.errors[0].message
    );
    assert!(
        result.errors[0].message.contains("draft"),
        "Error should list valid statuses: {}",
        result.errors[0].message
    );
    assert!(
        result.errors[0].message.contains("running"),
        "Error should list valid statuses: {}",
        result.errors[0].message
    );
    assert!(
        result.errors[0].message.contains("concluded"),
        "Error should list valid statuses: {}",
        result.errors[0].message
    );
    assert!(
        result.errors[0].message.contains("archived"),
        "Error should list valid statuses: {}",
        result.errors[0].message
    );
}

#[test]
fn test_4e_invalid_status_from_null() {
    // "completed" is also invalid when transitioning from null
    let result = validate_status_transition(
        "experiment",
        None,
        "completed",
        None,
    );

    assert!(
        !result.errors.is_empty(),
        "Should reject invalid status 'completed' even from null"
    );
    assert!(matches!(result.errors[0].code, ErrorCode::InvalidStatusTransition));
}

// =============================================================================
// 4f. IDEMPOTENT same-status: running -> running
// =============================================================================

#[test]
fn test_4f_idempotent_same_status() {
    let result = validate_status_transition(
        "experiment",
        Some("running"),
        "running",
        None,
    );

    assert!(
        result.errors.is_empty(),
        "running -> running should be valid (idempotent no-op): {:?}",
        result.errors
    );
    assert!(result.warnings.is_empty());
}

#[test]
fn test_4f_idempotent_all_experiment_statuses() {
    // Verify idempotent transitions for all valid experiment statuses
    for status in &["draft", "running", "concluded", "archived"] {
        let result = validate_status_transition(
            "experiment",
            Some(status),
            status,
            None,
        );
        assert!(
            result.errors.is_empty(),
            "{} -> {} should be idempotent: {:?}",
            status,
            status,
            result.errors
        );
        assert!(result.warnings.is_empty());
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
        run_id: "test-run".to_string(),
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
    let result = validate_status_transition(
        "experiment",
        None,
        "draft",
        None,
    );

    assert!(
        result.errors.is_empty(),
        "null -> draft should be valid: {:?}",
        result.errors
    );
    assert!(result.warnings.is_empty());
}

#[test]
fn test_4g_null_to_any_valid_status() {
    // null -> any valid status should succeed for experiment
    for status in &["draft", "running", "concluded", "archived"] {
        let result = validate_status_transition(
            "experiment",
            None,
            status,
            None,
        );
        assert!(
            result.errors.is_empty(),
            "null -> {} should be valid for experiment: {:?}",
            status,
            result.errors
        );
        assert!(result.warnings.is_empty());
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
        run_id: "test-run".to_string(),
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
    let result = validate_status_transition("metric", Some("active"), "paused", None);
    assert!(result.errors.is_empty(), "active -> paused should be valid");
    assert!(result.warnings.is_empty());
}

#[test]
fn test_metric_backward_paused_to_active_without_reason_warns() {
    let result = validate_status_transition("metric", Some("paused"), "active", None);
    // Backward transitions are soft constraints -- no errors
    assert!(
        result.errors.is_empty(),
        "paused -> active without reason should succeed (soft constraint)"
    );
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("Backward"));
    assert!(result.warnings[0].contains("without reason"));
}

#[test]
fn test_metric_backward_paused_to_active_with_reason() {
    let result = validate_status_transition(
        "metric",
        Some("paused"),
        "active",
        Some("Reactivating metric after review"),
    );
    assert!(
        result.errors.is_empty(),
        "paused -> active with reason should be valid: {:?}",
        result.errors
    );
    // Informational warning
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("Reactivating metric after review"));
}

// =============================================================================
// Additional: result status transitions
// =============================================================================

#[test]
fn test_result_forward_preliminary_to_final() {
    let result = validate_status_transition("result", Some("preliminary"), "final", None);
    assert!(result.errors.is_empty(), "preliminary -> final should be valid");
    assert!(result.warnings.is_empty());
}

#[test]
fn test_result_backward_final_to_preliminary_without_reason_warns() {
    let result = validate_status_transition("result", Some("final"), "preliminary", None);
    // Backward transitions are soft constraints -- no errors
    assert!(
        result.errors.is_empty(),
        "final -> preliminary without reason should succeed (soft constraint)"
    );
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("Backward"));
}

#[test]
fn test_result_backward_final_to_preliminary_with_reason() {
    let result = validate_status_transition(
        "result",
        Some("final"),
        "preliminary",
        Some("Corrections needed"),
    );
    assert!(
        result.errors.is_empty(),
        "final -> preliminary with reason should be valid: {:?}",
        result.errors
    );
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("Corrections needed"));
}

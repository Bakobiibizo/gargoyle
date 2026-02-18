// Scenario 1: Patch Protocol Fundamentals
//
// Tests the patch protocol's core operations:
//   1a. Valid create_entity (metric) with canonical_fields
//   1b. Invalid create_entity -- invalid status for entity type
//   1c. Invalid create_entity -- entity_ref type mismatch
//   1d. Invalid create_entity -- bad enum value
//   1e. Valid create_relation
//   1f. Invalid create_relation -- nonexistent target
//   1g. Invalid create_relation -- unapproved custom type
//   1h. Invalid create_relation -- soft-deleted target

mod common;

use gargoyle_lib::error::ErrorCode;
use gargoyle_lib::models::patch::*;
use gargoyle_lib::patch::apply::apply_patch_set;
use gargoyle_lib::schema::registry::SchemaRegistry;
use gargoyle_lib::services::store::StoreService;
use gargoyle_lib::validation::referential_validator::{validate_entity_refs, validate_relation_refs};
use gargoyle_lib::validation::schema_validator::validate_canonical_fields;
use gargoyle_lib::validation::status_validator::validate_status_transition;

// =============================================================================
// Mock lookups for validation tests
// These simulate database lookups using predefined entity mappings.
// =============================================================================

/// Returns a mock lookup matching the setup for test 1c.
/// "metric-ref" -> metric, "experiment-ref" -> experiment
fn lookup_1c(id: &str) -> Option<(String, Option<String>)> {
    match id {
        "metric-entity" => Some(("metric".to_string(), None)),
        "project-entity" => Some(("project".to_string(), None)),
        _ => None,
    }
}

/// Lookup for 1f: one valid entity, one missing
fn lookup_1f(id: &str) -> Option<(String, Option<String>)> {
    match id {
        "valid-metric" => Some(("metric".to_string(), None)),
        _ => None, // "nonexistent-uuid" returns None
    }
}

/// Lookup for 1g: two valid entities
fn lookup_1g(id: &str) -> Option<(String, Option<String>)> {
    match id {
        "met-1g" => Some(("metric".to_string(), None)),
        "exp-1g" => Some(("experiment".to_string(), None)),
        _ => None,
    }
}

/// Lookup for 1h: one valid, one soft-deleted
fn lookup_1h(id: &str) -> Option<(String, Option<String>)> {
    match id {
        "met-1h" => Some(("metric".to_string(), None)),
        "exp-1h" => Some(("experiment".to_string(), Some("2025-01-01T00:00:00.000Z".to_string()))),
        _ => None,
    }
}

// =============================================================================
// 1a. VALID create_entity (metric)
// =============================================================================

#[test]
fn test_1a_valid_create_entity_metric() {
    let conn = common::test_db();

    let patch_set = PatchSet {
        ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: "metric".to_string(),
            title: "MRR".to_string(),
            source: "template".to_string(),
            canonical_fields: serde_json::json!({
                "current_value": 200000,
                "target_value": 300000,
                "trend": "up",
                "data_source": "Stripe"
            }),
            body_md: None,
            status: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: "test-run".to_string(),
    };

    let result = apply_patch_set(&conn, &patch_set).expect("create_entity should succeed");
    assert_eq!(result.applied.len(), 1, "Should have one applied op");
    assert!(result.errors.is_empty(), "Should have no errors");

    let entity_id = result.applied[0].entity_id.as_ref().expect("Should have entity_id");

    // Verify entity was created with correct properties
    let entity = StoreService::get_entity(&conn, entity_id).expect("Entity should exist");
    assert_eq!(entity.title, "MRR");
    assert_eq!(entity.entity_type, "metric");
    assert_eq!(entity.schema_version, 1, "_schema_version should be 1");
    assert!(entity.status.is_none(), "Status should default to null");
    assert_eq!(entity.canonical_fields["current_value"], 200000);
    assert_eq!(entity.canonical_fields["target_value"], 300000);
    assert_eq!(entity.canonical_fields["trend"], "up");
    assert_eq!(entity.canonical_fields["data_source"], "Stripe");
}

#[test]
fn test_1a_valid_canonical_fields_pass_schema_validation() {
    let registry = SchemaRegistry::new();
    let fields = serde_json::json!({
        "current_value": 200000,
        "target_value": 300000,
        "trend": "up",
        "data_source": "Stripe"
    });

    // Validate through the schema registry
    let errors = registry.validate_canonical_fields("metric", 1, &fields);
    assert!(
        errors.is_empty(),
        "Valid metric canonical_fields should pass validation: {:?}",
        errors
    );
}

// =============================================================================
// 1b. INVALID create_entity -- invalid status for entity type
// =============================================================================

#[test]
fn test_1b_invalid_create_entity_bad_status() {
    // "completed" is NOT valid for metric (valid: active, paused, deprecated, archived)
    let result = validate_status_transition(
        "metric",
        None, // null -> "completed"
        "completed",
        None,
    );

    assert!(
        !result.errors.is_empty(),
        "Should reject invalid status 'completed' for metric"
    );
    assert!(
        result.errors.iter().any(|e| matches!(e.code, ErrorCode::InvalidStatusTransition)),
        "Should have InvalidStatusTransition error, got: {:?}",
        result.errors
    );

    let status_error = result.errors
        .iter()
        .find(|e| matches!(e.code, ErrorCode::InvalidStatusTransition))
        .unwrap();
    assert!(
        status_error.message.contains("completed"),
        "Error should mention the invalid status: {}",
        status_error.message
    );
    assert!(
        status_error.message.contains("active"),
        "Error should list valid status 'active': {}",
        status_error.message
    );
    assert!(
        status_error.message.contains("archived"),
        "Error should list valid status 'archived': {}",
        status_error.message
    );
}

#[test]
fn test_1b_invalid_status_for_experiment() {
    // "completed" is not valid for experiment either (valid: draft, running, concluded, archived)
    let result = validate_status_transition(
        "experiment",
        None,
        "completed",
        None,
    );

    assert!(
        !result.errors.is_empty(),
        "Should reject 'completed' for experiment"
    );
    assert!(matches!(result.errors[0].code, ErrorCode::InvalidStatusTransition));
    assert!(result.errors[0].message.contains("draft"));
    assert!(result.errors[0].message.contains("running"));
    assert!(result.errors[0].message.contains("concluded"));
    assert!(result.errors[0].message.contains("archived"));
}

// =============================================================================
// 1c. INVALID create_entity -- entity_ref type mismatch
// =============================================================================

#[test]
fn test_1c_invalid_create_entity_ref_type_mismatch() {
    // project_id on task is EntityRef("project") -- pointing to a metric should fail.
    let registry = SchemaRegistry::new();
    let task_field_defs = registry
        .get_schema("task", 1)
        .expect("task v1 schema should exist");

    let canonical_fields = serde_json::json!({
        "project_id": "metric-entity"
    });

    // The mock lookup returns "metric-entity" as type "metric",
    // but project_id expects type "project"
    let errors = validate_entity_refs(
        &canonical_fields,
        &task_field_defs,
        &lookup_1c,
    );

    assert!(
        !errors.is_empty(),
        "Should reject entity_ref pointing to wrong type"
    );
    assert!(
        errors.iter().any(|e| matches!(e.code, ErrorCode::EntityRefTypeMismatch)),
        "Should have EntityRefTypeMismatch error, got: {:?}",
        errors
    );

    let ref_error = errors
        .iter()
        .find(|e| matches!(e.code, ErrorCode::EntityRefTypeMismatch))
        .unwrap();
    assert!(
        ref_error.message.contains("metric"),
        "Error should mention actual type 'metric': {}",
        ref_error.message
    );
    assert!(
        ref_error.message.contains("project"),
        "Error should mention expected type 'project': {}",
        ref_error.message
    );
}

#[test]
fn test_1c_entity_ref_correct_type_passes() {
    let registry = SchemaRegistry::new();
    let task_field_defs = registry
        .get_schema("task", 1)
        .expect("task v1 schema should exist");

    let canonical_fields = serde_json::json!({
        "assignee_id": "someone",
        "project_id": "project-entity"
    });

    // project-entity is type "project" which matches EntityRef("project")
    let errors = validate_entity_refs(
        &canonical_fields,
        &task_field_defs,
        &lookup_1c,
    );

    assert!(
        errors.is_empty(),
        "Entity ref pointing to correct type should pass: {:?}",
        errors
    );
}

// =============================================================================
// 1d. INVALID create_entity -- bad enum value
// =============================================================================

#[test]
fn test_1d_invalid_create_entity_bad_enum_value() {
    let registry = SchemaRegistry::new();

    let field_defs = registry
        .get_schema("metric", 1)
        .expect("metric v1 schema should exist");

    let canonical_fields = serde_json::json!({
        "trend": "sideways"
    });

    let errors = validate_canonical_fields(&canonical_fields, &field_defs);

    assert!(
        !errors.is_empty(),
        "Should reject invalid enum value 'sideways' for trend"
    );
    assert!(
        errors.iter().any(|e| matches!(e.code, ErrorCode::InvalidEnumValue)),
        "Should have InvalidEnumValue error, got: {:?}",
        errors
    );

    let enum_error = errors
        .iter()
        .find(|e| matches!(e.code, ErrorCode::InvalidEnumValue))
        .unwrap();
    assert!(
        enum_error.message.contains("sideways"),
        "Error should mention the invalid value: {}",
        enum_error.message
    );
    assert!(
        enum_error.message.contains("up"),
        "Error should list valid values: {}",
        enum_error.message
    );
    assert!(
        enum_error.message.contains("down"),
        "Error should list valid values: {}",
        enum_error.message
    );
    assert!(
        enum_error.message.contains("flat"),
        "Error should list valid values: {}",
        enum_error.message
    );
}

#[test]
fn test_1d_bad_enum_via_registry_validation() {
    let registry = SchemaRegistry::new();

    let errors = registry.validate_canonical_fields(
        "metric",
        1,
        &serde_json::json!({"trend": "sideways"}),
    );

    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(e.code, ErrorCode::InvalidEnumValue)));
}

#[test]
fn test_1d_valid_enum_values_pass() {
    let registry = SchemaRegistry::new();
    let field_defs = registry.get_schema("metric", 1).unwrap();

    for valid_trend in &["up", "down", "flat"] {
        let fields = serde_json::json!({"trend": valid_trend});
        let errors = validate_canonical_fields(&fields, &field_defs);
        assert!(
            errors.is_empty(),
            "Valid trend '{}' should pass validation: {:?}",
            valid_trend,
            errors
        );
    }
}

// =============================================================================
// 1e. VALID create_relation
// =============================================================================

#[test]
fn test_1e_valid_create_relation() {
    let conn = common::test_db();

    // Create metric "MRR"
    let create_metric = PatchSet {
        ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: "metric".to_string(),
            title: "MRR".to_string(),
            source: "template".to_string(),
            canonical_fields: serde_json::json!({"current_value": 200000}),
            body_md: None,
            status: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: "test-run".to_string(),
    };
    let metric_result = apply_patch_set(&conn, &create_metric).unwrap();
    let metric_id = metric_result.applied[0].entity_id.as_ref().unwrap().clone();

    // Create experiment "Pricing Test"
    let create_experiment = PatchSet {
        ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: "experiment".to_string(),
            title: "Pricing Test".to_string(),
            source: "template".to_string(),
            canonical_fields: serde_json::json!({"hypothesis": "Higher prices increase revenue", "primary_metric": "MRR"}),
            body_md: None,
            status: None,
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: "test-run".to_string(),
    };
    let experiment_result = apply_patch_set(&conn, &create_experiment).unwrap();
    let experiment_id = experiment_result.applied[0].entity_id.as_ref().unwrap().clone();

    // Create relation: experiment measures metric
    let create_relation = PatchSet {
        ops: vec![PatchOp::CreateRelation(CreateRelationPayload {
            from_id: experiment_id.clone(),
            to_id: metric_id.clone(),
            relation_type: "measures".to_string(),
            weight: None,
            confidence: None,
            provenance_run_id: None,
            reason: None,
        })],
        run_id: "test-run".to_string(),
    };
    let relation_result = apply_patch_set(&conn, &create_relation)
        .expect("create_relation should succeed");
    assert_eq!(relation_result.applied.len(), 1);
    assert!(relation_result.applied[0].relation_id.is_some());

    // Verify relation exists in DB
    let relations = StoreService::get_relations(&conn, &experiment_id)
        .expect("get_relations should succeed");
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].from_id, experiment_id);
    assert_eq!(relations[0].to_id, metric_id);
    assert_eq!(relations[0].relation_type, "measures");
}

#[test]
fn test_1e_valid_relation_passes_validation() {
    // Test the validation function with mock entities that both exist
    let errors = validate_relation_refs(
        "met-1g",
        "exp-1g",
        "measures",
        &[],
        &lookup_1g,
    );
    assert!(
        errors.is_empty(),
        "Valid relation should pass validation: {:?}",
        errors
    );
}

// =============================================================================
// 1f. INVALID create_relation -- nonexistent target
// =============================================================================

#[test]
fn test_1f_invalid_create_relation_nonexistent_target() {
    let errors = validate_relation_refs(
        "valid-metric",
        "nonexistent-uuid",
        "measures",
        &[],
        &lookup_1f,
    );

    assert!(
        !errors.is_empty(),
        "Should reject relation to nonexistent entity"
    );
    assert!(
        errors.iter().any(|e| matches!(e.code, ErrorCode::EntityNotFound)),
        "Should have EntityNotFound error, got: {:?}",
        errors
    );

    let not_found_error = errors
        .iter()
        .find(|e| matches!(e.code, ErrorCode::EntityNotFound))
        .unwrap();
    assert_eq!(
        not_found_error.field_path, "to_id",
        "Error should be on to_id field"
    );
}

#[test]
fn test_1f_nonexistent_from_also_rejected() {
    // Both endpoints nonexistent
    let errors = validate_relation_refs(
        "nonexistent-from",
        "nonexistent-to",
        "measures",
        &[],
        &lookup_1f,
    );

    assert_eq!(
        errors.len(),
        2,
        "Should have errors for both nonexistent endpoints: {:?}",
        errors
    );
    assert!(errors.iter().all(|e| matches!(e.code, ErrorCode::EntityNotFound)));
}

// =============================================================================
// 1g. INVALID create_relation -- unapproved custom type
// =============================================================================

#[test]
fn test_1g_invalid_create_relation_unapproved_custom_type() {
    let errors = validate_relation_refs(
        "met-1g",
        "exp-1g",
        "custom:correlates_with",
        &[], // no approved custom types
        &lookup_1g,
    );

    assert!(
        !errors.is_empty(),
        "Should reject unapproved custom relation type"
    );
    assert!(
        errors.iter().any(|e| matches!(e.code, ErrorCode::RelationTypeNotApproved)),
        "Should have RelationTypeNotApproved error, got: {:?}",
        errors
    );

    let type_error = errors
        .iter()
        .find(|e| matches!(e.code, ErrorCode::RelationTypeNotApproved))
        .unwrap();
    assert!(
        type_error.message.contains("custom:correlates_with"),
        "Error should mention the rejected type: {}",
        type_error.message
    );
}

#[test]
fn test_1g_approved_custom_type_passes() {
    let approved = vec!["custom:correlates_with".to_string()];
    let errors = validate_relation_refs(
        "met-1g",
        "exp-1g",
        "custom:correlates_with",
        &approved,
        &lookup_1g,
    );

    assert!(
        errors.is_empty(),
        "Approved custom type should pass validation: {:?}",
        errors
    );
}

#[test]
fn test_1g_builtin_types_not_checked_against_custom_list() {
    // Built-in relation types (not starting with "custom:") should pass
    // regardless of the approved custom types list
    let errors = validate_relation_refs(
        "met-1g",
        "exp-1g",
        "measures",
        &[], // empty custom types list
        &lookup_1g,
    );

    assert!(
        errors.is_empty(),
        "Built-in type 'measures' should not need approval: {:?}",
        errors
    );
}

// =============================================================================
// 1h. INVALID create_relation -- soft-deleted target
// =============================================================================

#[test]
fn test_1h_invalid_create_relation_soft_deleted_target() {
    let errors = validate_relation_refs(
        "met-1h",
        "exp-1h", // soft-deleted
        "measures",
        &[],
        &lookup_1h,
    );

    assert!(
        !errors.is_empty(),
        "Should reject relation to soft-deleted entity"
    );
    assert!(
        errors.iter().any(|e| matches!(e.code, ErrorCode::EntityDeleted)),
        "Should have EntityDeleted error, got: {:?}",
        errors
    );

    let deleted_error = errors
        .iter()
        .find(|e| matches!(e.code, ErrorCode::EntityDeleted))
        .unwrap();
    assert_eq!(
        deleted_error.field_path, "to_id",
        "Error should be on to_id field"
    );
    assert!(
        deleted_error.message.contains("exp-1h"),
        "Error should mention the deleted entity ID: {}",
        deleted_error.message
    );
}

#[test]
fn test_1h_soft_deleted_also_checked_in_db() {
    // Integration test: verify soft-deleted entities don't appear in entity lookups
    let conn = common::test_db();

    common::insert_test_metric(&conn, "met-1h-db", "Active Metric");
    common::insert_test_experiment(&conn, "exp-1h-db", "To Be Deleted");

    // Soft-delete the experiment
    common::soft_delete_entity(&conn, "exp-1h-db");

    // Verify the soft-deleted entity is not found via StoreService
    let result = StoreService::get_entity(&conn, "exp-1h-db");
    assert!(
        result.is_err(),
        "Soft-deleted entity should not be returned by get_entity"
    );
}

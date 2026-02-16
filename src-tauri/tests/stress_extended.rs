// Extended stress tests covering all 22 entity types (Waves 1C + 2C).
//
// Properties verified:
//   P1: No panics, no hangs, no unstructured errors on any valid input
//   P2: If Ok, database is consistent
//   P3: If Err, database is unchanged (atomic rollback)
//   P4: Optimistic locking always enforced
//   P7: CRUD lifecycle works for all 22 entity types
//   P8: Status state machine enforced for all types
//   P9: Required field enforcement (decision requires owner_id + rationale)
//   P10: Cross-type entity_ref validation
//   P11: Template prerequisite chains
//
// Fuzz budget: ~50 cases per type x 22 types = ~1100 random ops minimum.

mod common;

use gargoyle_lib::error::GargoyleError;
use gargoyle_lib::models::patch::*;
use gargoyle_lib::patch::apply::apply_patch_set;
use proptest::prelude::*;

// =============================================================================
// Helpers
// =============================================================================

fn count_all_entities(conn: &rusqlite::Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn get_entity_status(conn: &rusqlite::Connection, id: &str) -> Option<String> {
    conn.query_row(
        "SELECT status FROM entities WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get(0),
    )
    .ok()
}

fn get_entity_updated_at(conn: &rusqlite::Connection, id: &str) -> String {
    conn.query_row(
        "SELECT updated_at FROM entities WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get(0),
    )
    .expect("Failed to get updated_at")
}

fn verify_entity_exists(conn: &rusqlite::Connection, id: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![id],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

fn assert_structured_error(err: &GargoyleError) {
    match err {
        GargoyleError::Database(_) => {}
        GargoyleError::Validation(v) => {
            let _ = &v.field_path;
            let _ = &v.code;
        }
        GargoyleError::NotFound { .. } => {}
        GargoyleError::LockConflict { .. } => {}
        GargoyleError::Schema(_) => {}
        GargoyleError::Serialization(_) => {}
    }
}

/// Create an entity via the patch system and return its (id, updated_at) on success.
fn create_entity_via_patch(
    conn: &rusqlite::Connection,
    entity_type: &str,
    title: &str,
    source: &str,
    canonical_fields: serde_json::Value,
    status: Option<String>,
) -> Result<(String, String), GargoyleError> {
    let patch_set = PatchSet {
        ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: entity_type.to_string(),
            title: title.to_string(),
            source: source.to_string(),
            canonical_fields,
            body_md: None,
            status,
            category: None,
            priority: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(conn, &patch_set)?;
    let entity_id = result.applied[0]
        .entity_id
        .as_ref()
        .unwrap()
        .clone();
    let updated_at = get_entity_updated_at(conn, &entity_id);
    Ok((entity_id, updated_at))
}

// =============================================================================
// Test: CRUD lifecycle for all 22 entity types
// =============================================================================

#[test]
fn test_crud_lifecycle_all_22_types() {
    let conn = common::test_db();

    for entity_type in common::generators::ALL_ENTITY_TYPES {
        // Build valid canonical_fields for this type (deterministic, not proptest)
        let canonical_fields = match *entity_type {
            "decision" => serde_json::json!({"owner_id": "test-owner", "rationale": "test rationale"}),
            "metric" => serde_json::json!({"current_value": 42.0}),
            "experiment" => serde_json::json!({"hypothesis": "test"}),
            "result" => serde_json::json!({"findings": "test"}),
            "task" => serde_json::json!({"assignee": "Alice"}),
            "project" => serde_json::json!({"objective": "Ship v2"}),
            "person" => serde_json::json!({"role": "Engineer"}),
            "note" => serde_json::json!({"context": "planning"}),
            "session" => serde_json::json!({"session_type": "planning"}),
            "campaign" => serde_json::json!({"objective": "Growth"}),
            "audience" => serde_json::json!({"segment_criteria": "Enterprise"}),
            "competitor" => serde_json::json!({"website": "https://example.com"}),
            "channel" => serde_json::json!({"channel_type": "email"}),
            "spec" => serde_json::json!({"spec_type": "technical"}),
            "budget" => serde_json::json!({"total_amount": 10000.0}),
            "vendor" => serde_json::json!({"vendor_type": "agency"}),
            "playbook" => serde_json::json!({"playbook_type": "sales"}),
            "taxonomy" => serde_json::json!({"taxonomy_type": "category"}),
            "backlog" => serde_json::json!({"priority_score": 5.0}),
            "brief" => serde_json::json!({"brief_type": "creative"}),
            "event" => serde_json::json!({"event_type": "conference"}),
            "policy" => serde_json::json!({"policy_type": "security"}),
            _ => serde_json::json!({}),
        };

        let initial_status = common::generators::initial_status_for_type(entity_type);

        // CREATE
        let (entity_id, updated_at) = create_entity_via_patch(
            &conn,
            entity_type,
            &format!("Test {}", entity_type),
            "manual",
            canonical_fields.clone(),
            Some(initial_status.to_string()),
        )
        .unwrap_or_else(|e| panic!("Failed to create {}: {:?}", entity_type, e));

        // READ
        assert!(
            verify_entity_exists(&conn, &entity_id),
            "Entity {} should exist after create",
            entity_type
        );

        // UPDATE (forward status transition)
        let second_status = common::generators::second_status_for_type(entity_type);
        let update_patch = PatchSet {
            ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                entity_id: entity_id.clone(),
                expected_updated_at: updated_at.clone(),
                title: Some(format!("Updated {}", entity_type)),
                body_md: None,
                status: Some(second_status.to_string()),
                canonical_fields: Some(canonical_fields.clone()),
                category: None,
                priority: None,
                reason: None,
            })],
            run_id: None,
        };

        let update_result = apply_patch_set(&conn, &update_patch)
            .unwrap_or_else(|e| panic!("Failed to update {}: {:?}", entity_type, e));
        assert_eq!(update_result.applied.len(), 1);

        // Verify status changed
        let current_status = get_entity_status(&conn, &entity_id);
        assert_eq!(
            current_status,
            Some(second_status.to_string()),
            "Status should be updated for {}",
            entity_type
        );

        // SOFT DELETE
        let _new_updated_at = get_entity_updated_at(&conn, &entity_id);
        common::soft_delete_entity(&conn, &entity_id);
        assert!(
            !verify_entity_exists(&conn, &entity_id),
            "Entity {} should not be visible after soft delete",
            entity_type
        );

        // Verify soft-deleted entity is still in DB (just has deleted_at set)
        let total_including_deleted: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            total_including_deleted, 1,
            "Soft-deleted {} entity should still exist in DB",
            entity_type
        );
    }
}

// =============================================================================
// Test: Status state machine -- forward transitions without reason
// =============================================================================

#[test]
fn test_status_forward_transitions_all_types() {
    use gargoyle_lib::validation::status_validator::validate_status_transition;

    for entity_type in common::generators::ALL_ENTITY_TYPES {
        let statuses: Vec<&str> = match *entity_type {
            "metric" => vec!["active", "paused", "deprecated", "archived"],
            "experiment" => vec!["draft", "running", "concluded", "archived"],
            "result" => vec!["draft", "final", "archived"],
            "task" => vec!["backlog", "todo", "in_progress", "blocked", "done", "archived"],
            "project" => vec!["planning", "active", "paused", "completed", "archived"],
            "decision" => vec!["proposed", "accepted", "deprecated", "superseded"],
            "person" => vec!["active", "inactive", "archived"],
            "note" => vec!["draft", "final", "archived"],
            "session" => vec!["scheduled", "in_progress", "completed", "cancelled"],
            "campaign" => vec!["planning", "active", "paused", "completed", "archived"],
            "audience" => vec!["draft", "validated", "active", "archived"],
            "competitor" => vec!["tracking", "dormant", "archived"],
            "channel" => vec!["evaluating", "active", "scaling", "paused", "deprecated"],
            "spec" => vec!["draft", "review", "approved", "deprecated"],
            "budget" => vec!["draft", "approved", "active", "closed"],
            "vendor" => vec!["evaluating", "active", "on_hold", "terminated"],
            "playbook" => vec!["draft", "active", "deprecated", "archived"],
            "taxonomy" => vec!["draft", "active", "archived"],
            "backlog" => vec!["open", "triaged", "scheduled", "closed"],
            "brief" => vec!["draft", "review", "approved", "archived"],
            "event" => vec!["proposed", "confirmed", "in_progress", "completed", "cancelled"],
            "policy" => vec!["draft", "active", "under_review", "deprecated"],
            _ => vec![],
        };

        // Forward transitions should always pass without reason
        for i in 0..statuses.len().saturating_sub(1) {
            let errors = validate_status_transition(
                entity_type,
                Some(statuses[i]),
                statuses[i + 1],
                None,
            );
            assert!(
                errors.is_empty(),
                "Forward transition {} -> {} for {} should succeed without reason, got: {:?}",
                statuses[i],
                statuses[i + 1],
                entity_type,
                errors
            );
        }
    }
}

// =============================================================================
// Test: Status state machine -- backward transitions require reason
// =============================================================================

#[test]
fn test_status_backward_transitions_require_reason() {
    use gargoyle_lib::validation::status_validator::validate_status_transition;

    for entity_type in common::generators::ALL_ENTITY_TYPES {
        let statuses: Vec<&str> = match *entity_type {
            "metric" => vec!["active", "paused", "deprecated", "archived"],
            "experiment" => vec!["draft", "running", "concluded", "archived"],
            "result" => vec!["draft", "final", "archived"],
            "task" => vec!["backlog", "todo", "in_progress", "blocked", "done", "archived"],
            "project" => vec!["planning", "active", "paused", "completed", "archived"],
            "decision" => vec!["proposed", "accepted", "deprecated", "superseded"],
            "person" => vec!["active", "inactive", "archived"],
            "note" => vec!["draft", "final", "archived"],
            "session" => vec!["scheduled", "in_progress", "completed", "cancelled"],
            "campaign" => vec!["planning", "active", "paused", "completed", "archived"],
            "audience" => vec!["draft", "validated", "active", "archived"],
            "competitor" => vec!["tracking", "dormant", "archived"],
            "channel" => vec!["evaluating", "active", "scaling", "paused", "deprecated"],
            "spec" => vec!["draft", "review", "approved", "deprecated"],
            "budget" => vec!["draft", "approved", "active", "closed"],
            "vendor" => vec!["evaluating", "active", "on_hold", "terminated"],
            "playbook" => vec!["draft", "active", "deprecated", "archived"],
            "taxonomy" => vec!["draft", "active", "archived"],
            "backlog" => vec!["open", "triaged", "scheduled", "closed"],
            "brief" => vec!["draft", "review", "approved", "archived"],
            "event" => vec!["proposed", "confirmed", "in_progress", "completed", "cancelled"],
            "policy" => vec!["draft", "active", "under_review", "deprecated"],
            _ => vec![],
        };

        if statuses.len() < 2 {
            continue;
        }

        // Backward transition without reason should fail
        let errors = validate_status_transition(
            entity_type,
            Some(statuses[statuses.len() - 1]),
            statuses[0],
            None,
        );
        assert!(
            !errors.is_empty(),
            "Backward transition {} -> {} for {} should require a reason",
            statuses[statuses.len() - 1],
            statuses[0],
            entity_type
        );

        // Backward transition with reason should succeed
        let errors_with_reason = validate_status_transition(
            entity_type,
            Some(statuses[statuses.len() - 1]),
            statuses[0],
            Some("Reverting for re-evaluation"),
        );
        assert!(
            errors_with_reason.is_empty(),
            "Backward transition {} -> {} for {} should succeed with reason, got: {:?}",
            statuses[statuses.len() - 1],
            statuses[0],
            entity_type,
            errors_with_reason
        );
    }
}

// =============================================================================
// Test: Required field enforcement -- decision requires owner_id + rationale
// =============================================================================

#[test]
fn test_decision_requires_owner_id_and_rationale() {
    use gargoyle_lib::schema::registry::SchemaRegistry;

    let reg = SchemaRegistry::new();

    // Missing both required fields
    let errors = reg.validate_canonical_fields("decision", 1, &serde_json::json!({}));
    assert!(
        errors.len() >= 2,
        "Decision with no fields should have at least 2 errors (missing owner_id + rationale), got: {:?}",
        errors
    );

    // Missing rationale only
    let errors = reg.validate_canonical_fields(
        "decision",
        1,
        &serde_json::json!({"owner_id": "test-owner"}),
    );
    assert!(
        errors.len() >= 1,
        "Decision missing rationale should have at least 1 error: {:?}",
        errors
    );

    // Missing owner_id only
    let errors = reg.validate_canonical_fields(
        "decision",
        1,
        &serde_json::json!({"rationale": "test rationale"}),
    );
    assert!(
        errors.len() >= 1,
        "Decision missing owner_id should have at least 1 error: {:?}",
        errors
    );

    // Both present -- should pass
    let errors = reg.validate_canonical_fields(
        "decision",
        1,
        &serde_json::json!({"owner_id": "test-owner", "rationale": "test rationale"}),
    );
    assert!(
        errors.is_empty(),
        "Decision with both required fields should pass: {:?}",
        errors
    );
}

// =============================================================================
// Test: Cross-type entity_ref validation (schema-level type check)
// =============================================================================

#[test]
fn test_cross_type_entity_ref_schema_validation() {
    use gargoyle_lib::schema::registry::SchemaRegistry;

    let reg = SchemaRegistry::new();

    // task.project_id must be a string (EntityRef to project)
    let errors = reg.validate_canonical_fields(
        "task",
        1,
        &serde_json::json!({"project_id": 12345}), // wrong type: number
    );
    assert!(
        !errors.is_empty(),
        "task.project_id with number value should fail schema validation"
    );

    // task.project_id with string value should pass schema validation
    let errors = reg.validate_canonical_fields(
        "task",
        1,
        &serde_json::json!({"project_id": "some-project-uuid"}),
    );
    assert!(
        errors.is_empty(),
        "task.project_id with string value should pass: {:?}",
        errors
    );

    // campaign.target_audience_id must be a string (EntityRef to audience)
    let errors = reg.validate_canonical_fields(
        "campaign",
        1,
        &serde_json::json!({"target_audience_id": true}), // wrong type: boolean
    );
    assert!(
        !errors.is_empty(),
        "campaign.target_audience_id with boolean value should fail"
    );

    // channel.primary_metric_id must be a string (EntityRef to metric)
    let errors = reg.validate_canonical_fields(
        "channel",
        1,
        &serde_json::json!({"primary_metric_id": 42}), // wrong type
    );
    assert!(
        !errors.is_empty(),
        "channel.primary_metric_id with number should fail"
    );

    // taxonomy.parent_id must be a string (EntityRef to taxonomy)
    let errors = reg.validate_canonical_fields(
        "taxonomy",
        1,
        &serde_json::json!({"parent_id": ["array"]}), // wrong type
    );
    assert!(
        !errors.is_empty(),
        "taxonomy.parent_id with array should fail"
    );

    // audience.icp_id must be a string (EntityRef to person)
    let errors = reg.validate_canonical_fields(
        "audience",
        1,
        &serde_json::json!({"icp_id": "valid-string-ref"}),
    );
    assert!(
        errors.is_empty(),
        "audience.icp_id with valid string should pass: {:?}",
        errors
    );

    // note.linked_entity_id (wildcard EntityRef to *)
    let errors = reg.validate_canonical_fields(
        "note",
        1,
        &serde_json::json!({"linked_entity_id": "any-entity-id"}),
    );
    assert!(
        errors.is_empty(),
        "note.linked_entity_id with string should pass: {:?}",
        errors
    );

    // experiment.source_experiment_id (EntityRef to experiment)
    let errors = reg.validate_canonical_fields(
        "experiment",
        1,
        &serde_json::json!({"source_experiment_id": false}), // wrong type
    );
    assert!(
        !errors.is_empty(),
        "experiment.source_experiment_id with boolean should fail"
    );
}

// =============================================================================
// Test: Cross-type referential integrity via patch system
// =============================================================================

#[test]
fn test_cross_type_entity_ref_create_via_patch() {
    let conn = common::test_db();

    // Create a project first
    let (project_id, _) = create_entity_via_patch(
        &conn,
        "project",
        "Test Project",
        "manual",
        serde_json::json!({"objective": "Ship v2"}),
        Some("planning".to_string()),
    )
    .expect("Failed to create project");

    // Create a task that references the project
    let task_result = create_entity_via_patch(
        &conn,
        "task",
        "Task referencing project",
        "manual",
        serde_json::json!({"project_id": project_id}),
        Some("backlog".to_string()),
    );
    // Should succeed (project_id is a valid entity ref at schema level)
    assert!(
        task_result.is_ok(),
        "Task with valid project_id ref should succeed: {:?}",
        task_result.err()
    );

    // Create a person first
    let (person_id, _) = create_entity_via_patch(
        &conn,
        "person",
        "Test Person",
        "manual",
        serde_json::json!({"role": "Engineer"}),
        Some("active".to_string()),
    )
    .expect("Failed to create person");

    // Create an audience that references the person as ICP
    let audience_result = create_entity_via_patch(
        &conn,
        "audience",
        "Enterprise ICP Audience",
        "manual",
        serde_json::json!({"icp_id": person_id, "segment_criteria": "Enterprise"}),
        Some("draft".to_string()),
    );
    assert!(
        audience_result.is_ok(),
        "Audience with valid icp_id ref should succeed: {:?}",
        audience_result.err()
    );

    // Create a metric first
    let (metric_id, _) = create_entity_via_patch(
        &conn,
        "metric",
        "Test Metric",
        "manual",
        serde_json::json!({"current_value": 42.0}),
        Some("active".to_string()),
    )
    .expect("Failed to create metric");

    // Create a channel that references the metric
    let channel_result = create_entity_via_patch(
        &conn,
        "channel",
        "Email Channel",
        "manual",
        serde_json::json!({"channel_type": "email", "primary_metric_id": metric_id}),
        Some("evaluating".to_string()),
    );
    assert!(
        channel_result.is_ok(),
        "Channel with valid primary_metric_id ref should succeed: {:?}",
        channel_result.err()
    );
}

// =============================================================================
// Test: Template prerequisite chain verification
// =============================================================================

#[test]
fn test_template_prerequisite_chain() {
    use gargoyle_lib::services::template_runner::{check_prerequisites, get_template_definition};

    let conn = common::test_db();

    // analytics-experiment-plan requires metric >= 1
    let def = get_template_definition("analytics-experiment-plan").unwrap();
    assert!(
        !def.prerequisites.is_empty(),
        "analytics-experiment-plan should have prerequisites"
    );
    assert_eq!(def.prerequisites[0].entity_type, "metric");
    assert_eq!(def.prerequisites[0].min_count, 1);

    // Before creating any metrics, prerequisites should not be satisfied
    let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
    let unsatisfied: Vec<_> = results.iter().filter(|r| !r.satisfied).collect();
    assert!(
        !unsatisfied.is_empty(),
        "analytics-experiment-plan should have unsatisfied prerequisites with no metrics"
    );

    // Create a metric
    create_entity_via_patch(
        &conn,
        "metric",
        "MRR",
        "manual",
        serde_json::json!({"current_value": 100.0}),
        Some("active".to_string()),
    )
    .expect("Failed to create metric");

    // Now prerequisites should be satisfied
    let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
    let unsatisfied: Vec<_> = results.iter().filter(|r| !r.satisfied).collect();
    assert!(
        unsatisfied.is_empty(),
        "analytics-experiment-plan should have all prerequisites satisfied after creating a metric"
    );

    // analytics-anomaly-investigation requires experiment >= 1
    let def = get_template_definition("analytics-anomaly-investigation").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "experiment");

    let results = check_prerequisites(&conn, "analytics-anomaly-investigation").unwrap();
    let unsatisfied: Vec<_> = results.iter().filter(|r| !r.satisfied).collect();
    assert!(
        !unsatisfied.is_empty(),
        "analytics-anomaly-investigation should need experiments"
    );

    // Create an experiment
    create_entity_via_patch(
        &conn,
        "experiment",
        "Test Experiment",
        "manual",
        serde_json::json!({"hypothesis": "testing"}),
        Some("draft".to_string()),
    )
    .expect("Failed to create experiment");

    let results = check_prerequisites(&conn, "analytics-anomaly-investigation").unwrap();
    let unsatisfied: Vec<_> = results.iter().filter(|r| !r.satisfied).collect();
    assert!(
        unsatisfied.is_empty(),
        "analytics-anomaly-investigation should be satisfied after creating experiment"
    );

    // mkt-positioning-narrative requires person >= 1
    let def = get_template_definition("mkt-positioning-narrative").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "person");

    let results = check_prerequisites(&conn, "mkt-positioning-narrative").unwrap();
    let unsatisfied: Vec<_> = results.iter().filter(|r| !r.satisfied).collect();
    assert!(
        !unsatisfied.is_empty(),
        "mkt-positioning-narrative should need a person"
    );

    create_entity_via_patch(
        &conn,
        "person",
        "Alice",
        "manual",
        serde_json::json!({"role": "PM"}),
        Some("active".to_string()),
    )
    .expect("Failed to create person");

    let results = check_prerequisites(&conn, "mkt-positioning-narrative").unwrap();
    let unsatisfied: Vec<_> = results.iter().filter(|r| !r.satisfied).collect();
    assert!(
        unsatisfied.is_empty(),
        "mkt-positioning-narrative should be satisfied after creating person"
    );
}

// =============================================================================
// Test: Template prerequisite dependencies for Wave 2B templates
// =============================================================================

#[test]
fn test_template_prerequisite_wave2b_dependencies() {
    use gargoyle_lib::services::template_runner::{check_prerequisites, get_template_definition};

    let conn = common::test_db();

    // Verify several Wave 2B template prerequisites

    // mkt-email-nurture-sequence requires audience >= 1
    let def = get_template_definition("mkt-email-nurture-sequence").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "audience");

    // mkt-launch-content-pack requires campaign >= 1
    let def = get_template_definition("mkt-launch-content-pack").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "campaign");

    // mkt-paid-ads-plan requires budget >= 1
    let def = get_template_definition("mkt-paid-ads-plan").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "budget");

    // mkt-social-distribution-plan requires channel >= 1
    let def = get_template_definition("mkt-social-distribution-plan").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "channel");

    // strategy-messaging-architecture requires decision >= 1
    let def = get_template_definition("strategy-messaging-architecture").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "decision");

    // mkt-editorial-calendar requires note >= 1
    let def = get_template_definition("mkt-editorial-calendar").unwrap();
    assert_eq!(def.prerequisites[0].entity_type, "note");

    // Create dependencies in order and verify prerequisites
    // 1. Create budget -> mkt-paid-ads-plan satisfied
    create_entity_via_patch(
        &conn, "budget", "Q1 Budget", "manual",
        serde_json::json!({"total_amount": 50000.0, "currency": "USD"}),
        Some("draft".to_string()),
    ).unwrap();

    let results = check_prerequisites(&conn, "mkt-paid-ads-plan").unwrap();
    assert!(results.iter().all(|r| r.satisfied), "mkt-paid-ads-plan should be satisfied");

    // 2. Create audience -> mkt-email-nurture-sequence satisfied
    create_entity_via_patch(
        &conn, "audience", "Enterprise Buyers", "manual",
        serde_json::json!({"segment_criteria": "Enterprise SaaS"}),
        Some("draft".to_string()),
    ).unwrap();

    let results = check_prerequisites(&conn, "mkt-email-nurture-sequence").unwrap();
    assert!(results.iter().all(|r| r.satisfied), "mkt-email-nurture-sequence should be satisfied");

    // 3. Create campaign -> mkt-launch-content-pack satisfied
    create_entity_via_patch(
        &conn, "campaign", "Spring Launch", "manual",
        serde_json::json!({"objective": "Launch new product"}),
        Some("planning".to_string()),
    ).unwrap();

    let results = check_prerequisites(&conn, "mkt-launch-content-pack").unwrap();
    assert!(results.iter().all(|r| r.satisfied), "mkt-launch-content-pack should be satisfied");

    // 4. Create channel -> mkt-social-distribution-plan satisfied
    create_entity_via_patch(
        &conn, "channel", "Email Channel", "manual",
        serde_json::json!({"channel_type": "email"}),
        Some("evaluating".to_string()),
    ).unwrap();

    let results = check_prerequisites(&conn, "mkt-social-distribution-plan").unwrap();
    assert!(results.iter().all(|r| r.satisfied), "mkt-social-distribution-plan should be satisfied");

    // 5. Create decision -> strategy-messaging-architecture satisfied
    create_entity_via_patch(
        &conn, "decision", "Pricing Decision", "manual",
        serde_json::json!({"owner_id": "ceo", "rationale": "Market positioning"}),
        Some("proposed".to_string()),
    ).unwrap();

    let results = check_prerequisites(&conn, "strategy-messaging-architecture").unwrap();
    assert!(results.iter().all(|r| r.satisfied), "strategy-messaging-architecture should be satisfied");

    // 6. Create note -> mkt-editorial-calendar satisfied
    create_entity_via_patch(
        &conn, "note", "Content Ideas", "manual",
        serde_json::json!({"context": "editorial planning"}),
        Some("draft".to_string()),
    ).unwrap();

    let results = check_prerequisites(&conn, "mkt-editorial-calendar").unwrap();
    assert!(results.iter().all(|r| r.satisfied), "mkt-editorial-calendar should be satisfied");
}

// =============================================================================
// Proptest: Fuzz create for all 22 types (~50 cases each = ~1100 total)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn fuzz_create_metric_extended(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("metric"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "metric".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => {
                assert_eq!(pr.applied.len(), 1);
                assert_eq!(count_all_entities(&conn), before + 1);
            }
            Err(e) => {
                assert_structured_error(&e);
                assert_eq!(count_all_entities(&conn), before);
            }
        }
    }

    #[test]
    fn fuzz_create_experiment_extended(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("experiment"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "experiment".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_result_extended(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("result"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "result".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_task(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("task"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "task".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_project(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("project"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "project".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_decision(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("decision"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "decision".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_person(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("person"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "person".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_note(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("note"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "note".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_session(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("session"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "session".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_campaign(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("campaign"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "campaign".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_audience(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("audience"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "audience".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_competitor(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("competitor"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "competitor".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_channel(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("channel"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "channel".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_spec(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("spec"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "spec".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_budget(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("budget"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "budget".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_vendor(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("vendor"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "vendor".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_playbook(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("playbook"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "playbook".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_taxonomy(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("taxonomy"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "taxonomy".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_backlog(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("backlog"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "backlog".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_brief(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("brief"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "brief".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_event(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("event"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "event".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }

    #[test]
    fn fuzz_create_policy(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("policy"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);
        let result = apply_patch_set(&conn, &PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "policy".to_string(), title, source, canonical_fields,
                body_md: None, status, category: None, priority: None,
            })], run_id: None,
        });
        match result {
            Ok(pr) => { assert_eq!(pr.applied.len(), 1); assert_eq!(count_all_entities(&conn), before + 1); }
            Err(e) => { assert_structured_error(&e); assert_eq!(count_all_entities(&conn), before); }
        }
    }
}

// =============================================================================
// Proptest: Fuzz update for all 22 types with status transitions (~50 cases each)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn fuzz_update_all_types_with_status_transitions(
        type_idx in 0usize..22,
        new_title in common::generators::gen_title(),
        use_stale_timestamp in proptest::bool::ANY,
    ) {
        let entity_type = common::generators::ALL_ENTITY_TYPES[type_idx];
        let conn = common::test_db();

        // Build valid canonical_fields for the type
        let canonical_fields = match entity_type {
            "decision" => serde_json::json!({"owner_id": "owner", "rationale": "reason"}),
            "metric" => serde_json::json!({"current_value": 10.0}),
            "experiment" => serde_json::json!({"hypothesis": "test"}),
            "result" => serde_json::json!({"findings": "test"}),
            "person" => serde_json::json!({"role": "PM"}),
            "session" => serde_json::json!({"session_type": "planning"}),
            "campaign" => serde_json::json!({"channel": "email"}),
            "channel" => serde_json::json!({"channel_type": "email"}),
            "spec" => serde_json::json!({"spec_type": "technical"}),
            "vendor" => serde_json::json!({"vendor_type": "agency"}),
            "playbook" => serde_json::json!({"playbook_type": "sales"}),
            "taxonomy" => serde_json::json!({"taxonomy_type": "category"}),
            "brief" => serde_json::json!({"brief_type": "creative"}),
            "event" => serde_json::json!({"event_type": "conference"}),
            "policy" => serde_json::json!({"policy_type": "security"}),
            _ => serde_json::json!({}),
        };

        let initial_status = common::generators::initial_status_for_type(entity_type);

        // Create entity
        let create_result = create_entity_via_patch(
            &conn,
            entity_type,
            &format!("Test {}", entity_type),
            "manual",
            canonical_fields.clone(),
            Some(initial_status.to_string()),
        );

        // Skip if create fails (e.g., decision without required fields -- covered by deterministic test)
        if let Ok((entity_id, actual_updated_at)) = create_result {
            let expected_updated_at = if use_stale_timestamp {
                "1970-01-01T00:00:00.000Z".to_string()
            } else {
                actual_updated_at.clone()
            };

            let second_status = common::generators::second_status_for_type(entity_type);

            let update_patch = PatchSet {
                ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                    entity_id: entity_id.clone(),
                    expected_updated_at: expected_updated_at.clone(),
                    title: Some(new_title),
                    body_md: None,
                    status: Some(second_status.to_string()),
                    canonical_fields: Some(canonical_fields),
                    category: None,
                    priority: None,
                    reason: None,
                })],
                run_id: None,
            };

            let result = apply_patch_set(&conn, &update_patch);

            match result {
                Ok(pr) => {
                    // If it succeeded, timestamp must have matched
                    assert!(!use_stale_timestamp,
                        "Update with stale timestamp should not succeed for type {}",
                        entity_type);
                    assert_eq!(pr.applied.len(), 1);
                }
                Err(e) => {
                    assert_structured_error(&e);
                    if use_stale_timestamp {
                        assert!(
                            matches!(e, GargoyleError::LockConflict { .. }),
                            "Stale timestamp should produce LockConflict for {}, got: {:?}",
                            entity_type, e
                        );
                    }
                }
            }
        }
    }
}

// =============================================================================
// Proptest: Schema validation for all 22 types (~50 cases each)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn fuzz_schema_validation_all_types(
        type_idx in 0usize..22,
        canonical_fields in common::generators::gen_entity_type().prop_flat_map(|t| {
            common::generators::gen_canonical_fields(&t)
        }),
    ) {
        use gargoyle_lib::schema::registry::SchemaRegistry;

        let entity_type = common::generators::ALL_ENTITY_TYPES[type_idx];
        let reg = SchemaRegistry::new();

        // This should never panic
        let errors = reg.validate_canonical_fields(entity_type, 1, &canonical_fields);

        // Every error should have a non-empty field_path (P6)
        for err in &errors {
            assert!(
                !err.field_path.is_empty(),
                "Validation error for {} should have non-empty field_path: {:?}",
                entity_type, err
            );
        }
    }
}

// =============================================================================
// Proptest: Status validation for all 22 types (~50 cases each)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn fuzz_status_validation_all_types(
        type_idx in 0usize..22,
        status in common::generators::gen_status(),
    ) {
        use gargoyle_lib::validation::status_validator::validate_status_transition;

        let entity_type = common::generators::ALL_ENTITY_TYPES[type_idx];

        if let Some(ref s) = status {
            // null -> new status should never panic
            let errors = validate_status_transition(entity_type, None, s, None);

            for err in &errors {
                assert!(
                    !err.field_path.is_empty(),
                    "Status validation error for {} should have non-empty field_path: {:?}",
                    entity_type, err
                );
            }
        }
    }
}

// =============================================================================
// Test: Verify 0 panics on valid input for all types (deterministic)
// =============================================================================

#[test]
fn test_zero_panics_valid_input_all_types() {
    let conn = common::test_db();

    for entity_type in common::generators::ALL_ENTITY_TYPES {
        let canonical_fields = match *entity_type {
            "decision" => serde_json::json!({"owner_id": "owner", "rationale": "reason"}),
            "metric" => serde_json::json!({"current_value": 42.0, "trend": "up"}),
            "experiment" => serde_json::json!({"hypothesis": "test"}),
            "result" => serde_json::json!({"findings": "test", "confidence_level": 0.95}),
            "task" => serde_json::json!({"assignee": "Alice", "effort_estimate": "M"}),
            "project" => serde_json::json!({"objective": "Ship", "timeline": "Q1"}),
            "person" => serde_json::json!({"email": "a@b.com", "external": false}),
            "note" => serde_json::json!({"context": "meeting", "tags": "a,b"}),
            "session" => serde_json::json!({"session_type": "standup", "agenda": "daily"}),
            "campaign" => serde_json::json!({"channel": "email", "budget": 5000.0}),
            "audience" => serde_json::json!({"segment_criteria": "Enterprise", "estimated_size": 10000.0}),
            "competitor" => serde_json::json!({"website": "https://x.com", "positioning": "leader"}),
            "channel" => serde_json::json!({"channel_type": "social", "budget_allocation": 1000.0}),
            "spec" => serde_json::json!({"spec_type": "product", "version": "1.0"}),
            "budget" => serde_json::json!({"total_amount": 50000.0, "currency": "USD"}),
            "vendor" => serde_json::json!({"vendor_type": "saas", "contract_value": 12000.0}),
            "playbook" => serde_json::json!({"playbook_type": "marketing", "owner": "team"}),
            "taxonomy" => serde_json::json!({"taxonomy_type": "tag", "level": 1.0}),
            "backlog" => serde_json::json!({"priority_score": 7.0, "effort": "L"}),
            "brief" => serde_json::json!({"brief_type": "campaign", "deadline": "2025-03-01"}),
            "event" => serde_json::json!({"event_type": "webinar", "expected_attendees": 500.0}),
            "policy" => serde_json::json!({"policy_type": "compliance", "owner": "legal"}),
            _ => serde_json::json!({}),
        };

        let initial_status = common::generators::initial_status_for_type(entity_type);

        // This should never panic
        let result = create_entity_via_patch(
            &conn,
            entity_type,
            &format!("Valid {}", entity_type),
            "manual",
            canonical_fields,
            Some(initial_status.to_string()),
        );

        assert!(
            result.is_ok(),
            "Creating valid {} entity should succeed: {:?}",
            entity_type,
            result.err()
        );
    }
}

// =============================================================================
// Test: Foundational templates have no prerequisites
// =============================================================================

#[test]
fn test_foundational_templates_no_prerequisites() {
    use gargoyle_lib::services::template_runner::get_template_definition;

    let foundational = [
        "analytics-metric-tree",
        "mkt-icp-definition",
        "mkt-competitive-intel",
        "analytics-measurement-framework-kpi-tree",
        "analytics-attribution-plan-utm-governance",
        "strategy-ICP-JTBD",
        "strategy-competitive-intelligence",
    ];

    for key in &foundational {
        let def = get_template_definition(key)
            .unwrap_or_else(|| panic!("Template {} should exist", key));
        assert!(
            def.prerequisites.is_empty(),
            "Foundational template {} should have no prerequisites, but has: {:?}",
            key, def.prerequisites
        );
    }
}

// =============================================================================
// Test: All template definitions are accessible
// =============================================================================

#[test]
fn test_all_template_definitions_accessible() {
    use gargoyle_lib::services::template_runner::list_template_definitions;

    let templates = list_template_definitions();
    assert!(
        templates.len() >= 80,
        "Should have at least 80 template definitions, got {}",
        templates.len()
    );

    // Every template should have a non-empty key and version
    for t in &templates {
        assert!(!t.key.is_empty(), "Template key should not be empty");
        assert!(!t.version.is_empty(), "Template version should not be empty");
        assert!(!t.category.is_empty(), "Template category should not be empty");
    }
}

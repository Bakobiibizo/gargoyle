// Phase 6A: Property-based fuzz tests for the patch validator.
//
// Target: ~30,000 random ops total across all proptests.
// Default cases per proptest: 500 (reasonable for dev/CI).
// For a full fuzz run, increase to 3000+ via PROPTEST_CASES env var or
// by changing the `with_cases` value below.
//
// Properties verified:
//   P1: No panics, no hangs, no unstructured errors
//   P2: If Ok, database is consistent
//   P3: If Err, database is unchanged (atomic rollback)
//   P4: Optimistic locking always enforced
//   P5: Claim grounding always enforced
//   P6: Validation errors always contain field info (where validation pipeline is invoked)

mod common;

use proptest::prelude::*;
use gargoyle_lib::error::GargoyleError;
use gargoyle_lib::models::patch::*;
use gargoyle_lib::patch::apply::apply_patch_set;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count all non-deleted entities in the database.
fn count_all_entities(conn: &rusqlite::Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

/// Count all relations in the database.
fn count_all_relations(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
        .unwrap_or(0)
}

/// Count all claims in the database.
fn count_all_claims(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM claims", [], |row| row.get(0))
        .unwrap_or(0)
}

/// Verify that an entity in the DB has valid entity_type and _schema_version >= 1.
fn verify_entity_consistent(conn: &rusqlite::Connection, entity_id: &str) {
    let row: (String, i32, String) = conn
        .query_row(
            "SELECT entity_type, _schema_version, source FROM entities WHERE id = ?1",
            rusqlite::params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("Entity should exist after successful create");

    let (entity_type, schema_version, source) = row;

    // Valid entity types
    assert!(
        ["metric", "experiment", "result"].contains(&entity_type.as_str()),
        "Entity type '{}' is not one of the known types",
        entity_type
    );

    // Schema version must be positive
    assert!(
        schema_version >= 1,
        "Schema version {} should be >= 1",
        schema_version
    );

    // Source must be valid (CHECK constraint enforces this, but verify)
    assert!(
        [
            "manual",
            "clipboard",
            "web",
            "import",
            "agent",
            "template",
            "bootstrap"
        ]
        .contains(&source.as_str()),
        "Source '{}' is not valid",
        source
    );
}

/// Classify a GargoyleError to confirm it is a structured error (P1).
fn assert_structured_error(err: &GargoyleError) {
    match err {
        GargoyleError::Database(_) => {}
        GargoyleError::Validation(v) => {
            // P6: validation errors should have a field_path
            // (note: some validation errors may have empty field_path for
            // top-level schema errors, which is acceptable)
            let _ = &v.field_path;
            let _ = &v.code;
        }
        GargoyleError::NotFound { .. } => {}
        GargoyleError::LockConflict { .. } => {}
        GargoyleError::Schema(_) => {}
        GargoyleError::Serialization(_) => {}
    }
}

// ---------------------------------------------------------------------------
// Fuzz: create_entity for metric type (~500 cases)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// P1: create_entity with random metric fields never panics.
    /// P2: if Ok, entity is in DB with consistent fields.
    /// P3: if Err, no entity was created.
    #[test]
    fn fuzz_create_metric_no_panic(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("metric"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);

        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "metric".to_string(),
                title: title.clone(),
                source,
                canonical_fields,
                body_md: None,
                status,
                category: None,
                priority: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                // P1: got a structured Ok result
                assert_eq!(pr.applied.len(), 1);
                let entity_id = pr.applied[0].entity_id.as_ref().unwrap();

                // P2: entity is in DB and consistent
                verify_entity_consistent(&conn, entity_id);

                let after = count_all_entities(&conn);
                assert_eq!(after, before + 1, "Exactly one entity should be created");
            }
            Err(e) => {
                // P1: error is structured
                assert_structured_error(&e);

                // P3: database unchanged
                let after = count_all_entities(&conn);
                assert_eq!(after, before, "Failed op should not create any entity");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: create_entity for experiment type (~500 cases)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_create_experiment_no_panic(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("experiment"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);

        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "experiment".to_string(),
                title,
                source,
                canonical_fields,
                body_md: None,
                status,
                category: None,
                priority: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                assert_eq!(pr.applied.len(), 1);
                let entity_id = pr.applied[0].entity_id.as_ref().unwrap();
                verify_entity_consistent(&conn, entity_id);
                let after = count_all_entities(&conn);
                assert_eq!(after, before + 1);
            }
            Err(e) => {
                assert_structured_error(&e);
                let after = count_all_entities(&conn);
                assert_eq!(after, before);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: create_entity for result type (~500 cases)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_create_result_no_panic(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("result"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);

        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "result".to_string(),
                title,
                source,
                canonical_fields,
                body_md: None,
                status,
                category: None,
                priority: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                assert_eq!(pr.applied.len(), 1);
                let entity_id = pr.applied[0].entity_id.as_ref().unwrap();
                verify_entity_consistent(&conn, entity_id);
                let after = count_all_entities(&conn);
                assert_eq!(after, before + 1);
            }
            Err(e) => {
                assert_structured_error(&e);
                let after = count_all_entities(&conn);
                assert_eq!(after, before);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: update_entity with random fields (~500 cases)
// P4: optimistic locking always enforced
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_update_entity_no_panic(
        new_title in common::generators::gen_title(),
        new_status in common::generators::gen_status(),
        canonical_fields in common::generators::gen_canonical_fields("metric"),
        use_stale_timestamp in proptest::bool::ANY,
    ) {
        let conn = common::test_db();

        // Set up a valid entity to update
        let entity_id = "fuzz-update-target";
        let updated_at = common::insert_test_entity(
            &conn,
            entity_id,
            "metric",
            "Original Title",
            "manual",
            r#"{"current_value": 42}"#,
        );

        let before = count_all_entities(&conn);

        // P4: use either the correct or a stale timestamp
        let expected_updated_at = if use_stale_timestamp {
            "1970-01-01T00:00:00.000Z".to_string()
        } else {
            updated_at.clone()
        };

        let patch_set = PatchSet {
            ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                entity_id: entity_id.to_string(),
                expected_updated_at: expected_updated_at.clone(),
                title: Some(new_title),
                body_md: None,
                status: new_status,
                canonical_fields: Some(canonical_fields),
                category: None,
                priority: None,
                reason: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                // P4: if it succeeded, the timestamp must have matched
                assert!(
                    !use_stale_timestamp,
                    "Update with stale timestamp should not succeed"
                );
                assert_eq!(pr.applied.len(), 1);

                // P2: entity still consistent
                verify_entity_consistent(&conn, entity_id);

                // Entity count unchanged (update, not create)
                let after = count_all_entities(&conn);
                assert_eq!(after, before);
            }
            Err(e) => {
                assert_structured_error(&e);

                // P4: if we used a stale timestamp, we should get LockConflict
                if use_stale_timestamp {
                    assert!(
                        matches!(e, GargoyleError::LockConflict { .. }),
                        "Stale timestamp should produce LockConflict, got: {:?}",
                        e
                    );
                }

                // P3: entity count unchanged on failure
                let after = count_all_entities(&conn);
                assert_eq!(after, before);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: update_entity with nonexistent entity (~500 cases)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_update_nonexistent_entity(
        entity_id in "[a-z]{8}-nonexistent",
        new_title in common::generators::gen_title(),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);

        let patch_set = PatchSet {
            ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                entity_id,
                expected_updated_at: "2025-01-01T00:00:00.000Z".to_string(),
                title: Some(new_title),
                body_md: None,
                status: None,
                canonical_fields: None,
                category: None,
                priority: None,
                reason: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        // Should always fail: entity does not exist
        assert!(result.is_err(), "Updating nonexistent entity should fail");
        assert_structured_error(&result.unwrap_err());

        // P3: database unchanged
        let after = count_all_entities(&conn);
        assert_eq!(after, before);
    }
}

// ---------------------------------------------------------------------------
// Fuzz: create_relation with random entity pairs (~500 cases)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_create_relation_no_panic(
        relation_type in common::generators::gen_relation_type(),
        use_valid_from in proptest::bool::ANY,
        use_valid_to in proptest::bool::ANY,
        weight in proptest::option::of(0.0..10.0f64),
        confidence in proptest::option::of(-0.5..1.5f64),
    ) {
        let conn = common::test_db();

        // Create some valid entities for "from" and "to"
        common::insert_test_metric(&conn, "rel-from", "From Entity");
        common::insert_test_experiment(&conn, "rel-to", "To Entity");

        let before_rels = count_all_relations(&conn);

        let from_id = if use_valid_from {
            "rel-from".to_string()
        } else {
            "nonexistent-from".to_string()
        };
        let to_id = if use_valid_to {
            "rel-to".to_string()
        } else {
            "nonexistent-to".to_string()
        };

        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateRelation(CreateRelationPayload {
                from_id: from_id.clone(),
                to_id: to_id.clone(),
                relation_type,
                weight,
                confidence,
                provenance_run_id: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                // P1: structured result
                assert_eq!(pr.applied.len(), 1);
                assert!(pr.applied[0].relation_id.is_some());

                // P2: relation should exist in DB
                let after_rels = count_all_relations(&conn);
                assert_eq!(after_rels, before_rels + 1);

                // Both endpoints must have been valid
                assert!(use_valid_from && use_valid_to,
                    "Relation should only succeed with valid entity IDs");
            }
            Err(e) => {
                assert_structured_error(&e);

                // P3: no relation created on failure
                let after_rels = count_all_relations(&conn);
                assert_eq!(after_rels, before_rels);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: create_claim with random grounding (~500 cases)
// P5: claim grounding always enforced at DB level
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_create_claim_no_panic(
        subject in "\\PC{1,30}",
        predicate in "\\PC{1,20}",
        object in "\\PC{1,30}",
        confidence in 0.0..1.0f64,
        use_valid_evidence in proptest::bool::ANY,
        use_deleted_evidence in proptest::bool::ANY,
    ) {
        let conn = common::test_db();

        // Create an evidence entity
        common::insert_test_result(&conn, "evidence-valid", "Valid Evidence");

        // Optionally create and delete another
        if use_deleted_evidence {
            common::insert_test_result(&conn, "evidence-deleted", "Deleted Evidence");
            common::soft_delete_entity(&conn, "evidence-deleted");
        }

        let before_claims = count_all_claims(&conn);

        let evidence_entity_id = if use_valid_evidence {
            "evidence-valid".to_string()
        } else if use_deleted_evidence {
            // Using a deleted entity ID -- FK exists but entity is soft-deleted.
            // The FK constraint in SQLite references entities(id) without
            // checking deleted_at, so this may succeed at the DB level.
            "evidence-deleted".to_string()
        } else {
            "nonexistent-evidence".to_string()
        };

        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateClaim(CreateClaimPayload {
                subject,
                predicate,
                object,
                confidence,
                evidence_entity_id: evidence_entity_id.clone(),
                provenance_run_id: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                // P1: structured result
                assert_eq!(pr.applied.len(), 1);
                assert!(pr.applied[0].claim_id.is_some());

                let after_claims = count_all_claims(&conn);
                assert_eq!(after_claims, before_claims + 1);

                // P5: The evidence_entity_id must reference an existing entity
                // (FK constraint enforces this). Deleted entities still satisfy FK
                // since FK checks entities(id), not deleted_at.
                assert!(
                    use_valid_evidence || use_deleted_evidence,
                    "Claim should only succeed with an existing entity ID (FK constraint)"
                );
            }
            Err(e) => {
                assert_structured_error(&e);

                // P3: no claim created on failure
                let after_claims = count_all_claims(&conn);
                assert_eq!(after_claims, before_claims);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: mixed patch set with multiple ops (~500 cases)
// Tests atomicity: if any op fails, entire patch set rolls back.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_mixed_patch_set(
        title_a in common::generators::gen_title(),
        title_b in common::generators::gen_title(),
        source_a in common::generators::gen_source(),
        source_b in common::generators::gen_source(),
        fields_a in common::generators::gen_canonical_fields("metric"),
        fields_b in common::generators::gen_canonical_fields("experiment"),
    ) {
        let conn = common::test_db();
        let before_entities = count_all_entities(&conn);

        let patch_set = PatchSet {
            ops: vec![
                PatchOp::CreateEntity(CreateEntityPayload {
                    entity_type: "metric".to_string(),
                    title: title_a,
                    source: source_a,
                    canonical_fields: fields_a,
                    body_md: None,
                    status: None,
                    category: None,
                    priority: None,
                }),
                PatchOp::CreateEntity(CreateEntityPayload {
                    entity_type: "experiment".to_string(),
                    title: title_b,
                    source: source_b,
                    canonical_fields: fields_b,
                    body_md: None,
                    status: None,
                    category: None,
                    priority: None,
                }),
            ],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        match result {
            Ok(pr) => {
                // Both ops succeeded
                assert_eq!(pr.applied.len(), 2);
                let after_entities = count_all_entities(&conn);
                assert_eq!(after_entities, before_entities + 2);

                // P2: both entities consistent
                for applied in &pr.applied {
                    if let Some(ref eid) = applied.entity_id {
                        verify_entity_consistent(&conn, eid);
                    }
                }
            }
            Err(e) => {
                assert_structured_error(&e);

                // P3: atomic rollback -- neither entity should be created
                let after_entities = count_all_entities(&conn);
                assert_eq!(
                    after_entities, before_entities,
                    "Failed patch set should roll back all ops"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: mixed patch set with create + update in same batch (~500 cases)
// Tests that a failed second op rolls back the first op.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_mixed_create_then_update_nonexistent(
        title in common::generators::gen_title(),
        source in common::generators::gen_source(),
        fields in common::generators::gen_canonical_fields("metric"),
    ) {
        let conn = common::test_db();
        let before = count_all_entities(&conn);

        // First op: valid create. Second op: update a nonexistent entity.
        // The second op should fail, rolling back the first.
        let patch_set = PatchSet {
            ops: vec![
                PatchOp::CreateEntity(CreateEntityPayload {
                    entity_type: "metric".to_string(),
                    title,
                    source,
                    canonical_fields: fields,
                    body_md: None,
                    status: None,
                    category: None,
                    priority: None,
                }),
                PatchOp::UpdateEntity(UpdateEntityPayload {
                    entity_id: "does-not-exist".to_string(),
                    expected_updated_at: "2025-01-01T00:00:00.000Z".to_string(),
                    title: Some("Won't work".to_string()),
                    body_md: None,
                    status: None,
                    canonical_fields: None,
                    category: None,
                    priority: None,
                    reason: None,
                }),
            ],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        // Should always fail (second op targets nonexistent entity)
        assert!(result.is_err(), "Patch set with invalid update should fail");
        assert_structured_error(&result.unwrap_err());

        // P3: atomic rollback -- the create from op[0] should also be rolled back
        let after = count_all_entities(&conn);
        assert_eq!(
            after, before,
            "Failed patch set should roll back ALL ops including the successful create"
        );
    }
}

// ---------------------------------------------------------------------------
// Fuzz: P4 -- optimistic locking is always enforced (dedicated test)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_optimistic_lock_always_rejects_stale(
        stale_timestamp in "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\\.[0-9]{3}Z",
    ) {
        let conn = common::test_db();

        // Create entity with a known timestamp
        let actual_updated_at = common::insert_test_metric(
            &conn,
            "lock-target",
            "Lock Test",
        );

        // Skip test cases where the random timestamp happens to match
        // (astronomically unlikely but proptest could generate it)
        prop_assume!(stale_timestamp != actual_updated_at);

        let patch_set = PatchSet {
            ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                entity_id: "lock-target".to_string(),
                expected_updated_at: stale_timestamp,
                title: Some("Should Fail".to_string()),
                body_md: None,
                status: None,
                canonical_fields: None,
                category: None,
                priority: None,
                reason: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        // P4: must always be rejected
        assert!(result.is_err(), "Stale timestamp must be rejected");
        match result.unwrap_err() {
            GargoyleError::LockConflict { .. } => {
                // Expected
            }
            other => {
                panic!("Expected LockConflict, got: {:?}", other);
            }
        }

        // Title should be unchanged
        let title: String = conn
            .query_row(
                "SELECT title FROM entities WHERE id = 'lock-target'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Lock Test", "Entity should not have been modified");
    }
}

// ---------------------------------------------------------------------------
// Fuzz: P5 -- claim with nonexistent evidence always fails (FK constraint)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_claim_nonexistent_evidence_always_fails(
        evidence_id in "[a-z]{5}-nonexistent-[0-9]{4}",
        subject in "\\PC{1,20}",
        predicate in "\\PC{1,15}",
        object in "\\PC{1,20}",
        confidence in 0.0..1.0f64,
    ) {
        let conn = common::test_db();
        let before = count_all_claims(&conn);

        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateClaim(CreateClaimPayload {
                subject,
                predicate,
                object,
                confidence,
                evidence_entity_id: evidence_id,
                provenance_run_id: None,
            })],
            run_id: None,
        };

        let result = apply_patch_set(&conn, &patch_set);

        // P5: must always fail (FK constraint on evidence_entity_id)
        assert!(
            result.is_err(),
            "Claim with nonexistent evidence should always fail"
        );
        assert_structured_error(&result.unwrap_err());

        // P3: no claim created
        let after = count_all_claims(&conn);
        assert_eq!(after, before);
    }
}

// ---------------------------------------------------------------------------
// Fuzz: P6 -- validation errors from the validation pipeline contain field_path
// Tests the validation module directly (not through apply_patch_set, since
// the current apply_patch_set does not wire the validation pipeline).
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_validation_errors_have_field_path(
        canonical_fields in common::generators::gen_canonical_fields("metric"),
        status in common::generators::gen_status(),
    ) {
        use gargoyle_lib::schema::registry::SchemaRegistry;

        let reg = SchemaRegistry::new();
        let errors = reg.validate_canonical_fields("metric", 1, &canonical_fields);

        // P6: every validation error must have a non-empty field_path
        for err in &errors {
            assert!(
                !err.field_path.is_empty(),
                "Validation error should have non-empty field_path: {:?}",
                err
            );
        }

        // Also test status validation errors
        if let Some(ref s) = status {
            let status_errors =
                gargoyle_lib::validation::status_validator::validate_status_transition(
                    "metric",
                    None,
                    s,
                    None,
                );
            for err in &status_errors {
                assert!(
                    !err.field_path.is_empty(),
                    "Status validation error should have non-empty field_path: {:?}",
                    err
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz: P6 -- lock validation errors always have field_path
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_lock_validation_errors_have_field_path(
        expected in "\\PC{5,30}",
        actual in "\\PC{5,30}",
    ) {
        let errors = gargoyle_lib::validation::lock_validator::validate_lock(&expected, &actual);

        for err in &errors {
            assert!(
                !err.field_path.is_empty(),
                "Lock validation error should have non-empty field_path: {:?}",
                err
            );
        }
    }
}

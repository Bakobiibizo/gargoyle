// Scenario 3: Schema Versioning and Migration
//
// Tests schema version tracking and migration:
//   3a-3b. Update bumps _schema_version (stays at v1 since current is v1)
//   3c-3d. Schema migration (v0 -> v1) via SchemaMigrator::migrate_all_entities
//   3e. Update after migration succeeds
//   3f. Stale entity (_schema_version = 0) blocks update with SchemaVersionMismatch

mod common;

use gargoyle_lib::error::{ErrorCode, GargoyleError};
use gargoyle_lib::models::patch::*;
use gargoyle_lib::patch::apply::apply_patch_set;
use gargoyle_lib::schema::version::SchemaMigrator;
use gargoyle_lib::services::store::StoreService;

// =============================================================================
// 3a-3b. Update keeps _schema_version at current (v1)
// =============================================================================

#[test]
fn test_3a_3b_update_maintains_schema_version() {
    let conn = common::test_db();

    // Create a v1 metric via patch system
    let create_set = PatchSet {
        ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: "metric".to_string(),
            title: "MRR".to_string(),
            source: "template".to_string(),
            canonical_fields: serde_json::json!({"current_value": 200000}),
            body_md: None,
            status: None,
            category: None,
            priority: None,
        })],
        run_id: None,
    };
    let create_result = apply_patch_set(&conn, &create_set).expect("create should succeed");
    let entity_id = create_result.applied[0]
        .entity_id
        .as_ref()
        .unwrap()
        .clone();

    // Verify: _schema_version = 1 on creation
    let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(
        entity.schema_version, 1,
        "Newly created entity should have _schema_version = 1"
    );
    let t1 = entity.updated_at.clone();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update the entity
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: entity_id.clone(),
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
    apply_patch_set(&conn, &update_set).expect("update should succeed");

    // Verify: _schema_version stays at 1 after update
    let updated_entity = StoreService::get_entity(&conn, &entity_id).unwrap();
    assert_eq!(
        updated_entity.schema_version, 1,
        "_schema_version should remain 1 after update (current version is 1)"
    );
    assert_eq!(updated_entity.title, "MRR Updated");
    assert_eq!(updated_entity.canonical_fields["current_value"], 210000);
    assert_ne!(
        updated_entity.updated_at, t1,
        "updated_at should advance after update"
    );
}

// =============================================================================
// 3c-3d. Schema migration (v0 -> v1) via SchemaMigrator
// =============================================================================

#[test]
fn test_3c_3d_schema_migration_v0_to_v1() {
    let conn = common::test_db();

    // Manually insert entities at v0 (simulating old data before migration)
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    for i in 0..5 {
        let id = format!("stale-metric-{}", i);
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
             VALUES (?1, 'metric', ?2, '', 'manual', '{\"current_value\": 100}', 0, ?3, ?3)",
            rusqlite::params![id, format!("Stale Metric {}", i), now],
        )
        .expect("Failed to insert stale entity");
    }

    // Also insert 2 entities already at v1
    for i in 0..2 {
        let id = format!("current-metric-{}", i);
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
             VALUES (?1, 'metric', ?2, '', 'manual', '{\"current_value\": 200}', 1, ?3, ?3)",
            rusqlite::params![id, format!("Current Metric {}", i), now],
        )
        .expect("Failed to insert current entity");
    }

    // Verify: 5 stale entities exist
    let stale_before = SchemaMigrator::find_stale_entities(&conn, "metric")
        .expect("find_stale should succeed");
    assert_eq!(
        stale_before.len(),
        5,
        "Should find 5 stale entities before migration"
    );

    // Run migration
    let migrated_count = SchemaMigrator::migrate_all_entities(&conn, "metric")
        .expect("migrate_all should succeed");
    assert_eq!(
        migrated_count, 5,
        "Should migrate exactly 5 entities from v0 to v1"
    );

    // Verify: 0 stale entities remain
    let stale_after = SchemaMigrator::find_stale_entities(&conn, "metric")
        .expect("find_stale after migration should succeed");
    assert!(
        stale_after.is_empty(),
        "No stale entities should remain after migration"
    );

    // Verify: all 7 entities are at v1
    let total_at_v1: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND _schema_version = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        total_at_v1, 7,
        "All 7 metric entities should be at _schema_version = 1"
    );

    // Verify: each migrated entity has _schema_version = 1
    for i in 0..5 {
        let id = format!("stale-metric-{}", i);
        let row = common::get_entity_row(&conn, &id).expect("Entity should exist");
        assert_eq!(
            row.5, 1,
            "Migrated entity {} should have _schema_version = 1",
            id
        );
    }
}

// =============================================================================
// 3e. Update after migration succeeds
// =============================================================================

#[test]
fn test_3e_update_after_migration() {
    let conn = common::test_db();

    // Insert entity at v0
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES ('mig-ent-1', 'metric', 'Pre-Migration Metric', '', 'manual', '{\"current_value\": 50}', 0, ?1, ?1)",
        rusqlite::params![now],
    )
    .expect("Failed to insert v0 entity");
    // Insert FTS entry so update can delete/re-insert it
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = 'mig-ent-1'",
        [],
    )
    .expect("Failed to insert FTS");

    // Migrate the entity from v0 to v1
    SchemaMigrator::migrate_entity(&conn, "mig-ent-1").expect("migrate should succeed");

    // Verify migration succeeded
    let row = common::get_entity_row(&conn, "mig-ent-1").unwrap();
    assert_eq!(row.5, 1, "Entity should be at v1 after migration");

    // Get the new updated_at after migration
    let updated_at = common::get_updated_at(&conn, "mig-ent-1");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update the migrated entity
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: "mig-ent-1".to_string(),
            expected_updated_at: updated_at.clone(),
            title: Some("Post-Migration Metric".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 150})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };

    let result = apply_patch_set(&conn, &update_set);
    assert!(
        result.is_ok(),
        "Update after migration should succeed, got: {:?}",
        result.err()
    );

    // Verify final state
    let entity = StoreService::get_entity(&conn, "mig-ent-1").unwrap();
    assert_eq!(entity.title, "Post-Migration Metric");
    assert_eq!(entity.canonical_fields["current_value"], 150);
    assert_eq!(entity.schema_version, 1);
}

// =============================================================================
// 3f. STALE ENTITY -- version behind current blocks update
// =============================================================================

#[test]
fn test_3f_stale_entity_blocks_update() {
    let conn = common::test_db();

    // Insert entity at v0 directly (simulating a stale entity)
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES ('stale-ent-1', 'metric', 'Stale Metric', '', 'manual', '{\"current_value\": 100}', 0, ?1, ?1)",
        rusqlite::params![now],
    )
    .expect("Failed to insert stale entity");
    // Insert FTS entry
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = 'stale-ent-1'",
        [],
    )
    .expect("Failed to insert FTS");

    // Attempt to update the stale entity (v0 < v1 current)
    let update_set = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: "stale-ent-1".to_string(),
            expected_updated_at: now.clone(),
            title: Some("Should Not Work".to_string()),
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
    assert!(
        result.is_err(),
        "Should reject update on stale entity (v0 < v1)"
    );

    match result.unwrap_err() {
        GargoyleError::Validation(ve) => {
            assert!(
                matches!(ve.code, ErrorCode::SchemaVersionMismatch),
                "Should be SchemaVersionMismatch, got: {:?}",
                ve.code
            );
            assert!(
                ve.message.contains("schema version"),
                "Error should mention schema version: {}",
                ve.message
            );
            assert!(
                ve.message.contains("Migration required"),
                "Error should mention migration: {}",
                ve.message
            );
            assert_eq!(
                ve.expected.as_deref(),
                Some("1"),
                "Expected version should be 1"
            );
            assert_eq!(
                ve.actual.as_deref(),
                Some("0"),
                "Actual version should be 0"
            );
        }
        other => panic!("Expected Validation(SchemaVersionMismatch), got: {:?}", other),
    }

    // Verify entity was NOT modified
    let row = common::get_entity_row(&conn, "stale-ent-1").unwrap();
    assert_eq!(row.2, "Stale Metric", "Title should remain unchanged");
    assert_eq!(row.5, 0, "_schema_version should still be 0");
}

// =============================================================================
// 3g. Migration then update -- full cycle
// =============================================================================

#[test]
fn test_3g_migration_then_update_full_cycle() {
    let conn = common::test_db();

    // Insert multiple entities at v0
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    for i in 0..3 {
        let id = format!("cycle-{}", i);
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
             VALUES (?1, 'metric', ?2, '', 'manual', '{\"current_value\": 50}', 0, ?3, ?3)",
            rusqlite::params![id, format!("Cycle Metric {}", i), now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
            rusqlite::params![id],
        )
        .unwrap();
    }

    // Step 1: Verify all are stale
    let stale = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
    assert_eq!(stale.len(), 3);

    // Step 2: Attempt update on stale entity -- should fail
    let update_stale = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: "cycle-0".to_string(),
            expected_updated_at: now.clone(),
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
    let fail_result = apply_patch_set(&conn, &update_stale);
    assert!(fail_result.is_err());
    assert!(matches!(
        fail_result.unwrap_err(),
        GargoyleError::Validation(_)
    ));

    // Step 3: Migrate all
    let count = SchemaMigrator::migrate_all_entities(&conn, "metric").unwrap();
    assert_eq!(count, 3);

    // Step 4: Verify none are stale
    let stale_after = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
    assert!(stale_after.is_empty());

    // Step 5: Update now succeeds
    let new_updated_at = common::get_updated_at(&conn, "cycle-0");
    std::thread::sleep(std::time::Duration::from_millis(10));

    let update_after = PatchSet {
        ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
            entity_id: "cycle-0".to_string(),
            expected_updated_at: new_updated_at,
            title: Some("Cycle Metric Updated".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(serde_json::json!({"current_value": 999})),
            category: None,
            priority: None,
            reason: None,
        })],
        run_id: None,
    };
    let success_result = apply_patch_set(&conn, &update_after);
    assert!(
        success_result.is_ok(),
        "Update after migration should succeed, got: {:?}",
        success_result.err()
    );

    let entity = StoreService::get_entity(&conn, "cycle-0").unwrap();
    assert_eq!(entity.title, "Cycle Metric Updated");
    assert_eq!(entity.canonical_fields["current_value"], 999);
    assert_eq!(entity.schema_version, 1);
}

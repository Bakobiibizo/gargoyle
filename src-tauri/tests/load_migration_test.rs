// Load test: Schema migration while entities exist.
//
// Phase 6E: Verifies that SchemaMigrator correctly handles bulk migrations
// of 20 entities, including stale version detection and full migration.

mod common;

use gargoyle_lib::schema::version::SchemaMigrator;
use rusqlite::params;

#[test]
fn test_schema_migration_under_load() {
    let conn = common::test_db();

    // Create 20 metric entities at schema_version 1
    for i in 0..20 {
        let id = format!("load-metric-{}", i);
        common::insert_test_entity(
            &conn,
            &id,
            "metric",
            &format!("Load Metric {}", i),
            "manual",
            r#"{"current_value": 100}"#,
        );
    }

    // Verify all at v1
    let v1_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND _schema_version = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(v1_count, 20, "All 20 should be at v1, got {}", v1_count);

    // Manually set all to v0 (simulate stale)
    conn.execute(
        "UPDATE entities SET _schema_version = 0 WHERE entity_type = 'metric'",
        [],
    )
    .unwrap();

    // Verify all are now stale
    let v0_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND _schema_version = 0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(v0_count, 20, "All 20 should be at v0, got {}", v0_count);

    // find_stale_entities should detect all 20
    let stale_before = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
    assert_eq!(
        stale_before.len(),
        20,
        "Should find 20 stale entities, found {}",
        stale_before.len()
    );

    // Run migration
    let migrated = SchemaMigrator::migrate_all_entities(&conn, "metric").unwrap();
    assert_eq!(migrated, 20, "Should migrate 20 entities, got {}", migrated);

    // Verify: 0 stale entities remain
    let stale_after = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
    assert!(
        stale_after.is_empty(),
        "Expected 0 stale entities, found {}",
        stale_after.len()
    );

    // Verify: All at current version (>= 1)
    let current_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND _schema_version >= 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        current_count, 20,
        "All 20 should be at current version, got {}",
        current_count
    );

    // Verify: No partial schema application (zero_version == 0)
    let zero_version: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND _schema_version < 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        zero_version, 0,
        "No entities should have version < 1, found {}",
        zero_version
    );
}

#[test]
fn test_migration_idempotent_on_current_entities() {
    let conn = common::test_db();

    // Create 10 entities already at the current version
    for i in 0..10 {
        let id = format!("current-metric-{}", i);
        common::insert_test_entity(
            &conn,
            &id,
            "metric",
            &format!("Current Metric {}", i),
            "manual",
            r#"{"current_value": 42}"#,
        );
    }

    // Running migrate_all_entities on already-current entities should migrate 0
    let migrated = SchemaMigrator::migrate_all_entities(&conn, "metric").unwrap();
    assert_eq!(
        migrated, 0,
        "Should migrate 0 entities (all current), got {}",
        migrated
    );

    // find_stale_entities should return empty
    let stale = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
    assert!(
        stale.is_empty(),
        "No stale entities should exist, found {}",
        stale.len()
    );
}

#[test]
fn test_migration_mixed_versions() {
    let conn = common::test_db();

    // Create entities: 10 at v0 (stale) and 10 at v1 (current)
    for i in 0..10 {
        let id = format!("stale-mix-{}", i);
        common::insert_test_entity(
            &conn,
            &id,
            "experiment",
            &format!("Stale Experiment {}", i),
            "manual",
            r#"{"hypothesis": "test"}"#,
        );
    }
    for i in 10..20 {
        let id = format!("current-mix-{}", i);
        common::insert_test_entity(
            &conn,
            &id,
            "experiment",
            &format!("Current Experiment {}", i),
            "manual",
            r#"{"hypothesis": "test"}"#,
        );
    }

    // Set first 10 to v0
    for i in 0..10 {
        conn.execute(
            "UPDATE entities SET _schema_version = 0 WHERE id = ?1",
            params![format!("stale-mix-{}", i)],
        )
        .unwrap();
    }

    // Verify we have a mix
    let stale_count = SchemaMigrator::find_stale_entities(&conn, "experiment")
        .unwrap()
        .len();
    assert_eq!(stale_count, 10, "Should have 10 stale, got {}", stale_count);

    // Migrate all
    let migrated = SchemaMigrator::migrate_all_entities(&conn, "experiment").unwrap();
    assert_eq!(
        migrated, 10,
        "Should migrate only the 10 stale entities, got {}",
        migrated
    );

    // All 20 should now be at current version
    let total_at_current: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'experiment' AND _schema_version >= 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(total_at_current, 20);

    // No stale entities remain
    let stale_after = SchemaMigrator::find_stale_entities(&conn, "experiment").unwrap();
    assert!(stale_after.is_empty());
}

#[test]
fn test_migration_does_not_affect_other_types() {
    let conn = common::test_db();

    // Create 5 metrics at v0 and 5 experiments at v0
    for i in 0..5 {
        let metric_id = format!("mt-{}", i);
        common::insert_test_entity(
            &conn,
            &metric_id,
            "metric",
            &format!("Metric {}", i),
            "manual",
            r#"{"current_value": 0}"#,
        );
        conn.execute(
            "UPDATE entities SET _schema_version = 0 WHERE id = ?1",
            params![metric_id],
        )
        .unwrap();

        let exp_id = format!("ex-{}", i);
        common::insert_test_entity(
            &conn,
            &exp_id,
            "experiment",
            &format!("Experiment {}", i),
            "manual",
            r#"{"hypothesis": "test"}"#,
        );
        conn.execute(
            "UPDATE entities SET _schema_version = 0 WHERE id = ?1",
            params![exp_id],
        )
        .unwrap();
    }

    // Migrate only metrics
    let migrated = SchemaMigrator::migrate_all_entities(&conn, "metric").unwrap();
    assert_eq!(migrated, 5);

    // Experiments should still be stale
    let stale_experiments = SchemaMigrator::find_stale_entities(&conn, "experiment").unwrap();
    assert_eq!(
        stale_experiments.len(),
        5,
        "Experiments should still be stale after migrating only metrics"
    );

    // Metrics should be migrated
    let stale_metrics = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
    assert!(
        stale_metrics.is_empty(),
        "All metrics should be migrated"
    );
}

#[test]
fn test_migration_updated_at_changes() {
    let conn = common::test_db();

    // Create entity and record its original updated_at
    let id = "ts-check-entity";
    let original_ts = common::insert_test_entity(
        &conn,
        id,
        "metric",
        "Timestamp Check Metric",
        "manual",
        r#"{"current_value": 0}"#,
    );

    // Set to stale version
    conn.execute(
        "UPDATE entities SET _schema_version = 0 WHERE id = ?1",
        params![id],
    )
    .unwrap();

    // Small delay to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Migrate
    SchemaMigrator::migrate_all_entities(&conn, "metric").unwrap();

    // Check updated_at changed
    let new_ts = common::get_updated_at(&conn, id);
    assert_ne!(
        original_ts, new_ts,
        "updated_at should change after migration"
    );
}

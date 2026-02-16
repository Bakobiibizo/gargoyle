// Scenario 10: related_to Audit Threshold
//
// Tests that the audit system correctly:
//   - Computes the related_to ratio
//   - Triggers a warning at >= 20%
//   - Reduces the ratio via custom type reclassification
//   - Handles empty databases
//   - Produces accurate breakdowns by entity type pairs
//   - Reflects changes after projection rebuild

mod common;

use gargoyle_lib::services::graph_builder::{
    approve_custom_type, audit_related_to, rebuild_projection, reclassify_relations,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn insert_entity(conn: &Connection, id: &str, entity_type: &str, title: &str) {
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

fn insert_relation(
    conn: &Connection,
    id: &str,
    from_id: &str,
    to_id: &str,
    relation_type: &str,
) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO relations (id, from_id, to_id, relation_type, weight, created_at)
         VALUES (?1, ?2, ?3, ?4, 1.0, ?5)",
        params![id, from_id, to_id, relation_type, now],
    )
    .expect("Failed to insert test relation");
}

/// Create N entities cycling through metric/experiment/result types.
fn create_entity_pool(conn: &Connection, prefix: &str, count: usize) {
    for i in 0..count {
        let entity_type = match i % 3 {
            0 => "metric",
            1 => "experiment",
            _ => "result",
        };
        insert_entity(
            conn,
            &format!("{}-{}", prefix, i),
            entity_type,
            &format!("{} Entity {}", prefix, i),
        );
    }
}

// =============================================================================
// 10a. BELOW THRESHOLD
// =============================================================================

#[test]
fn test_10a_below_threshold_no_warning() {
    let conn = common::test_db();

    // Create a pool of 10 entities to serve as endpoints
    create_entity_pool(&conn, "a", 10);

    // Create 50 relations: 8 related_to, 42 typed => 8/50 = 16%
    let mut rel_idx = 0;
    for i in 0..8 {
        insert_relation(
            &conn,
            &format!("r-a-{}", rel_idx),
            &format!("a-{}", i),
            &format!("a-{}", (i + 1) % 10),
            "related_to",
        );
        rel_idx += 1;
    }
    for i in 0..42 {
        insert_relation(
            &conn,
            &format!("r-a-{}", rel_idx),
            &format!("a-{}", i % 10),
            &format!("a-{}", (i + 1) % 10),
            "measures",
        );
        rel_idx += 1;
    }

    let result = audit_related_to(&conn).expect("audit_related_to should succeed");

    assert_eq!(result.total_relations, 50);
    assert_eq!(result.related_to_count, 8);
    assert!(
        (result.ratio - 0.16).abs() < 0.01,
        "Ratio should be ~16%, got {}",
        result.ratio
    );
    assert!(
        !result.threshold_exceeded,
        "Threshold should NOT be exceeded at 16%"
    );
    assert!(
        result.warning_message.is_none(),
        "No warning message expected below threshold"
    );
}

// =============================================================================
// 10b. AT THRESHOLD (exceeds 20%)
// =============================================================================

#[test]
fn test_10b_at_threshold_warning_with_breakdown() {
    let conn = common::test_db();

    // Create a pool of 10 entities
    create_entity_pool(&conn, "b", 10);

    // Build up to 53 relations: 11 related_to + 42 measures => 11/53 ~ 20.7%
    let mut rel_idx = 0;
    for i in 0..11 {
        insert_relation(
            &conn,
            &format!("r-b-{}", rel_idx),
            &format!("b-{}", i % 10),
            &format!("b-{}", (i + 1) % 10),
            "related_to",
        );
        rel_idx += 1;
    }
    for i in 0..42 {
        insert_relation(
            &conn,
            &format!("r-b-{}", rel_idx),
            &format!("b-{}", i % 10),
            &format!("b-{}", (i + 1) % 10),
            "measures",
        );
        rel_idx += 1;
    }

    let result = audit_related_to(&conn).expect("audit_related_to should succeed");

    assert_eq!(result.total_relations, 53);
    assert_eq!(result.related_to_count, 11);
    assert!(
        result.ratio >= 0.20,
        "Ratio should be >= 20%, got {:.4}",
        result.ratio
    );
    assert!(
        result.threshold_exceeded,
        "Threshold should be exceeded at {:.1}%",
        result.ratio * 100.0
    );
    assert!(
        result.warning_message.is_some(),
        "Warning message expected when threshold exceeded"
    );

    let msg = result.warning_message.unwrap();
    assert!(
        msg.contains("related_to edges at"),
        "Warning should mention ratio: {}",
        msg
    );
    assert!(
        msg.contains("threshold: 20%"),
        "Warning should mention threshold: {}",
        msg
    );
    assert!(
        msg.contains("Suggested: consider custom relation types"),
        "Warning should contain suggestion: {}",
        msg
    );

    // Breakdown should have entries
    assert!(
        !result.breakdown.is_empty(),
        "Breakdown should contain entries for related_to relations"
    );

    // Verify breakdown sums to related_to_count
    let breakdown_total: usize = result.breakdown.iter().map(|b| b.count).sum();
    assert_eq!(
        breakdown_total, 11,
        "Breakdown total should equal related_to_count"
    );
}

// =============================================================================
// 10c. CUSTOM TYPE REDUCES RATIO
// =============================================================================

#[test]
fn test_10c_custom_type_reduces_ratio() {
    let conn = common::test_db();

    // Create entity pool
    create_entity_pool(&conn, "c", 10);

    // 50 relations: 12 related_to + 38 measures => 12/50 = 24% (above threshold)
    let mut rel_idx = 0;
    let mut related_to_ids: Vec<String> = Vec::new();
    for i in 0..12 {
        let id = format!("r-c-{}", rel_idx);
        insert_relation(
            &conn,
            &id,
            &format!("c-{}", i % 10),
            &format!("c-{}", (i + 1) % 10),
            "related_to",
        );
        related_to_ids.push(id);
        rel_idx += 1;
    }
    for i in 0..38 {
        insert_relation(
            &conn,
            &format!("r-c-{}", rel_idx),
            &format!("c-{}", i % 10),
            &format!("c-{}", (i + 1) % 10),
            "measures",
        );
        rel_idx += 1;
    }

    // Verify: initially above threshold
    let before = audit_related_to(&conn).expect("audit before should succeed");
    assert_eq!(before.total_relations, 50);
    assert_eq!(before.related_to_count, 12);
    assert!(before.threshold_exceeded, "Should exceed threshold at 24%");
    assert!(before.warning_message.is_some());

    // Approve custom type
    approve_custom_type(
        &conn,
        "custom:correlates_with",
        "Statistical correlation between entities",
        &["metric".to_string(), "experiment".to_string()],
        &["metric".to_string(), "result".to_string()],
    )
    .expect("approve_custom_type should succeed");

    // Reclassify 5 related_to as custom:correlates_with
    let ids_to_reclassify: Vec<String> = related_to_ids[0..5].to_vec();
    let reclassified = reclassify_relations(
        &conn,
        "related_to",
        "custom:correlates_with",
        &ids_to_reclassify,
    )
    .expect("reclassify_relations should succeed");
    assert_eq!(reclassified, 5, "Should reclassify exactly 5 relations");

    // After reclassification: 7 related_to / 50 total = 14% (below threshold)
    let after = audit_related_to(&conn).expect("audit after should succeed");
    assert_eq!(after.total_relations, 50, "Total relations unchanged");
    assert_eq!(
        after.related_to_count, 7,
        "related_to count should drop from 12 to 7"
    );
    assert!(
        after.ratio < 0.20,
        "Ratio should be below 20% after reclassification, got {:.4}",
        after.ratio
    );
    assert!(
        !after.threshold_exceeded,
        "Threshold should no longer be exceeded"
    );
    assert!(
        after.warning_message.is_none(),
        "Warning should be cleared after ratio drops below 20%"
    );
}

// =============================================================================
// 10d. Empty database -- no relations
// =============================================================================

#[test]
fn test_10d_empty_database_no_relations() {
    let conn = common::test_db();

    let result = audit_related_to(&conn).expect("audit on empty DB should succeed");

    assert_eq!(result.total_relations, 0);
    assert_eq!(result.related_to_count, 0);
    assert!(
        (result.ratio - 0.0).abs() < f64::EPSILON,
        "Ratio should be 0.0 for empty DB, got {}",
        result.ratio
    );
    assert!(
        !result.threshold_exceeded,
        "Threshold should not be exceeded with 0 relations"
    );
    assert!(
        result.warning_message.is_none(),
        "No warning for empty database"
    );
    assert!(
        result.breakdown.is_empty(),
        "Breakdown should be empty with no relations"
    );
}

// =============================================================================
// 10e. Breakdown accuracy
// =============================================================================

#[test]
fn test_10e_breakdown_accuracy() {
    let conn = common::test_db();

    // Create specific entity types for controlled breakdown
    insert_entity(&conn, "met-1", "metric", "MRR");
    insert_entity(&conn, "met-2", "metric", "ARPU");
    insert_entity(&conn, "exp-1", "experiment", "Pricing Test");
    insert_entity(&conn, "exp-2", "experiment", "Onboarding Test");
    insert_entity(&conn, "res-1", "result", "Q1 Analysis");

    // Create related_to relations between specific type pairs:
    //   metric -> experiment: 3 relations
    //   metric -> metric: 1 relation
    //   experiment -> result: 2 relations
    insert_relation(&conn, "rt-1", "met-1", "exp-1", "related_to");
    insert_relation(&conn, "rt-2", "met-1", "exp-2", "related_to");
    insert_relation(&conn, "rt-3", "met-2", "exp-1", "related_to");
    insert_relation(&conn, "rt-4", "met-1", "met-2", "related_to");
    insert_relation(&conn, "rt-5", "exp-1", "res-1", "related_to");
    insert_relation(&conn, "rt-6", "exp-2", "res-1", "related_to");

    // Also add some typed relations so we have a mix
    insert_relation(&conn, "typed-1", "exp-1", "met-1", "measures");
    insert_relation(&conn, "typed-2", "exp-2", "met-2", "tests");

    let result = audit_related_to(&conn).expect("audit should succeed");

    assert_eq!(result.total_relations, 8);
    assert_eq!(result.related_to_count, 6);

    // Build a map from (from_type, to_type) -> count
    let breakdown_map: HashMap<(String, String), usize> = result
        .breakdown
        .iter()
        .map(|b| ((b.from_type.clone(), b.to_type.clone()), b.count))
        .collect();

    // Verify expected breakdowns
    assert_eq!(
        breakdown_map.get(&("metric".to_string(), "experiment".to_string())),
        Some(&3),
        "metric -> experiment should have 3 related_to relations"
    );
    assert_eq!(
        breakdown_map.get(&("metric".to_string(), "metric".to_string())),
        Some(&1),
        "metric -> metric should have 1 related_to relation"
    );
    assert_eq!(
        breakdown_map.get(&("experiment".to_string(), "result".to_string())),
        Some(&2),
        "experiment -> result should have 2 related_to relations"
    );

    // Verify breakdown total matches related_to_count
    let breakdown_total: usize = result.breakdown.iter().map(|b| b.count).sum();
    assert_eq!(breakdown_total, 6, "Breakdown total should equal 6");

    // The ratio is 6/8 = 75% => definitely above threshold
    assert!(result.threshold_exceeded, "6/8 = 75% should exceed threshold");
    assert!(result.warning_message.is_some());
}

// =============================================================================
// 10f. Projection rebuild after changes
// =============================================================================

#[test]
fn test_10f_projection_rebuild_after_changes() {
    let conn = common::test_db();

    // Create entities
    insert_entity(&conn, "p-m1", "metric", "Revenue");
    insert_entity(&conn, "p-m2", "metric", "Churn");
    insert_entity(&conn, "p-e1", "experiment", "Retention Campaign");
    insert_entity(&conn, "p-r1", "result", "Campaign Results");

    // Create initial relations
    insert_relation(&conn, "p-rel-1", "p-e1", "p-m1", "measures");
    insert_relation(&conn, "p-rel-2", "p-e1", "p-m2", "tests");
    insert_relation(&conn, "p-rel-3", "p-r1", "p-m1", "evidence_for");
    insert_relation(&conn, "p-rel-4", "p-m1", "p-m2", "related_to");
    insert_relation(&conn, "p-rel-5", "p-e1", "p-r1", "related_to");

    // First rebuild: verify initial counts
    let stats1 = rebuild_projection(&conn).expect("rebuild_projection should succeed");

    assert_eq!(stats1.total_entities, 4);
    assert_eq!(stats1.total_relations, 5);

    // Check entity type counts
    let entity_map1: HashMap<String, usize> =
        stats1.entity_type_counts.into_iter().collect();
    assert_eq!(entity_map1["metric"], 2);
    assert_eq!(entity_map1["experiment"], 1);
    assert_eq!(entity_map1["result"], 1);

    // Check relation type counts
    let rel_map1: HashMap<String, usize> =
        stats1.relation_type_counts.into_iter().collect();
    assert_eq!(rel_map1["measures"], 1);
    assert_eq!(rel_map1["tests"], 1);
    assert_eq!(rel_map1["evidence_for"], 1);
    assert_eq!(rel_map1["related_to"], 2);

    // Reclassify: approve custom type and reclassify one related_to
    approve_custom_type(
        &conn,
        "custom:drives",
        "Causal driver relationship",
        &["metric".to_string()],
        &["metric".to_string()],
    )
    .expect("approve_custom_type should succeed");

    let reclassified = reclassify_relations(
        &conn,
        "related_to",
        "custom:drives",
        &["p-rel-4".to_string()],
    )
    .expect("reclassify should succeed");
    assert_eq!(reclassified, 1);

    // Second rebuild: verify changes are reflected
    let stats2 = rebuild_projection(&conn).expect("second rebuild should succeed");

    assert_eq!(stats2.total_entities, 4, "Entity count unchanged");
    assert_eq!(stats2.total_relations, 5, "Total relation count unchanged");

    // Relation type counts should now reflect the reclassification
    let rel_map2: HashMap<String, usize> =
        stats2.relation_type_counts.into_iter().collect();
    assert_eq!(
        rel_map2.get("related_to"),
        Some(&1),
        "related_to should drop from 2 to 1"
    );
    assert_eq!(
        rel_map2.get("custom:drives"),
        Some(&1),
        "custom:drives should now have 1"
    );
    assert_eq!(rel_map2["measures"], 1, "measures unchanged");
    assert_eq!(rel_map2["tests"], 1, "tests unchanged");
    assert_eq!(rel_map2["evidence_for"], 1, "evidence_for unchanged");
}

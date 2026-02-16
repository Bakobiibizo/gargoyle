// Scenario 8: Relation Decision Tree Compliance
//
// Since the decision tree is advisory (not enforced by the validator), these
// integration tests validate that:
//   - Standard relation types work correctly
//   - Graph queries return expected results
//   - Entity type pairs have correct relation semantics
//   - Depth-limited traversal behaves properly

mod common;

use gargoyle_lib::services::graph_builder::{
    get_entity_graph, get_related_entities,
};
use rusqlite::{params, Connection};
use std::collections::HashSet;

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

// =============================================================================
// 8a. measures -- metric -> experiment
// =============================================================================

#[test]
fn test_8a_measures_metric_experiment() {
    let conn = common::test_db();

    // Create metric "MRR" and experiment "Pricing Test"
    insert_entity(&conn, "mrr-1", "metric", "MRR");
    insert_entity(&conn, "exp-1", "experiment", "Pricing Test");

    // Create relation: experiment measures metric
    insert_relation(&conn, "rel-8a", "exp-1", "mrr-1", "measures");

    // Verify: relation exists in the DB
    let rel_type: String = conn
        .query_row(
            "SELECT relation_type FROM relations WHERE id = ?1",
            params!["rel-8a"],
            |row| row.get(0),
        )
        .expect("Relation should exist");
    assert_eq!(rel_type, "measures");

    // Verify: get_related_entities from experiment with "measures" returns the metric
    let related = get_related_entities(&conn, "exp-1", "measures")
        .expect("get_related_entities should succeed");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].entity_id, "mrr-1");
    assert_eq!(related[0].entity_type, "metric");
    assert_eq!(related[0].title, "MRR");

    // Verify: get_related_entities from metric with "measures" returns the experiment
    let related_reverse = get_related_entities(&conn, "mrr-1", "measures")
        .expect("get_related_entities reverse should succeed");
    assert_eq!(related_reverse.len(), 1);
    assert_eq!(related_reverse[0].entity_id, "exp-1");
    assert_eq!(related_reverse[0].entity_type, "experiment");
}

// =============================================================================
// 8b. evidence_for -- result -> metric
// =============================================================================

#[test]
fn test_8b_evidence_for_result_metric() {
    let conn = common::test_db();

    // Create result "Q1 Results" and metric "MRR"
    insert_entity(&conn, "res-1", "result", "Q1 Results");
    insert_entity(&conn, "mrr-2", "metric", "MRR");

    // Create relation: result evidence_for metric
    insert_relation(&conn, "rel-8b", "res-1", "mrr-2", "evidence_for");

    // Verify: relation exists
    let rel_type: String = conn
        .query_row(
            "SELECT relation_type FROM relations WHERE id = ?1",
            params!["rel-8b"],
            |row| row.get(0),
        )
        .expect("Relation should exist");
    assert_eq!(rel_type, "evidence_for");

    // Verify: graph traversal from result finds metric
    let graph = get_entity_graph(&conn, "res-1", 1)
        .expect("get_entity_graph should succeed");
    assert_eq!(graph.root.entity_id, "res-1");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].entity_id, "mrr-2");
    assert_eq!(graph.nodes[0].entity_type, "metric");

    // Verify: graph traversal from metric finds result
    let graph_reverse = get_entity_graph(&conn, "mrr-2", 1)
        .expect("get_entity_graph reverse should succeed");
    assert_eq!(graph_reverse.root.entity_id, "mrr-2");
    assert_eq!(graph_reverse.nodes.len(), 1);
    assert_eq!(graph_reverse.nodes[0].entity_id, "res-1");

    // Verify: the edge has correct relation_type
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].relation_type, "evidence_for");
}

// =============================================================================
// 8c. tests -- experiment -> metric
// =============================================================================

#[test]
fn test_8c_tests_experiment_metric() {
    let conn = common::test_db();

    insert_entity(&conn, "exp-2", "experiment", "Conversion Funnel Test");
    insert_entity(&conn, "mrr-3", "metric", "Conversion Rate");

    // Create relation: experiment tests metric
    insert_relation(&conn, "rel-8c", "exp-2", "mrr-3", "tests");

    // Verify: correct relation_type in graph
    let graph = get_entity_graph(&conn, "exp-2", 1)
        .expect("get_entity_graph should succeed");

    assert_eq!(graph.root.entity_id, "exp-2");
    assert_eq!(graph.root.entity_type, "experiment");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].entity_id, "mrr-3");

    // Verify edge relation_type is "tests"
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].relation_type, "tests");
    assert_eq!(graph.edges[0].from_id, "exp-2");
    assert_eq!(graph.edges[0].to_id, "mrr-3");

    // Verify via get_related_entities with type filter
    let related = get_related_entities(&conn, "exp-2", "tests")
        .expect("get_related_entities should succeed");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].entity_id, "mrr-3");

    // Verify other relation types return empty
    let unrelated = get_related_entities(&conn, "exp-2", "measures")
        .expect("get_related_entities for wrong type should succeed");
    assert!(unrelated.is_empty(), "No 'measures' relations should exist");
}

// =============================================================================
// 8d. Graph traversal with multiple relation types
// =============================================================================

#[test]
fn test_8d_graph_traversal_multiple_relation_types() {
    let conn = common::test_db();

    // Create entities: metric, experiment, result
    insert_entity(&conn, "m1", "metric", "Revenue");
    insert_entity(&conn, "e1", "experiment", "Price Increase Test");
    insert_entity(&conn, "r1", "result", "Q2 Revenue Analysis");
    insert_entity(&conn, "m2", "metric", "ARPU");

    // Create various relation types
    insert_relation(&conn, "rel-d1", "e1", "m1", "measures");       // experiment measures metric
    insert_relation(&conn, "rel-d2", "e1", "m2", "tests");          // experiment tests metric
    insert_relation(&conn, "rel-d3", "r1", "m1", "evidence_for");   // result evidence_for metric
    insert_relation(&conn, "rel-d4", "m1", "m2", "related_to");     // metric related_to metric

    // Traverse from m1 with depth=2: should find all connected entities
    let graph = get_entity_graph(&conn, "m1", 2)
        .expect("get_entity_graph should succeed");

    assert_eq!(graph.root.entity_id, "m1");

    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.entity_id.as_str()).collect();
    assert!(node_ids.contains("e1"), "Should find experiment via measures relation");
    assert!(node_ids.contains("r1"), "Should find result via evidence_for relation");
    assert!(node_ids.contains("m2"), "Should find metric via related_to relation");
    assert_eq!(node_ids.len(), 3, "Should find exactly 3 connected entities");

    // Verify all 4 edges are present
    assert_eq!(graph.edges.len(), 4);

    let edge_types: HashSet<&str> = graph.edges.iter().map(|e| e.relation_type.as_str()).collect();
    assert!(edge_types.contains("measures"));
    assert!(edge_types.contains("tests"));
    assert!(edge_types.contains("evidence_for"));
    assert!(edge_types.contains("related_to"));
}

// =============================================================================
// 8e. Graph traversal respects depth
// =============================================================================

#[test]
fn test_8e_graph_traversal_respects_depth() {
    let conn = common::test_db();

    // Create chain: A -> B -> C -> D
    insert_entity(&conn, "chain-a", "metric", "Node A");
    insert_entity(&conn, "chain-b", "experiment", "Node B");
    insert_entity(&conn, "chain-c", "result", "Node C");
    insert_entity(&conn, "chain-d", "metric", "Node D");

    insert_relation(&conn, "rel-ab", "chain-a", "chain-b", "measures");
    insert_relation(&conn, "rel-bc", "chain-b", "chain-c", "evidence_for");
    insert_relation(&conn, "rel-cd", "chain-c", "chain-d", "tests");

    // Depth=1: only B found
    let graph_d1 = get_entity_graph(&conn, "chain-a", 1)
        .expect("depth=1 should succeed");
    let ids_d1: HashSet<&str> = graph_d1.nodes.iter().map(|n| n.entity_id.as_str()).collect();
    assert!(ids_d1.contains("chain-b"), "Depth 1: should find B");
    assert!(!ids_d1.contains("chain-c"), "Depth 1: should NOT find C");
    assert!(!ids_d1.contains("chain-d"), "Depth 1: should NOT find D");
    assert_eq!(ids_d1.len(), 1);

    // Depth=2: B and C found
    let graph_d2 = get_entity_graph(&conn, "chain-a", 2)
        .expect("depth=2 should succeed");
    let ids_d2: HashSet<&str> = graph_d2.nodes.iter().map(|n| n.entity_id.as_str()).collect();
    assert!(ids_d2.contains("chain-b"), "Depth 2: should find B");
    assert!(ids_d2.contains("chain-c"), "Depth 2: should find C");
    assert!(!ids_d2.contains("chain-d"), "Depth 2: should NOT find D");
    assert_eq!(ids_d2.len(), 2);

    // Depth=3: B, C, and D found
    let graph_d3 = get_entity_graph(&conn, "chain-a", 3)
        .expect("depth=3 should succeed");
    let ids_d3: HashSet<&str> = graph_d3.nodes.iter().map(|n| n.entity_id.as_str()).collect();
    assert!(ids_d3.contains("chain-b"), "Depth 3: should find B");
    assert!(ids_d3.contains("chain-c"), "Depth 3: should find C");
    assert!(ids_d3.contains("chain-d"), "Depth 3: should find D");
    assert_eq!(ids_d3.len(), 3);

    // Depth=0: no neighbors found, only root
    let graph_d0 = get_entity_graph(&conn, "chain-a", 0)
        .expect("depth=0 should succeed");
    assert!(graph_d0.nodes.is_empty(), "Depth 0: no neighbors");
    assert!(graph_d0.edges.is_empty(), "Depth 0: no edges");
    assert_eq!(graph_d0.root.entity_id, "chain-a");
}

// =============================================================================
// 8f. related_to creates a valid relation
// =============================================================================

#[test]
fn test_8f_related_to_creates_valid_relation() {
    let conn = common::test_db();

    insert_entity(&conn, "m-rt", "metric", "Churn Rate");
    insert_entity(&conn, "e-rt", "experiment", "Retention Campaign");

    // related_to is a valid (but generic) relation type
    insert_relation(&conn, "rel-rt", "m-rt", "e-rt", "related_to");

    // Verify: relation exists
    let rel_type: String = conn
        .query_row(
            "SELECT relation_type FROM relations WHERE id = ?1",
            params!["rel-rt"],
            |row| row.get(0),
        )
        .expect("Relation should exist");
    assert_eq!(rel_type, "related_to");

    // Verify: appears in graph traversal
    let graph = get_entity_graph(&conn, "m-rt", 1)
        .expect("get_entity_graph should succeed");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].entity_id, "e-rt");
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].relation_type, "related_to");

    // Verify: get_related_entities with "related_to" filter returns it
    let related = get_related_entities(&conn, "m-rt", "related_to")
        .expect("get_related_entities should succeed");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].entity_id, "e-rt");

    // Verify: related_to contributes to audit ratio (checked in scenario 10)
    // Here we just confirm the relation is properly typed
    let count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE relation_type = 'related_to'",
            [],
            |row| row.get(0),
        )
        .expect("Count query should succeed");
    assert_eq!(count, 1);
}

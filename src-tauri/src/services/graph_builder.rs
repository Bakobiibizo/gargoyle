// GraphBuilder: projection rebuild + related_to audit

use std::collections::{HashSet, VecDeque};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{GargoyleError, Result};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub entity_id: String,
    pub entity_type: String,
    pub title: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub relation_id: String,
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub weight: f64,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityGraph {
    pub root: GraphNode,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub total_relations: usize,
    pub related_to_count: usize,
    pub ratio: f64,
    pub threshold_exceeded: bool,
    pub breakdown: Vec<RelationBreakdown>,
    pub warning_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationBreakdown {
    pub from_type: String,
    pub to_type: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionStats {
    pub total_entities: usize,
    pub total_relations: usize,
    pub relation_type_counts: Vec<(String, usize)>,
    pub entity_type_counts: Vec<(String, usize)>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const RELATED_TO_THRESHOLD: f64 = 0.20;

// ---------------------------------------------------------------------------
// Graph projection
// ---------------------------------------------------------------------------

/// Returns the subgraph reachable from `entity_id` up to `depth` hops.
///
/// BFS traversal follows relations in both directions (from_id and to_id).
/// Visited entities are tracked to avoid cycles.
pub fn get_entity_graph(conn: &Connection, entity_id: &str, depth: usize) -> Result<EntityGraph> {
    // Fetch the root entity
    let root = fetch_graph_node(conn, entity_id)?;

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(entity_id.to_string());

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    // BFS queue: (entity_id, current_depth)
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((entity_id.to_string(), 0));

    while let Some((current_id, current_depth)) = queue.pop_front() {
        if current_depth >= depth {
            continue;
        }

        // Find relations where this entity is from_id or to_id
        let neighbor_edges = fetch_edges_for_entity(conn, &current_id)?;

        for edge in neighbor_edges {
            // Determine the neighbor id
            let neighbor_id = if edge.from_id == current_id {
                edge.to_id.clone()
            } else {
                edge.from_id.clone()
            };

            // Always collect the edge (even if neighbor visited, the edge may be new)
            if !edges.iter().any(|e| e.relation_id == edge.relation_id) {
                edges.push(edge);
            }

            if !visited.contains(&neighbor_id) {
                visited.insert(neighbor_id.clone());
                let node = fetch_graph_node(conn, &neighbor_id)?;
                nodes.push(node);
                queue.push_back((neighbor_id, current_depth + 1));
            }
        }
    }

    Ok(EntityGraph { root, nodes, edges })
}

/// Returns entities connected to `entity_id` by a specific `relation_type`.
pub fn get_related_entities(
    conn: &Connection,
    entity_id: &str,
    relation_type: &str,
) -> Result<Vec<GraphNode>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.from_id, r.to_id, r.relation_type, r.weight, r.confidence
         FROM relations r
         WHERE (r.from_id = ?1 OR r.to_id = ?1)
           AND r.relation_type = ?2",
    )?;

    let neighbor_ids: Vec<String> = stmt
        .query_map(params![entity_id, relation_type], |row| {
            let from_id: String = row.get(1)?;
            let to_id: String = row.get(2)?;
            if from_id == entity_id {
                Ok(to_id)
            } else {
                Ok(from_id)
            }
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut nodes = Vec::new();
    let mut seen = HashSet::new();
    for nid in neighbor_ids {
        if seen.insert(nid.clone()) {
            nodes.push(fetch_graph_node(conn, &nid)?);
        }
    }

    Ok(nodes)
}

/// Rebuild projection stats: counts entities by type and relations by type.
pub fn rebuild_projection(conn: &Connection) -> Result<ProjectionStats> {
    let total_entities: usize = conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;

    let total_relations: usize =
        conn.query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))?;

    // Entity type counts
    let mut stmt = conn.prepare(
        "SELECT entity_type, COUNT(*) FROM entities WHERE deleted_at IS NULL GROUP BY entity_type ORDER BY COUNT(*) DESC",
    )?;
    let entity_type_counts: Vec<(String, usize)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Relation type counts
    let mut stmt = conn.prepare(
        "SELECT relation_type, COUNT(*) FROM relations GROUP BY relation_type ORDER BY COUNT(*) DESC",
    )?;
    let relation_type_counts: Vec<(String, usize)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(ProjectionStats {
        total_entities,
        total_relations,
        relation_type_counts,
        entity_type_counts,
    })
}

// ---------------------------------------------------------------------------
// related_to audit (Scenario 10)
// ---------------------------------------------------------------------------

/// Audit the ratio of `related_to` relations vs typed relations.
///
/// Returns an `AuditResult` with breakdown and optional warning if ratio >= 20%.
pub fn audit_related_to(conn: &Connection) -> Result<AuditResult> {
    let total_relations: usize =
        conn.query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))?;

    let related_to_count: usize = conn.query_row(
        "SELECT COUNT(*) FROM relations WHERE relation_type = 'related_to'",
        [],
        |row| row.get(0),
    )?;

    let ratio = if total_relations == 0 {
        0.0
    } else {
        related_to_count as f64 / total_relations as f64
    };

    let threshold_exceeded = ratio >= RELATED_TO_THRESHOLD;

    // Breakdown by (from_type, to_type) for related_to relations
    let mut stmt = conn.prepare(
        "SELECT e1.entity_type AS from_type, e2.entity_type AS to_type, COUNT(*) AS cnt
         FROM relations r
         JOIN entities e1 ON r.from_id = e1.id
         JOIN entities e2 ON r.to_id = e2.id
         WHERE r.relation_type = 'related_to'
         GROUP BY e1.entity_type, e2.entity_type
         ORDER BY cnt DESC",
    )?;

    let breakdown: Vec<RelationBreakdown> = stmt
        .query_map([], |row| {
            Ok(RelationBreakdown {
                from_type: row.get(0)?,
                to_type: row.get(1)?,
                count: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let warning_message = if threshold_exceeded {
        let pct = ratio * 100.0;
        let mut msg = format!(
            "related_to edges at {:.1}% (threshold: 20%):",
            pct
        );
        for b in &breakdown {
            msg.push_str(&format!("\n  {} \u{2192} {}: {}", b.from_type, b.to_type, b.count));
        }
        msg.push_str("\n  Suggested: consider custom relation types for these patterns.");
        Some(msg)
    } else {
        None
    };

    Ok(AuditResult {
        total_relations,
        related_to_count,
        ratio,
        threshold_exceeded,
        breakdown,
        warning_message,
    })
}

// ---------------------------------------------------------------------------
// Custom type registration
// ---------------------------------------------------------------------------

/// Approve a custom relation type. The `type_key` must start with "custom:".
pub fn approve_custom_type(
    conn: &Connection,
    type_key: &str,
    description: &str,
    from_types: &[String],
    to_types: &[String],
) -> Result<()> {
    if !type_key.starts_with("custom:") {
        return Err(GargoyleError::Validation(crate::error::ValidationError {
            code: crate::error::ErrorCode::RelationTypeNotApproved,
            field_path: "type_key".to_string(),
            message: format!(
                "Custom relation type key must start with 'custom:', got '{}'",
                type_key
            ),
            expected: Some("custom:*".to_string()),
            actual: Some(type_key.to_string()),
        }));
    }

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    let from_types_json = serde_json::to_string(from_types)?;
    let to_types_json = serde_json::to_string(to_types)?;

    conn.execute(
        "INSERT INTO custom_relation_types (type_key, description, expected_from_types, expected_to_types, approved_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![type_key, description, from_types_json, to_types_json, now],
    )?;

    Ok(())
}

/// Reclassify existing relations from one relation_type to another.
///
/// Only reclassifies from `related_to` to a specific type.
pub fn reclassify_relations(
    conn: &Connection,
    from_type_key: &str,
    to_type_key: &str,
    relation_ids: &[String],
) -> Result<usize> {
    if relation_ids.is_empty() {
        return Ok(0);
    }

    // Validate from_type_key is "related_to"
    if from_type_key != "related_to" {
        return Err(GargoyleError::Validation(crate::error::ValidationError {
            code: crate::error::ErrorCode::RelationTypeNotApproved,
            field_path: "from_type_key".to_string(),
            message: format!(
                "Can only reclassify from 'related_to', got '{}'",
                from_type_key
            ),
            expected: Some("related_to".to_string()),
            actual: Some(from_type_key.to_string()),
        }));
    }

    let mut total_updated = 0usize;
    for rid in relation_ids {
        let updated = conn.execute(
            "UPDATE relations SET relation_type = ?1 WHERE id = ?2 AND relation_type = ?3",
            params![to_type_key, rid, from_type_key],
        )?;
        total_updated += updated;
    }

    Ok(total_updated)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn fetch_graph_node(conn: &Connection, entity_id: &str) -> Result<GraphNode> {
    conn.query_row(
        "SELECT id, entity_type, title, status FROM entities WHERE id = ?1",
        params![entity_id],
        |row| {
            Ok(GraphNode {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
            entity_type: "entity".to_string(),
            id: entity_id.to_string(),
        },
        other => GargoyleError::Database(other),
    })
}

fn fetch_edges_for_entity(conn: &Connection, entity_id: &str) -> Result<Vec<GraphEdge>> {
    let mut stmt = conn.prepare(
        "SELECT id, from_id, to_id, relation_type, weight, confidence
         FROM relations
         WHERE from_id = ?1 OR to_id = ?1",
    )?;

    let edges: Vec<GraphEdge> = stmt
        .query_map(params![entity_id], |row| {
            Ok(GraphEdge {
                relation_id: row.get(0)?,
                from_id: row.get(1)?,
                to_id: row.get(2)?,
                relation_type: row.get(3)?,
                weight: row.get(4)?,
                confidence: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(edges)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use rusqlite::Connection;

    fn test_db() -> Connection {
        let conn = crate::db::connection::create_memory_connection().unwrap();
        crate::db::migrations::run_migrations(&conn).unwrap();
        conn
    }

    fn insert_test_entity(conn: &Connection, id: &str, entity_type: &str, title: &str) -> String {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at) VALUES (?1, ?2, ?3, '', 'manual', '{}', 1, ?4, ?4)",
            params![id, entity_type, title, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
            params![id],
        )
        .unwrap();
        now
    }

    fn insert_test_relation(
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
            "INSERT INTO relations (id, from_id, to_id, relation_type, weight, created_at) VALUES (?1, ?2, ?3, ?4, 1.0, ?5)",
            params![id, from_id, to_id, relation_type, now],
        )
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // Graph traversal tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_entity_graph_depth_1_returns_immediate_neighbors() {
        let conn = test_db();
        insert_test_entity(&conn, "e1", "metric", "Root Metric");
        insert_test_entity(&conn, "e2", "experiment", "Experiment A");
        insert_test_entity(&conn, "e3", "result", "Result B");
        insert_test_entity(&conn, "e4", "metric", "Far Metric"); // 2 hops away

        insert_test_relation(&conn, "r1", "e1", "e2", "measures");
        insert_test_relation(&conn, "r2", "e1", "e3", "produced");
        insert_test_relation(&conn, "r3", "e2", "e4", "related_to"); // e4 is 2 hops from e1

        let graph = get_entity_graph(&conn, "e1", 1).unwrap();

        assert_eq!(graph.root.entity_id, "e1");
        assert_eq!(graph.root.title, "Root Metric");

        // Should find e2 and e3 (depth=1 neighbors), but NOT e4 (depth=2)
        let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.entity_id.as_str()).collect();
        assert!(node_ids.contains("e2"), "Should contain e2");
        assert!(node_ids.contains("e3"), "Should contain e3");
        assert!(!node_ids.contains("e4"), "Should NOT contain e4 at depth 1");

        // Should have edges r1 and r2 (connecting e1 to e2 and e3)
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_get_entity_graph_depth_2_returns_two_level_neighbors() {
        let conn = test_db();
        insert_test_entity(&conn, "e1", "metric", "Root");
        insert_test_entity(&conn, "e2", "experiment", "Mid");
        insert_test_entity(&conn, "e3", "result", "Leaf");

        insert_test_relation(&conn, "r1", "e1", "e2", "measures");
        insert_test_relation(&conn, "r2", "e2", "e3", "produced");

        let graph = get_entity_graph(&conn, "e1", 2).unwrap();

        let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.entity_id.as_str()).collect();
        assert!(node_ids.contains("e2"), "Should contain e2 at depth 1");
        assert!(node_ids.contains("e3"), "Should contain e3 at depth 2");
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_get_entity_graph_handles_cycles() {
        let conn = test_db();
        insert_test_entity(&conn, "e1", "metric", "A");
        insert_test_entity(&conn, "e2", "metric", "B");
        insert_test_entity(&conn, "e3", "metric", "C");

        // Cycle: e1 -> e2 -> e3 -> e1
        insert_test_relation(&conn, "r1", "e1", "e2", "related_to");
        insert_test_relation(&conn, "r2", "e2", "e3", "related_to");
        insert_test_relation(&conn, "r3", "e3", "e1", "related_to");

        // Should not infinite loop; depth=10 is more than enough to traverse the cycle
        let graph = get_entity_graph(&conn, "e1", 10).unwrap();

        // All 3 nodes should be found (e2 and e3 in nodes, e1 is root)
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 3);
    }

    #[test]
    fn test_get_entity_graph_no_relations_returns_just_root() {
        let conn = test_db();
        insert_test_entity(&conn, "e1", "metric", "Lonely Metric");

        let graph = get_entity_graph(&conn, "e1", 3).unwrap();

        assert_eq!(graph.root.entity_id, "e1");
        assert_eq!(graph.root.title, "Lonely Metric");
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_get_related_entities_filters_by_relation_type() {
        let conn = test_db();
        insert_test_entity(&conn, "e1", "metric", "Root");
        insert_test_entity(&conn, "e2", "experiment", "Exp A");
        insert_test_entity(&conn, "e3", "result", "Result B");
        insert_test_entity(&conn, "e4", "metric", "Metric C");

        insert_test_relation(&conn, "r1", "e1", "e2", "measures");
        insert_test_relation(&conn, "r2", "e1", "e3", "related_to");
        insert_test_relation(&conn, "r3", "e1", "e4", "measures");

        let nodes = get_related_entities(&conn, "e1", "measures").unwrap();

        let ids: HashSet<&str> = nodes.iter().map(|n| n.entity_id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("e2"));
        assert!(ids.contains("e4"));
        // e3 is related_to, not measures
        assert!(!ids.contains("e3"));
    }

    // -----------------------------------------------------------------------
    // Audit tests (Scenario 10)
    // -----------------------------------------------------------------------

    #[test]
    fn test_audit_below_threshold_no_warning() {
        let conn = test_db();

        // Create entities
        for i in 0..10 {
            insert_test_entity(
                &conn,
                &format!("e{}", i),
                if i % 2 == 0 { "metric" } else { "experiment" },
                &format!("Entity {}", i),
            );
        }

        // Create 50 relations total, 8 of which are related_to => 8/50 = 16%
        let mut rel_count = 0;
        for i in 0..8 {
            insert_test_relation(
                &conn,
                &format!("r{}", rel_count),
                &format!("e{}", i),
                &format!("e{}", (i + 1) % 10),
                "related_to",
            );
            rel_count += 1;
        }
        for i in 0..(50 - 8) {
            insert_test_relation(
                &conn,
                &format!("r{}", rel_count),
                &format!("e{}", i % 10),
                &format!("e{}", (i + 1) % 10),
                "measures",
            );
            rel_count += 1;
        }

        let result = audit_related_to(&conn).unwrap();

        assert_eq!(result.total_relations, 50);
        assert_eq!(result.related_to_count, 8);
        assert!((result.ratio - 0.16).abs() < 0.01);
        assert!(!result.threshold_exceeded);
        assert!(result.warning_message.is_none());
    }

    #[test]
    fn test_audit_at_threshold_warning_with_breakdown() {
        let conn = test_db();

        // Create entities of different types
        for i in 0..10 {
            let etype = match i % 3 {
                0 => "metric",
                1 => "experiment",
                _ => "result",
            };
            insert_test_entity(&conn, &format!("e{}", i), etype, &format!("Entity {}", i));
        }

        // 53 total relations, 11 are related_to => 11/53 = 20.7% (exceeds 20%)
        let mut rel_count = 0;
        for i in 0..11 {
            insert_test_relation(
                &conn,
                &format!("r{}", rel_count),
                &format!("e{}", i % 10),
                &format!("e{}", (i + 1) % 10),
                "related_to",
            );
            rel_count += 1;
        }
        for i in 0..42 {
            insert_test_relation(
                &conn,
                &format!("r{}", rel_count),
                &format!("e{}", i % 10),
                &format!("e{}", (i + 1) % 10),
                "measures",
            );
            rel_count += 1;
        }

        let result = audit_related_to(&conn).unwrap();

        assert_eq!(result.total_relations, 53);
        assert_eq!(result.related_to_count, 11);
        assert!(result.ratio >= 0.20);
        assert!(result.threshold_exceeded);
        assert!(result.warning_message.is_some());

        let msg = result.warning_message.unwrap();
        assert!(msg.contains("related_to edges at"));
        assert!(msg.contains("threshold: 20%"));
        assert!(msg.contains("Suggested: consider custom relation types"));
        assert!(!result.breakdown.is_empty());
    }

    #[test]
    fn test_audit_custom_type_reduces_ratio() {
        let conn = test_db();

        // Create entities
        for i in 0..5 {
            insert_test_entity(
                &conn,
                &format!("e{}", i),
                "metric",
                &format!("Metric {}", i),
            );
        }

        // 10 total, 5 related_to => 50% (well above threshold)
        let mut rel_count = 0;
        for i in 0..5 {
            insert_test_relation(
                &conn,
                &format!("r{}", rel_count),
                &format!("e{}", i),
                &format!("e{}", (i + 1) % 5),
                "related_to",
            );
            rel_count += 1;
        }
        for i in 0..5 {
            insert_test_relation(
                &conn,
                &format!("r{}", rel_count),
                &format!("e{}", i),
                &format!("e{}", (i + 1) % 5),
                "measures",
            );
            rel_count += 1;
        }

        // Before reclassification: 5/10 = 50%
        let result_before = audit_related_to(&conn).unwrap();
        assert!(result_before.threshold_exceeded);
        assert_eq!(result_before.related_to_count, 5);

        // Approve custom type and reclassify 3 relations
        approve_custom_type(
            &conn,
            "custom:correlates_with",
            "Statistical correlation between metrics",
            &["metric".to_string()],
            &["metric".to_string()],
        )
        .unwrap();

        let ids_to_reclassify: Vec<String> =
            vec!["r0".to_string(), "r1".to_string(), "r2".to_string()];
        let reclassified =
            reclassify_relations(&conn, "related_to", "custom:correlates_with", &ids_to_reclassify)
                .unwrap();
        assert_eq!(reclassified, 3);

        // After reclassification: 2/10 = 20% => exactly at threshold
        let result_after = audit_related_to(&conn).unwrap();
        assert_eq!(result_after.related_to_count, 2);
        assert_eq!(result_after.total_relations, 10);
        assert!((result_after.ratio - 0.20).abs() < 0.01);
    }

    // -----------------------------------------------------------------------
    // Custom type tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_approve_custom_type_inserts_correctly() {
        let conn = test_db();

        approve_custom_type(
            &conn,
            "custom:correlates_with",
            "Statistical correlation",
            &["metric".to_string()],
            &["metric".to_string(), "result".to_string()],
        )
        .unwrap();

        let (desc, from_types, to_types, approved_at): (String, String, String, String) = conn
            .query_row(
                "SELECT description, expected_from_types, expected_to_types, approved_at FROM custom_relation_types WHERE type_key = ?1",
                params!["custom:correlates_with"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(desc, "Statistical correlation");
        let from: Vec<String> = serde_json::from_str(&from_types).unwrap();
        assert_eq!(from, vec!["metric"]);
        let to: Vec<String> = serde_json::from_str(&to_types).unwrap();
        assert_eq!(to, vec!["metric", "result"]);
        assert!(!approved_at.is_empty());
    }

    #[test]
    fn test_approve_custom_type_rejects_non_custom_prefix() {
        let conn = test_db();

        let result = approve_custom_type(
            &conn,
            "measures",
            "Not a custom type",
            &[],
            &[],
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("custom:"), "Error should mention 'custom:' prefix requirement: {}", msg);
    }

    #[test]
    fn test_reclassify_relations_changes_relation_type() {
        let conn = test_db();
        insert_test_entity(&conn, "e1", "metric", "M1");
        insert_test_entity(&conn, "e2", "metric", "M2");
        insert_test_entity(&conn, "e3", "metric", "M3");

        insert_test_relation(&conn, "r1", "e1", "e2", "related_to");
        insert_test_relation(&conn, "r2", "e1", "e3", "related_to");
        insert_test_relation(&conn, "r3", "e2", "e3", "measures"); // not related_to

        let updated = reclassify_relations(
            &conn,
            "related_to",
            "custom:correlates_with",
            &["r1".to_string(), "r2".to_string()],
        )
        .unwrap();

        assert_eq!(updated, 2);

        // Verify the relations were updated
        let r1_type: String = conn
            .query_row(
                "SELECT relation_type FROM relations WHERE id = ?1",
                params!["r1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(r1_type, "custom:correlates_with");

        let r2_type: String = conn
            .query_row(
                "SELECT relation_type FROM relations WHERE id = ?1",
                params!["r2"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(r2_type, "custom:correlates_with");

        // r3 should be unchanged (it was "measures", not "related_to")
        let r3_type: String = conn
            .query_row(
                "SELECT relation_type FROM relations WHERE id = ?1",
                params!["r3"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(r3_type, "measures");
    }

    // -----------------------------------------------------------------------
    // Projection tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rebuild_projection_counts_correctly() {
        let conn = test_db();

        insert_test_entity(&conn, "e1", "metric", "Metric 1");
        insert_test_entity(&conn, "e2", "metric", "Metric 2");
        insert_test_entity(&conn, "e3", "experiment", "Experiment 1");
        insert_test_entity(&conn, "e4", "result", "Result 1");

        insert_test_relation(&conn, "r1", "e1", "e2", "related_to");
        insert_test_relation(&conn, "r2", "e1", "e3", "measures");
        insert_test_relation(&conn, "r3", "e3", "e4", "produced");
        insert_test_relation(&conn, "r4", "e2", "e4", "measures");

        let stats = rebuild_projection(&conn).unwrap();

        assert_eq!(stats.total_entities, 4);
        assert_eq!(stats.total_relations, 4);

        // Entity type counts
        let entity_map: HashMap<String, usize> =
            stats.entity_type_counts.into_iter().collect();
        assert_eq!(entity_map["metric"], 2);
        assert_eq!(entity_map["experiment"], 1);
        assert_eq!(entity_map["result"], 1);

        // Relation type counts
        let rel_map: HashMap<String, usize> =
            stats.relation_type_counts.into_iter().collect();
        assert_eq!(rel_map["measures"], 2);
        assert_eq!(rel_map["related_to"], 1);
        assert_eq!(rel_map["produced"], 1);
    }
}

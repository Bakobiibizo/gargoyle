// Performance benchmark tests for the Gargoyle knowledge graph system.
//
// These tests populate an in-memory SQLite database with 10,000 entities
// distributed across all 22 entity types, plus ~5,000 relations, then
// measure the performance of key operations against defined targets.
//
// Targets:
//   - FTS search: <200ms average (10 queries)
//   - Graph projection depth 2: <1s average
//   - Graph projection depth 3: <1s average
//   - List all entities: <500ms
//   - List entities by type: <100ms
//   - Batch insert (1000 via patch): measure throughput
//   - Dedup check: <500ms
//
// Run with: cargo test --test perf_benchmarks -- --nocapture 2>&1

mod common;

use std::time::Instant;

use rusqlite::{params, Connection};
use serde_json::json;

use gargoyle_lib::db::connection::create_memory_connection;
use gargoyle_lib::db::migrations::run_migrations;
use gargoyle_lib::services::dedup::DedupPipeline;
use gargoyle_lib::services::graph_builder;
use gargoyle_lib::services::indexer::IndexerService;
use gargoyle_lib::services::store::StoreService;
use gargoyle_lib::models::patch::{CreateEntityPayload, PatchOp, PatchSet};

use common::generators::{initial_status_for_type, ALL_ENTITY_TYPES};

// =============================================================================
// Constants
// =============================================================================

const TOTAL_ENTITIES: usize = 10_000;
const TOTAL_RELATIONS: usize = 5_000;

// Performance target thresholds (milliseconds)
const TARGET_FTS_SEARCH_MS: u128 = 200;
const TARGET_GRAPH_DEPTH2_MS: u128 = 1_000;
const TARGET_GRAPH_DEPTH3_MS: u128 = 1_000;
const TARGET_LIST_ALL_MS: u128 = 500;
const TARGET_LIST_BY_TYPE_MS: u128 = 100;
const TARGET_DEDUP_CHECK_MS: u128 = 500;

// =============================================================================
// Canonical fields for each entity type (deterministic, schema-valid)
// =============================================================================

fn pick<'a>(options: &'a [&'a str], index: usize) -> &'a str {
    options[index % options.len()]
}

fn canonical_fields_for_type(entity_type: &str, index: usize) -> serde_json::Value {
    match entity_type {
        "metric" => {
            let trend = pick(&["up", "down", "flat"], index);
            json!({
                "current_value": (index as f64) * 1.5,
                "target_value": (index as f64) * 2.0,
                "trend": trend,
                "data_source": "analytics"
            })
        }
        "experiment" => json!({
            "hypothesis": format!("Testing hypothesis #{}", index),
            "funnel_position": "checkout"
        }),
        "result" => json!({
            "findings": format!("Significant improvement in area {}", index),
            "methodology": "A/B test",
            "confidence_level": 0.5 + (index % 50) as f64 / 100.0
        }),
        "task" => {
            let effort = pick(&["S", "M", "L"], index);
            json!({
                "assignee": format!("person-{}", index % 20),
                "effort_estimate": effort,
                "acceptance_criteria": "Tests pass"
            })
        }
        "project" => json!({
            "owner_id": format!("lead-{}", index % 10),
            "objective": format!("Launch initiative {}", index),
            "success_criteria": "Revenue up 10%",
            "timeline": "Q1 2026"
        }),
        "decision" => json!({
            "owner_id": format!("decider-{}", index % 5),
            "rationale": format!("Based on data analysis #{}", index),
            "decided_at": "2026-01-15"
        }),
        "person" => {
            let role = pick(&["Engineer", "Designer", "PM", "Data Analyst"], index);
            let team = pick(&["Platform", "Growth", "Product"], index);
            json!({
                "email": format!("person{}@example.com", index),
                "role": role,
                "team": team,
                "external": false
            })
        }
        "note" => json!({
            "context": format!("Meeting notes from session {}", index),
            "tags": "planning,strategy"
        }),
        "session" => {
            let session_type = pick(&["planning", "review", "standup"], index);
            json!({
                "session_type": session_type,
                "participants": "Alice, Bob, Charlie",
                "agenda": format!("Sprint {} planning", index)
            })
        }
        "campaign" => {
            let channel = pick(&["email", "paid_social", "paid_search"], index);
            json!({
                "objective": format!("Increase signups batch {}", index),
                "budget": 10000.0 + (index as f64) * 100.0,
                "channel": channel
            })
        }
        "audience" => json!({
            "segment_criteria": format!("Enterprise segment {}", index),
            "estimated_size": 50000.0 + (index as f64) * 1000.0,
            "channels": "email,linkedin"
        }),
        "competitor" => json!({
            "website": format!("https://competitor{}.com", index),
            "positioning": "Enterprise leader",
            "strengths": "Strong brand recognition"
        }),
        "channel" => {
            let channel_type = pick(&["email", "social", "search"], index);
            json!({
                "channel_type": channel_type,
                "cost_model": "CPC",
                "budget_allocation": 5000.0 + (index as f64) * 50.0
            })
        }
        "spec" => {
            let spec_type = pick(&["technical", "product"], index);
            json!({
                "spec_type": spec_type,
                "version": format!("{}.0", 1 + index % 5),
                "author": format!("Author {}", index % 10)
            })
        }
        "budget" => json!({
            "total_amount": 100000.0 + (index as f64) * 1000.0,
            "currency": "USD",
            "period": format!("Q{} 2026", 1 + index % 4)
        }),
        "vendor" => {
            let vendor_type = pick(&["agency", "saas"], index);
            json!({
                "vendor_type": vendor_type,
                "contract_value": 50000.0 + (index as f64) * 500.0,
                "primary_contact": format!("Contact {}", index % 15)
            })
        }
        "playbook" => {
            let playbook_type = pick(&["sales", "marketing"], index);
            json!({
                "playbook_type": playbook_type,
                "trigger_conditions": "Lead qualifies",
                "expected_outcome": format!("Close deal variant {}", index)
            })
        }
        "taxonomy" => {
            let taxonomy_type = pick(&["category", "tag"], index);
            json!({
                "taxonomy_type": taxonomy_type,
                "level": (index % 5) as f64
            })
        }
        "backlog" => {
            let effort = pick(&["S", "M", "L"], index);
            json!({
                "priority_score": 1.0 + (index % 100) as f64,
                "effort": effort,
                "requester": format!("Team {}", index % 8)
            })
        }
        "brief" => {
            let brief_type = pick(&["creative", "campaign"], index);
            json!({
                "brief_type": brief_type,
                "deadline": "2026-06-01",
                "stakeholders": "Marketing, Design"
            })
        }
        "event" => {
            let event_type = pick(&["conference", "webinar"], index);
            json!({
                "event_type": event_type,
                "venue": "Virtual",
                "expected_attendees": 100.0 + (index as f64) * 10.0
            })
        }
        "policy" => {
            let policy_type = pick(&["security", "compliance"], index);
            json!({
                "policy_type": policy_type,
                "effective_date": "2026-01-01",
                "owner": format!("Legal team {}", index % 3)
            })
        }
        _ => json!({}),
    }
}

/// Human-readable title prefix for each entity type.
fn title_prefix_for_type(entity_type: &str) -> &str {
    match entity_type {
        "metric" => "Metric Analysis",
        "experiment" => "Growth Experiment",
        "result" => "Research Finding",
        "task" => "Engineering Task",
        "project" => "Strategic Project",
        "decision" => "Architecture Decision",
        "person" => "Team Member",
        "note" => "Meeting Note",
        "session" => "Planning Session",
        "campaign" => "Marketing Campaign",
        "audience" => "Target Audience",
        "competitor" => "Market Competitor",
        "channel" => "Distribution Channel",
        "spec" => "Technical Spec",
        "budget" => "Budget Allocation",
        "vendor" => "Service Vendor",
        "playbook" => "Sales Playbook",
        "taxonomy" => "Content Taxonomy",
        "backlog" => "Product Backlog",
        "brief" => "Creative Brief",
        "event" => "Industry Event",
        "policy" => "Security Policy",
        _ => "Entity",
    }
}

// =============================================================================
// Shared database setup — populates 10k entities + 5k relations
// =============================================================================

fn setup_populated_db() -> Connection {
    let conn = create_memory_connection().expect("Failed to create in-memory connection");
    run_migrations(&conn).expect("Failed to run migrations");

    let start = Instant::now();
    let num_types = ALL_ENTITY_TYPES.len();
    let per_type = TOTAL_ENTITIES / num_types; // ~454 per type
    let remainder = TOTAL_ENTITIES % num_types;

    // Collect all entity IDs for relation creation
    let mut all_entity_ids: Vec<String> = Vec::with_capacity(TOTAL_ENTITIES);

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    // Use a transaction for fast bulk inserts
    conn.execute_batch("BEGIN TRANSACTION").unwrap();

    for (type_idx, &entity_type) in ALL_ENTITY_TYPES.iter().enumerate() {
        let count = if type_idx < remainder {
            per_type + 1
        } else {
            per_type
        };

        let status = initial_status_for_type(entity_type);
        let prefix = title_prefix_for_type(entity_type);

        for i in 0..count {
            let id = format!("perf-{}-{}", entity_type, i);
            let title = format!("{} #{}", prefix, i);
            let body = format!(
                "This is {} number {} for performance testing. It covers analysis, planning, and execution phases.",
                entity_type, i
            );
            let cf = canonical_fields_for_type(entity_type, i);
            let cf_str = serde_json::to_string(&cf).unwrap();

            conn.execute(
                "INSERT INTO entities (id, entity_type, title, body_md, status, source, canonical_fields, _schema_version, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'manual', ?6, 1, ?7, ?7)",
                params![id, entity_type, title, body, status, cf_str, now],
            )
            .expect("Failed to insert entity");

            all_entity_ids.push(id);
        }
    }

    // Populate FTS index in bulk using rebuild (much faster than row-by-row inserts)
    conn.execute(
        "INSERT INTO entities_fts(entities_fts) VALUES('rebuild')",
        [],
    )
    .expect("Failed to rebuild FTS index");

    // Insert relations using deterministic pairing
    let relation_types = [
        "measures",
        "tests",
        "evidence_for",
        "supports",
        "related_to",
        "produced",
        "derived_from",
    ];

    for i in 0..TOTAL_RELATIONS {
        let from_idx = i % all_entity_ids.len();
        // Use a stride to spread relations across entity types
        let to_idx = (from_idx + 1 + (i * 7) % (all_entity_ids.len() - 1)) % all_entity_ids.len();

        // Skip self-relations
        if from_idx == to_idx {
            continue;
        }

        let rel_id = format!("rel-{}", i);
        let rel_type = relation_types[i % relation_types.len()];

        conn.execute(
            "INSERT INTO relations (id, from_id, to_id, relation_type, weight, created_at)
             VALUES (?1, ?2, ?3, ?4, 1.0, ?5)",
            params![
                rel_id,
                all_entity_ids[from_idx],
                all_entity_ids[to_idx],
                rel_type,
                now
            ],
        )
        .expect("Failed to insert relation");
    }

    conn.execute_batch("COMMIT").unwrap();

    let elapsed = start.elapsed();
    eprintln!(
        "[SETUP] Populated {} entities + {} relations in {:.2}s",
        TOTAL_ENTITIES,
        TOTAL_RELATIONS,
        elapsed.as_secs_f64()
    );

    // Verify counts
    let entity_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        entity_count, TOTAL_ENTITIES as i64,
        "Expected {} entities, got {}",
        TOTAL_ENTITIES, entity_count
    );

    let relation_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
        .unwrap();
    eprintln!(
        "[SETUP] Verified: {} entities, {} relations in DB",
        entity_count, relation_count
    );

    conn
}

/// Find entity IDs that have at least `min_relations` relations.
fn find_entities_with_relations(conn: &Connection, min_relations: usize, limit: usize) -> Vec<String> {
    let mut stmt = conn
        .prepare(
            "SELECT entity_id, cnt FROM (
                SELECT from_id AS entity_id, COUNT(*) AS cnt FROM relations GROUP BY from_id
                UNION ALL
                SELECT to_id AS entity_id, COUNT(*) AS cnt FROM relations GROUP BY to_id
             ) GROUP BY entity_id HAVING SUM(cnt) >= ?1 ORDER BY SUM(cnt) DESC LIMIT ?2",
        )
        .unwrap();

    let ids: Vec<String> = stmt
        .query_map(params![min_relations as i64, limit as i64], |row| {
            row.get(0)
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    ids
}

// =============================================================================
// Benchmark helpers
// =============================================================================

fn report_benchmark(name: &str, elapsed_ms: u128, target_ms: u128) {
    let status = if elapsed_ms <= target_ms {
        "PASS"
    } else {
        "WARN"
    };
    eprintln!(
        "[PERF] {} | {} | {:.1}ms (target: <{}ms)",
        status, name, elapsed_ms as f64, target_ms
    );
    if elapsed_ms > target_ms {
        eprintln!(
            "  WARNING: {} exceeded target by {:.1}ms ({:.0}% over)",
            name,
            (elapsed_ms - target_ms) as f64,
            ((elapsed_ms as f64 / target_ms as f64) - 1.0) * 100.0
        );
    }
}

// =============================================================================
// Benchmark Tests
// =============================================================================

#[test]
fn test_perf_fts_search_10k_entities() {
    let conn = setup_populated_db();

    let queries = vec![
        "Metric",
        "Analysis",
        "Growth",
        "Engineering",
        "Strategic",
        "Planning",
        "Marketing",
        "Research",
        "Technical",
        "performance",
    ];

    let mut total_ms: u128 = 0;
    let mut total_results: usize = 0;

    for query in &queries {
        let start = Instant::now();
        let results = IndexerService::search_fts(&conn, query, 20).unwrap();
        let elapsed = start.elapsed().as_millis();
        total_ms += elapsed;
        total_results += results.len();
        eprintln!(
            "  FTS '{}': {}ms ({} results)",
            query,
            elapsed,
            results.len()
        );
    }

    let avg_ms = total_ms / queries.len() as u128;
    eprintln!(
        "  FTS average: {}ms over {} queries ({} total results)",
        avg_ms,
        queries.len(),
        total_results
    );

    report_benchmark("FTS search (avg of 10 queries)", avg_ms, TARGET_FTS_SEARCH_MS);

    // Soft assertion: warn but do not fail
    if avg_ms > TARGET_FTS_SEARCH_MS {
        eprintln!(
            "  NOTE: FTS search average {}ms exceeds {}ms target. This may be acceptable in CI.",
            avg_ms, TARGET_FTS_SEARCH_MS
        );
    }
}

#[test]
fn test_perf_graph_projection_depth2() {
    let conn = setup_populated_db();

    let candidates = find_entities_with_relations(&conn, 3, 5);
    assert!(
        !candidates.is_empty(),
        "Need entities with relations for graph benchmark"
    );

    let mut total_ms: u128 = 0;

    for entity_id in &candidates {
        let start = Instant::now();
        let graph = graph_builder::get_entity_graph(&conn, entity_id, 2).unwrap();
        let elapsed = start.elapsed().as_millis();
        total_ms += elapsed;
        eprintln!(
            "  Graph depth=2 for '{}': {}ms ({} nodes, {} edges)",
            entity_id,
            elapsed,
            graph.nodes.len(),
            graph.edges.len()
        );
    }

    let avg_ms = total_ms / candidates.len() as u128;
    eprintln!(
        "  Graph depth=2 average: {}ms over {} entities",
        avg_ms,
        candidates.len()
    );

    report_benchmark("Graph projection depth=2", avg_ms, TARGET_GRAPH_DEPTH2_MS);

    if avg_ms > TARGET_GRAPH_DEPTH2_MS {
        eprintln!(
            "  NOTE: Graph depth=2 average {}ms exceeds {}ms target.",
            avg_ms, TARGET_GRAPH_DEPTH2_MS
        );
    }
}

#[test]
fn test_perf_graph_projection_depth3() {
    let conn = setup_populated_db();

    let candidates = find_entities_with_relations(&conn, 3, 5);
    assert!(
        !candidates.is_empty(),
        "Need entities with relations for graph benchmark"
    );

    let mut total_ms: u128 = 0;

    for entity_id in &candidates {
        let start = Instant::now();
        let graph = graph_builder::get_entity_graph(&conn, entity_id, 3).unwrap();
        let elapsed = start.elapsed().as_millis();
        total_ms += elapsed;
        eprintln!(
            "  Graph depth=3 for '{}': {}ms ({} nodes, {} edges)",
            entity_id,
            elapsed,
            graph.nodes.len(),
            graph.edges.len()
        );
    }

    let avg_ms = total_ms / candidates.len() as u128;
    eprintln!(
        "  Graph depth=3 average: {}ms over {} entities",
        avg_ms,
        candidates.len()
    );

    report_benchmark("Graph projection depth=3", avg_ms, TARGET_GRAPH_DEPTH3_MS);

    if avg_ms > TARGET_GRAPH_DEPTH3_MS {
        eprintln!(
            "  NOTE: Graph depth=3 average {}ms exceeds {}ms target.",
            avg_ms, TARGET_GRAPH_DEPTH3_MS
        );
    }
}

#[test]
fn test_perf_list_entities_10k() {
    let conn = setup_populated_db();

    let start = Instant::now();
    let entities = StoreService::list_entities(&conn, None).unwrap();
    let elapsed_ms = start.elapsed().as_millis();

    eprintln!(
        "  Listed {} entities (no filter) in {}ms",
        entities.len(),
        elapsed_ms
    );
    assert_eq!(
        entities.len(),
        TOTAL_ENTITIES,
        "Should return all {} entities",
        TOTAL_ENTITIES
    );

    report_benchmark("List all entities (10k)", elapsed_ms, TARGET_LIST_ALL_MS);

    if elapsed_ms > TARGET_LIST_ALL_MS {
        eprintln!(
            "  NOTE: List all entities {}ms exceeds {}ms target.",
            elapsed_ms, TARGET_LIST_ALL_MS
        );
    }
}

#[test]
fn test_perf_list_entities_by_type() {
    let conn = setup_populated_db();

    // Test listing for several different entity types
    let test_types = ["metric", "experiment", "task", "project", "campaign"];
    let mut total_ms: u128 = 0;

    for &entity_type in &test_types {
        let start = Instant::now();
        let entities = StoreService::list_entities(&conn, Some(entity_type)).unwrap();
        let elapsed = start.elapsed().as_millis();
        total_ms += elapsed;
        eprintln!(
            "  List '{}': {}ms ({} entities)",
            entity_type,
            elapsed,
            entities.len()
        );
        assert!(
            !entities.is_empty(),
            "Should have entities of type '{}'",
            entity_type
        );
    }

    let avg_ms = total_ms / test_types.len() as u128;
    eprintln!(
        "  List-by-type average: {}ms over {} types",
        avg_ms,
        test_types.len()
    );

    report_benchmark("List entities by type (avg)", avg_ms, TARGET_LIST_BY_TYPE_MS);

    if avg_ms > TARGET_LIST_BY_TYPE_MS {
        eprintln!(
            "  NOTE: List by type average {}ms exceeds {}ms target.",
            avg_ms, TARGET_LIST_BY_TYPE_MS
        );
    }
}

#[test]
fn test_perf_batch_insert_1000() {
    let conn = create_memory_connection().expect("Failed to create in-memory connection");
    run_migrations(&conn).expect("Failed to run migrations");

    let batch_size = 1000;

    // Build a PatchSet with 1000 CreateEntity ops
    let mut ops = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let type_idx = i % ALL_ENTITY_TYPES.len();
        let entity_type = ALL_ENTITY_TYPES[type_idx];
        let status = initial_status_for_type(entity_type);
        let cf = canonical_fields_for_type(entity_type, i);

        ops.push(PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: entity_type.to_string(),
            title: format!("Batch {} #{}", title_prefix_for_type(entity_type), i),
            source: "manual".to_string(),
            canonical_fields: cf,
            body_md: Some(format!("Batch insert entity {} for throughput test", i)),
            status: Some(status.to_string()),
            category: None,
            priority: None,
        }));
    }

    let patch_set = PatchSet {
        ops,
        run_id: None,
    };

    let start = Instant::now();
    let result = StoreService::apply_patch_set(&conn, &patch_set).unwrap();
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis();

    let throughput = batch_size as f64 / elapsed.as_secs_f64();

    eprintln!(
        "  Batch insert {} entities via patch protocol: {}ms ({:.0} entities/sec)",
        batch_size, elapsed_ms, throughput
    );
    eprintln!(
        "  Applied: {}, Errors: {}",
        result.applied.len(),
        result.errors.len()
    );

    report_benchmark(
        &format!("Batch insert {} entities", batch_size),
        elapsed_ms,
        10_000, // No hard target, just measure. Use 10s as generous upper bound.
    );

    assert_eq!(
        result.applied.len(),
        batch_size,
        "All {} entities should be applied",
        batch_size
    );
    assert!(
        result.errors.is_empty(),
        "No errors expected during batch insert"
    );

    // Verify entities exist
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, batch_size as i64);
}

#[test]
fn test_perf_dedup_check_10k() {
    let conn = setup_populated_db();

    // Insert a new entity that is likely to have duplicates (same title as existing)
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES ('dedup-probe', 'metric', 'Metric Analysis #1', 'Probe entity for dedup', 'manual', '{\"current_value\": 999}', 1, ?1, ?1)",
        params![now],
    )
    .unwrap();

    // Also populate FTS for the probe
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = 'dedup-probe'",
        [],
    )
    .unwrap();

    let start = Instant::now();
    let suggestions = DedupPipeline::check_for_duplicates(&conn, "dedup-probe").unwrap();
    let elapsed_ms = start.elapsed().as_millis();

    eprintln!(
        "  Dedup check with {}+ entities in DB: {}ms ({} suggestions found)",
        TOTAL_ENTITIES,
        elapsed_ms,
        suggestions.len()
    );

    report_benchmark("Dedup check (10k entities)", elapsed_ms, TARGET_DEDUP_CHECK_MS);

    if elapsed_ms > TARGET_DEDUP_CHECK_MS {
        eprintln!(
            "  NOTE: Dedup check {}ms exceeds {}ms target.",
            elapsed_ms, TARGET_DEDUP_CHECK_MS
        );
    }
}

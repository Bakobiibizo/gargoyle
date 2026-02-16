// Scenario 9: Template chain integration tests
//
// Tests the 3-template chain end-to-end:
//   1. analytics-metric-tree (foundational)
//   2. analytics-experiment-plan (requires metric)
//   3. analytics-anomaly-investigation (requires experiment)
//
// Covers: full chain, prerequisite checking, advisory enforcement,
//         provenance reconstruction, and run logging.

mod common;

use gargoyle_lib::services::template_runner::{
    check_prerequisites, get_template_definition, run_template_full, TemplateInput, TemplateOutput,
};
use rusqlite::params;
use serde_json::json;

/// Helper: run the metric-tree template and return the output.
fn run_metric_tree(conn: &rusqlite::Connection) -> TemplateOutput {
    let input = TemplateInput {
        template_key: "analytics-metric-tree".to_string(),
        params: json!({
            "business_model": "SaaS",
            "primary_objective": "Revenue Growth",
            "customer_journey": "Acquisition -> Activation -> Revenue -> Retention -> Referral"
        }),
        force: false,
    };
    run_template_full(conn, &input).expect("metric-tree should succeed")
}

/// Helper: pick a metric entity ID from a metric-tree output.
fn pick_metric_id(output: &TemplateOutput) -> String {
    output
        .patch_result
        .applied
        .iter()
        .find_map(|op| op.entity_id.clone())
        .expect("Should have at least one entity with an ID")
}

/// Helper: run the experiment-plan template.
fn run_experiment_plan(
    conn: &rusqlite::Connection,
    metric_entity_id: &str,
) -> TemplateOutput {
    let input = TemplateInput {
        template_key: "analytics-experiment-plan".to_string(),
        params: json!({
            "hypothesis": "Increasing trial length from 14 to 30 days will improve activation rate",
            "funnel_position": "activation",
            "metric_entity_id": metric_entity_id
        }),
        force: false,
    };
    run_template_full(conn, &input).expect("experiment-plan should succeed")
}

/// Helper: run the anomaly-investigation template.
fn run_anomaly_investigation(
    conn: &rusqlite::Connection,
    kpi_entity_id: &str,
) -> TemplateOutput {
    let input = TemplateInput {
        template_key: "analytics-anomaly-investigation".to_string(),
        params: json!({
            "kpi_entity_id": kpi_entity_id,
            "time_window": "last_30_days",
            "baseline_period": "previous_quarter"
        }),
        force: false,
    };
    run_template_full(conn, &input).expect("anomaly-investigation should succeed")
}

// =============================================================================
// Test 1: Full 3-template chain
// =============================================================================

#[test]
fn test_full_3_template_chain() {
    let conn = common::test_db();

    // Step 1: Run analytics-metric-tree -> produces 7 metrics + 6 relations
    let mt_output = run_metric_tree(&conn);
    assert!(!mt_output.run_id.is_empty());
    assert!(mt_output.warnings.is_empty());

    // Count metric entities produced
    let metric_entity_count = mt_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.entity_id.is_some() && op.relation_id.is_none())
        .count();
    assert!(
        metric_entity_count >= 5,
        "metric-tree should produce at least 5 entities, got {}",
        metric_entity_count
    );

    // Step 2: Pick a metric entity_id from step 1
    let metric_id = pick_metric_id(&mt_output);

    // Step 3: Run analytics-experiment-plan with that metric_id
    let ep_output = run_experiment_plan(&conn, &metric_id);
    assert!(!ep_output.run_id.is_empty());
    assert_ne!(ep_output.run_id, mt_output.run_id);

    // Verify experiment entity was created
    let experiment_entities: Vec<_> = ep_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.entity_id.is_some() && op.relation_id.is_none() && op.claim_id.is_none())
        .collect();
    assert_eq!(
        experiment_entities.len(),
        1,
        "experiment-plan should produce exactly 1 entity"
    );
    let experiment_id = experiment_entities[0].entity_id.as_ref().unwrap();

    // Verify relations were created (tests + measures)
    let relation_count = ep_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.relation_id.is_some())
        .count();
    assert_eq!(
        relation_count, 2,
        "experiment-plan should produce exactly 2 relations"
    );

    // Verify entity properties in DB
    let (etype, source, status): (String, String, Option<String>) = conn
        .query_row(
            "SELECT entity_type, source, status FROM entities WHERE id = ?1",
            params![experiment_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(etype, "experiment");
    assert_eq!(source, "template");
    assert_eq!(status, Some("draft".to_string()));

    // Step 4: Run analytics-anomaly-investigation with the same metric_id
    let ai_output = run_anomaly_investigation(&conn, &metric_id);
    assert!(!ai_output.run_id.is_empty());
    assert_ne!(ai_output.run_id, ep_output.run_id);

    // Verify result entity was created
    let result_entities: Vec<_> = ai_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.entity_id.is_some() && op.relation_id.is_none() && op.claim_id.is_none())
        .collect();
    assert_eq!(
        result_entities.len(),
        1,
        "anomaly-investigation should produce exactly 1 result entity"
    );
    let result_entity_id = result_entities[0].entity_id.as_ref().unwrap();

    // Verify result entity properties in DB
    let (rtype, rsource, rstatus): (String, String, Option<String>) = conn
        .query_row(
            "SELECT entity_type, source, status FROM entities WHERE id = ?1",
            params![result_entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(rtype, "result");
    assert_eq!(rsource, "template");
    assert_eq!(rstatus, Some("draft".to_string()));

    // Verify relation was created (evidence_for)
    let ai_relation_count = ai_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.relation_id.is_some())
        .count();
    assert_eq!(
        ai_relation_count, 1,
        "anomaly-investigation should produce 1 relation"
    );

    // Verify claim was created
    let claim_count = ai_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.claim_id.is_some())
        .count();
    assert_eq!(
        claim_count, 1,
        "anomaly-investigation should produce 1 claim"
    );

    // Verify claim properties in DB
    let claim_id = ai_output
        .patch_result
        .applied
        .iter()
        .find_map(|op| op.claim_id.clone())
        .unwrap();
    let (subject, predicate, object, confidence, evidence_eid): (
        String,
        String,
        String,
        f64,
        String,
    ) = conn
        .query_row(
            "SELECT subject, predicate, object, confidence, evidence_entity_id FROM claims WHERE claim_id = ?1",
            params![claim_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();
    assert_eq!(predicate, "anomaly_detected_in");
    assert_eq!(object, "last_30_days");
    assert!((confidence - 0.5).abs() < f64::EPSILON);
    assert_eq!(evidence_eid, *result_entity_id);
    assert!(!subject.is_empty(), "Claim subject should be the metric title");

    // Verify all 3 runs exist in the database
    let run_count: usize = conn
        .query_row("SELECT COUNT(*) FROM runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(run_count, 3, "Should have exactly 3 runs");
}

// =============================================================================
// Test 2: Prerequisite checking
// =============================================================================

#[test]
fn test_prerequisite_checking_experiment_plan_fresh_db() {
    let conn = common::test_db();

    // On fresh DB (no metrics), experiment-plan prerequisites should be unsatisfied
    let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].satisfied,
        "experiment-plan prerequisites should not be satisfied on fresh DB"
    );
    assert!(results[0].message.is_some());
    let msg = results[0].message.as_ref().unwrap();
    assert!(
        msg.contains("metric"),
        "Message should mention metric: {}",
        msg
    );
}

#[test]
fn test_prerequisite_checking_experiment_plan_after_metric_tree() {
    let conn = common::test_db();

    // Run metric-tree first to create metrics
    let _mt_output = run_metric_tree(&conn);

    // Now experiment-plan prerequisites should be satisfied
    let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        results[0].satisfied,
        "experiment-plan prerequisites should be satisfied after metric-tree"
    );
    assert!(results[0].message.is_none());
}

#[test]
fn test_prerequisite_checking_anomaly_investigation_fresh_db() {
    let conn = common::test_db();

    // On fresh DB (no experiments), anomaly-investigation prerequisites should be unsatisfied
    let results = check_prerequisites(&conn, "analytics-anomaly-investigation").unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].satisfied,
        "anomaly-investigation prerequisites should not be satisfied on fresh DB"
    );
}

#[test]
fn test_prerequisite_checking_anomaly_investigation_after_chain() {
    let conn = common::test_db();

    // Run metric-tree
    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);

    // Run experiment-plan to create an experiment
    let _ep_output = run_experiment_plan(&conn, &metric_id);

    // Now anomaly-investigation prerequisites should be satisfied (needs experiment)
    let results = check_prerequisites(&conn, "analytics-anomaly-investigation").unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        results[0].satisfied,
        "anomaly-investigation prerequisites should be satisfied after experiment-plan"
    );
}

#[test]
fn test_prerequisite_checking_metric_tree_always_satisfied() {
    let conn = common::test_db();

    // metric-tree has no prerequisites
    let results = check_prerequisites(&conn, "analytics-metric-tree").unwrap();
    assert!(
        results.is_empty(),
        "metric-tree should have no prerequisites"
    );
}

// =============================================================================
// Test 3: Prerequisite enforcement (advisory)
// =============================================================================

#[test]
fn test_advisory_enforcement_experiment_plan_force_false() {
    let conn = common::test_db();

    // Attempt experiment-plan with no metrics and force=false -> error
    let input = TemplateInput {
        template_key: "analytics-experiment-plan".to_string(),
        params: json!({
            "hypothesis": "Test hypothesis",
            "funnel_position": "activation",
            "metric_entity_id": "nonexistent-id"
        }),
        force: false,
    };

    let result = run_template_full(&conn, &input);
    assert!(result.is_err(), "Should fail when prerequisites not met and force=false");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Prerequisites not met"),
        "Error should mention prerequisites: {}",
        err_msg
    );
}

#[test]
fn test_advisory_enforcement_experiment_plan_force_true() {
    let conn = common::test_db();

    // First, insert a metric entity so the generate_ops validation passes
    // (even though prerequisite check will be forced past)
    let metric_id = uuid::Uuid::new_v4().to_string();
    common::insert_test_metric(&conn, &metric_id, "Test Metric");

    // Now soft-delete all metrics so prerequisite is not satisfied
    // But we keep the entity in the DB (deleted) so it exists for FK,
    // Actually let's create a fresh metric but delete it so prereq fails.
    // Then create another metric so the template can reference it.
    //
    // Actually the prereq checks for entity_type=metric with deleted_at IS NULL.
    // If we delete the metric, the template's entity lookup will also fail.
    // So let's just test force=true with a metric present.
    //
    // The experiment-plan requires metric(1). If we have 1 metric, prereqs are met.
    // But anomaly-investigation requires experiment(1), and we have 0 experiments.
    // Let's test force=true with anomaly-investigation instead, since that
    // requires an experiment but we can still have a metric to reference.

    let input = TemplateInput {
        template_key: "analytics-anomaly-investigation".to_string(),
        params: json!({
            "kpi_entity_id": metric_id,
            "time_window": "last_7_days",
            "baseline_period": "previous_month"
        }),
        force: true,
    };

    let output = run_template_full(&conn, &input).expect("Should succeed with force=true");
    assert!(!output.run_id.is_empty());
    assert!(
        !output.warnings.is_empty(),
        "Should have warnings when prerequisites forced"
    );
    assert!(
        output.warnings[0].starts_with("FORCED:"),
        "Warning should start with FORCED: but got: {}",
        output.warnings[0]
    );

    // Verify entities were still created
    let result_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'result' AND provenance_run_id = ?1",
            params![output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(result_count, 1, "Should have created 1 result entity");
}

#[test]
fn test_advisory_enforcement_anomaly_force_false_fails() {
    let conn = common::test_db();

    // No experiments exist, anomaly-investigation with force=false should fail
    let metric_id = uuid::Uuid::new_v4().to_string();
    common::insert_test_metric(&conn, &metric_id, "Some Metric");

    let input = TemplateInput {
        template_key: "analytics-anomaly-investigation".to_string(),
        params: json!({
            "kpi_entity_id": metric_id,
            "time_window": "last_30_days",
            "baseline_period": "previous_quarter"
        }),
        force: false,
    };

    let result = run_template_full(&conn, &input);
    assert!(
        result.is_err(),
        "Should fail when experiment prerequisite not met and force=false"
    );
}

// =============================================================================
// Test 4: Provenance reconstruction
// =============================================================================

#[test]
fn test_provenance_reconstruction_metric_tree() {
    let conn = common::test_db();
    let mt_output = run_metric_tree(&conn);

    // Verify all entities traceable to run
    let entity_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
            params![mt_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        entity_count >= 5,
        "Expected at least 5 entities with provenance_run_id, got {}",
        entity_count
    );

    // Verify all relations traceable to run
    let relation_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE provenance_run_id = ?1",
            params![mt_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        relation_count >= 5,
        "Expected at least 5 relations with provenance_run_id, got {}",
        relation_count
    );

    // 100% coverage: total entities created by this run should match DB
    let total_entity_ops = mt_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.entity_id.is_some() && op.relation_id.is_none())
        .count();
    assert_eq!(
        entity_count, total_entity_ops,
        "Entity count in DB should match applied entity ops"
    );

    let total_relation_ops = mt_output
        .patch_result
        .applied
        .iter()
        .filter(|op| op.relation_id.is_some())
        .count();
    assert_eq!(
        relation_count, total_relation_ops,
        "Relation count in DB should match applied relation ops"
    );
}

#[test]
fn test_provenance_reconstruction_experiment_plan() {
    let conn = common::test_db();

    // Setup: create metrics first
    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);

    let ep_output = run_experiment_plan(&conn, &metric_id);

    // Verify experiment entity is traceable
    let entity_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
            params![ep_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(entity_count, 1, "experiment-plan should produce 1 entity");

    // Verify relations are traceable
    let relation_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE provenance_run_id = ?1",
            params![ep_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        relation_count, 2,
        "experiment-plan should produce 2 relations"
    );

    // Verify the relation types
    let mut stmt = conn
        .prepare(
            "SELECT relation_type FROM relations WHERE provenance_run_id = ?1 ORDER BY relation_type",
        )
        .unwrap();
    let rel_types: Vec<String> = stmt
        .query_map(params![ep_output.run_id], |row| row.get(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        rel_types.contains(&"tests".to_string()),
        "Should have 'tests' relation"
    );
    assert!(
        rel_types.contains(&"measures".to_string()),
        "Should have 'measures' relation"
    );
}

#[test]
fn test_provenance_reconstruction_anomaly_investigation() {
    let conn = common::test_db();

    // Setup: full chain
    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let _ep_output = run_experiment_plan(&conn, &metric_id);

    let ai_output = run_anomaly_investigation(&conn, &metric_id);

    // Verify result entity is traceable
    let entity_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
            params![ai_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        entity_count, 1,
        "anomaly-investigation should produce 1 entity"
    );

    // Verify relation is traceable
    let relation_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE provenance_run_id = ?1",
            params![ai_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        relation_count, 1,
        "anomaly-investigation should produce 1 relation"
    );

    // Verify relation type is evidence_for
    let rel_type: String = conn
        .query_row(
            "SELECT relation_type FROM relations WHERE provenance_run_id = ?1",
            params![ai_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(rel_type, "evidence_for");

    // Verify claim is traceable
    let claim_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM claims WHERE provenance_run_id = ?1",
            params![ai_output.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        claim_count, 1,
        "anomaly-investigation should produce 1 claim"
    );
}

#[test]
fn test_provenance_100_percent_coverage_full_chain() {
    let conn = common::test_db();

    // Run full chain
    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let ep_output = run_experiment_plan(&conn, &metric_id);
    let ai_output = run_anomaly_investigation(&conn, &metric_id);

    let run_ids = vec![&mt_output.run_id, &ep_output.run_id, &ai_output.run_id];

    // Every entity in the DB should have a provenance_run_id matching one of our runs
    let total_entities: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let mut provenance_entities: usize = 0;
    for run_id in &run_ids {
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
                params![run_id],
                |row| row.get(0),
            )
            .unwrap();
        provenance_entities += count;
    }
    assert_eq!(
        total_entities, provenance_entities,
        "100% of entities should be traceable to a run"
    );

    // Every relation should be traceable
    let total_relations: usize = conn
        .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
        .unwrap();

    let mut provenance_relations: usize = 0;
    for run_id in &run_ids {
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM relations WHERE provenance_run_id = ?1",
                params![run_id],
                |row| row.get(0),
            )
            .unwrap();
        provenance_relations += count;
    }
    assert_eq!(
        total_relations, provenance_relations,
        "100% of relations should be traceable to a run"
    );
}

// =============================================================================
// Test 5: Run logging
// =============================================================================

#[test]
fn test_run_logging_metric_tree() {
    let conn = common::test_db();
    let mt_output = run_metric_tree(&conn);

    // Verify run record exists
    let (template_key, template_category, status, inputs_snapshot): (
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT template_key, template_category, status, inputs_snapshot FROM runs WHERE run_id = ?1",
            params![mt_output.run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(template_key, "analytics-metric-tree");
    assert_eq!(template_category, "analytics");
    assert_eq!(status, "applied");

    let inputs: serde_json::Value = serde_json::from_str(&inputs_snapshot).unwrap();
    assert_eq!(inputs["business_model"], "SaaS");
    assert_eq!(inputs["primary_objective"], "Revenue Growth");
}

#[test]
fn test_run_logging_experiment_plan() {
    let conn = common::test_db();

    // Setup
    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let ep_output = run_experiment_plan(&conn, &metric_id);

    let (template_key, template_category, status, inputs_snapshot): (
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT template_key, template_category, status, inputs_snapshot FROM runs WHERE run_id = ?1",
            params![ep_output.run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(template_key, "analytics-experiment-plan");
    assert_eq!(template_category, "analytics");
    assert_eq!(status, "applied");

    let inputs: serde_json::Value = serde_json::from_str(&inputs_snapshot).unwrap();
    assert!(
        inputs["hypothesis"]
            .as_str()
            .unwrap()
            .contains("trial length"),
        "inputs_snapshot should contain the hypothesis"
    );
    assert_eq!(inputs["funnel_position"], "activation");
    assert_eq!(inputs["metric_entity_id"], metric_id);
}

#[test]
fn test_run_logging_anomaly_investigation() {
    let conn = common::test_db();

    // Setup: full chain
    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let _ep_output = run_experiment_plan(&conn, &metric_id);
    let ai_output = run_anomaly_investigation(&conn, &metric_id);

    let (template_key, template_category, status, inputs_snapshot): (
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT template_key, template_category, status, inputs_snapshot FROM runs WHERE run_id = ?1",
            params![ai_output.run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(template_key, "analytics-anomaly-investigation");
    assert_eq!(template_category, "analytics");
    assert_eq!(status, "applied");

    let inputs: serde_json::Value = serde_json::from_str(&inputs_snapshot).unwrap();
    assert_eq!(inputs["kpi_entity_id"], metric_id);
    assert_eq!(inputs["time_window"], "last_30_days");
    assert_eq!(inputs["baseline_period"], "previous_quarter");
}

#[test]
fn test_run_logging_each_run_has_unique_id() {
    let conn = common::test_db();

    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let ep_output = run_experiment_plan(&conn, &metric_id);
    let ai_output = run_anomaly_investigation(&conn, &metric_id);

    // All run IDs should be unique
    assert_ne!(mt_output.run_id, ep_output.run_id);
    assert_ne!(ep_output.run_id, ai_output.run_id);
    assert_ne!(mt_output.run_id, ai_output.run_id);

    // Total runs in DB should be 3
    let total: usize = conn
        .query_row("SELECT COUNT(*) FROM runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 3);
}

// =============================================================================
// Additional edge cases
// =============================================================================

#[test]
fn test_experiment_plan_with_nonexistent_metric() {
    let conn = common::test_db();

    // Insert a metric so prerequisite is met
    let real_metric_id = uuid::Uuid::new_v4().to_string();
    common::insert_test_metric(&conn, &real_metric_id, "Real Metric");

    // Try to create experiment referencing a nonexistent metric
    let input = TemplateInput {
        template_key: "analytics-experiment-plan".to_string(),
        params: json!({
            "hypothesis": "Test",
            "funnel_position": "activation",
            "metric_entity_id": "nonexistent-metric-id"
        }),
        force: false,
    };

    let result = run_template_full(&conn, &input);
    assert!(
        result.is_err(),
        "Should fail when metric_entity_id doesn't exist"
    );
}

#[test]
fn test_anomaly_investigation_with_nonexistent_kpi() {
    let conn = common::test_db();

    // Insert experiment so prerequisite is met
    let exp_id = uuid::Uuid::new_v4().to_string();
    common::insert_test_experiment(&conn, &exp_id, "Real Experiment");

    let input = TemplateInput {
        template_key: "analytics-anomaly-investigation".to_string(),
        params: json!({
            "kpi_entity_id": "nonexistent-kpi-id",
            "time_window": "last_30_days",
            "baseline_period": "previous_quarter"
        }),
        force: false,
    };

    let result = run_template_full(&conn, &input);
    assert!(
        result.is_err(),
        "Should fail when kpi_entity_id doesn't exist"
    );
}

#[test]
fn test_experiment_plan_entity_has_correct_canonical_fields() {
    let conn = common::test_db();

    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let ep_output = run_experiment_plan(&conn, &metric_id);

    let experiment_id = ep_output
        .patch_result
        .applied
        .iter()
        .find_map(|op| {
            if op.entity_id.is_some() && op.relation_id.is_none() {
                op.entity_id.clone()
            } else {
                None
            }
        })
        .unwrap();

    let canonical_fields_str: String = conn
        .query_row(
            "SELECT canonical_fields FROM entities WHERE id = ?1",
            params![experiment_id],
            |row| row.get(0),
        )
        .unwrap();

    let cf: serde_json::Value = serde_json::from_str(&canonical_fields_str).unwrap();
    assert!(
        cf["hypothesis"].as_str().unwrap().contains("trial length"),
        "canonical_fields should contain hypothesis"
    );
    assert_eq!(cf["funnel_position"], "activation");
}

#[test]
fn test_anomaly_investigation_result_has_correct_canonical_fields() {
    let conn = common::test_db();

    let mt_output = run_metric_tree(&conn);
    let metric_id = pick_metric_id(&mt_output);
    let _ep_output = run_experiment_plan(&conn, &metric_id);
    let ai_output = run_anomaly_investigation(&conn, &metric_id);

    let result_entity_id = ai_output
        .patch_result
        .applied
        .iter()
        .find_map(|op| {
            if op.entity_id.is_some() && op.relation_id.is_none() {
                op.entity_id.clone()
            } else {
                None
            }
        })
        .unwrap();

    let canonical_fields_str: String = conn
        .query_row(
            "SELECT canonical_fields FROM entities WHERE id = ?1",
            params![result_entity_id],
            |row| row.get(0),
        )
        .unwrap();

    let cf: serde_json::Value = serde_json::from_str(&canonical_fields_str).unwrap();
    assert_eq!(cf["findings"], "Investigation pending");
    assert_eq!(cf["methodology"], "time_series_comparison");
    assert_eq!(cf["confidence_level"], 0.0);
}

#[test]
fn test_template_definitions_exist_for_all_three() {
    let mt_def = get_template_definition("analytics-metric-tree");
    assert!(mt_def.is_some());
    let mt_def = mt_def.unwrap();
    assert_eq!(mt_def.category, "analytics");
    assert!(mt_def.prerequisites.is_empty());

    let ep_def = get_template_definition("analytics-experiment-plan");
    assert!(ep_def.is_some());
    let ep_def = ep_def.unwrap();
    assert_eq!(ep_def.category, "analytics");
    assert_eq!(ep_def.prerequisites.len(), 1);
    assert_eq!(ep_def.prerequisites[0].entity_type, "metric");

    let ai_def = get_template_definition("analytics-anomaly-investigation");
    assert!(ai_def.is_some());
    let ai_def = ai_def.unwrap();
    assert_eq!(ai_def.category, "analytics");
    assert_eq!(ai_def.prerequisites.len(), 1);
    assert_eq!(ai_def.prerequisites[0].entity_type, "experiment");
}

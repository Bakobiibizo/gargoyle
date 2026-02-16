// Load test: Run 5 experiment-plan templates referencing the same metric.
//
// Phase 6E: Verifies concurrent-style template execution, entity integrity,
// and run logging under repeated template invocations.

mod common;

use gargoyle_lib::services::template_runner::{run_template_full, TemplateInput, TemplateOutput};
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

#[test]
fn test_5_concurrent_experiment_plans() {
    let conn = common::test_db();

    // Step 1: Create a metric tree first (prerequisite for experiment-plan)
    let metric_output = run_metric_tree(&conn);

    // Get one metric entity_id from the result
    let metric_entity_id = pick_metric_id(&metric_output);

    // Step 2: Run 5 experiment-plan templates, all referencing the same metric
    let mut run_ids = Vec::new();
    for i in 0..5 {
        let input = TemplateInput {
            template_key: "analytics-experiment-plan".to_string(),
            params: json!({
                "hypothesis": format!("Hypothesis {}: Testing growth lever", i + 1),
                "funnel_position": "activation",
                "metric_entity_id": metric_entity_id
            }),
            force: false,
        };

        let output = run_template_full(&conn, &input).unwrap();
        assert!(!output.run_id.is_empty());
        assert!(
            output.patch_result.errors.is_empty(),
            "Run {} should have no errors: {:?}",
            i + 1,
            output.patch_result.errors
        );
        run_ids.push(output.run_id);
    }

    // Verify: All run IDs are unique
    let unique_count = {
        let mut sorted = run_ids.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };
    assert_eq!(unique_count, 5, "All 5 run IDs should be unique");

    // Verify: 5 experiment entities created (different experiments, no dedup)
    let exp_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'experiment' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(exp_count, 5, "Expected 5 experiments, got {}", exp_count);

    // Verify: 5 experiment-plan runs logged
    let run_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM runs WHERE template_key = 'analytics-experiment-plan'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(run_count, 5, "Expected 5 experiment-plan runs, got {}", run_count);

    // Verify: No entity corruption (all experiments have correct entity_type)
    let corrupt_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'experiment' AND deleted_at IS NULL AND source != 'template'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        corrupt_count, 0,
        "No experiment entities should have corrupted source"
    );

    // Verify: Each experiment has distinct provenance_run_id
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT provenance_run_id FROM entities WHERE entity_type = 'experiment' AND deleted_at IS NULL",
        )
        .unwrap();
    let provenance_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        provenance_ids.len(),
        5,
        "Each experiment should have a distinct provenance_run_id"
    );

    // Verify: Each experiment has relations back to the metric
    let rel_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE to_id = ?1",
            params![metric_entity_id],
            |row| row.get(0),
        )
        .unwrap();
    // Each experiment-plan creates 2 relations (tests + measures) to the metric
    assert_eq!(
        rel_count, 10,
        "Expected 10 relations to metric (2 per experiment), got {}",
        rel_count
    );
}

#[test]
fn test_concurrent_experiments_each_has_unique_title() {
    let conn = common::test_db();

    // Setup: create metrics
    let metric_output = run_metric_tree(&conn);
    let metric_entity_id = pick_metric_id(&metric_output);

    // Run 5 experiments with different hypotheses
    for i in 0..5 {
        let input = TemplateInput {
            template_key: "analytics-experiment-plan".to_string(),
            params: json!({
                "hypothesis": format!("Unique Hypothesis #{}: {}", i + 1,
                    ["Pricing affects churn", "Feature X boosts activation",
                     "Onboarding flow improves NPS", "Email cadence drives retention",
                     "Referral incentive increases virality"][i]),
                "funnel_position": "activation",
                "metric_entity_id": metric_entity_id
            }),
            force: false,
        };

        run_template_full(&conn, &input).unwrap();
    }

    // Verify each experiment has a unique title
    let mut stmt = conn
        .prepare(
            "SELECT title FROM entities WHERE entity_type = 'experiment' AND deleted_at IS NULL ORDER BY title",
        )
        .unwrap();
    let titles: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(titles.len(), 5);

    // Check all titles are unique
    let unique_titles = {
        let mut sorted = titles.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };
    assert_eq!(
        unique_titles, 5,
        "All 5 experiment titles should be unique"
    );
}

#[test]
fn test_concurrent_experiments_run_status_all_applied() {
    let conn = common::test_db();

    // Setup: create metrics
    let metric_output = run_metric_tree(&conn);
    let metric_entity_id = pick_metric_id(&metric_output);

    // Run 5 experiments
    for i in 0..5 {
        let input = TemplateInput {
            template_key: "analytics-experiment-plan".to_string(),
            params: json!({
                "hypothesis": format!("Status Test Hypothesis {}", i + 1),
                "funnel_position": "activation",
                "metric_entity_id": metric_entity_id
            }),
            force: false,
        };

        run_template_full(&conn, &input).unwrap();
    }

    // All runs should have status = 'applied'
    let non_applied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM runs WHERE template_key = 'analytics-experiment-plan' AND status != 'applied'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        non_applied, 0,
        "All experiment-plan runs should have status 'applied'"
    );
}

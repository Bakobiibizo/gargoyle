// Load test: Run analytics-metric-tree 10 times with varied inputs.
//
// Phase 6E: Verifies graph growth under load, projection rebuild performance,
// and related_to ratio stays healthy.

mod common;

use gargoyle_lib::services::graph_builder;
use gargoyle_lib::services::template_runner::{run_template_full, TemplateInput};
use serde_json::json;

#[test]
fn test_graph_growth_10_metric_trees() {
    let conn = common::test_db();

    let business_models = vec![
        "SaaS",
        "E-commerce",
        "Marketplace",
        "FinTech",
        "Healthcare",
        "EdTech",
        "Gaming",
        "Media",
        "IoT",
        "Enterprise",
    ];

    for (i, model) in business_models.iter().enumerate() {
        let input = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({
                "business_model": model,
                "primary_objective": format!("Growth Strategy {}", i + 1),
                "customer_journey": "Acquisition -> Activation -> Revenue -> Retention -> Referral"
            }),
            force: false,
        };

        let output = run_template_full(&conn, &input).unwrap();
        assert!(!output.run_id.is_empty(), "Run {} should produce a run_id", i + 1);
    }

    // Verify: ~50-100 metric entities created (7 per run x 10 runs = 70)
    let entity_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        entity_count >= 50,
        "Expected 50+ metrics, got {}",
        entity_count
    );

    // Verify: 10 runs logged
    let run_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM runs WHERE template_key = 'analytics-metric-tree'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(run_count, 10, "Expected 10 runs, got {}", run_count);

    // Verify: Graph projection rebuilds in < 1 second
    let start = std::time::Instant::now();
    let stats = graph_builder::rebuild_projection(&conn).unwrap();
    let duration = start.elapsed();
    assert!(
        duration.as_secs() < 1,
        "Projection rebuild took {:?}",
        duration
    );

    // Verify projection stats are consistent
    assert_eq!(
        stats.total_entities as i64, entity_count,
        "Projection entity count should match DB count"
    );
    assert!(
        stats.total_relations > 0,
        "Should have relations after 10 metric tree runs"
    );

    // Verify: related_to ratio stays below 20%
    // (templates produce typed relations like "measures", not "related_to")
    let audit = graph_builder::audit_related_to(&conn).unwrap();
    assert!(
        !audit.threshold_exceeded,
        "related_to ratio {:.1}% exceeds 20% threshold",
        audit.ratio * 100.0
    );
}

#[test]
fn test_graph_growth_entity_type_consistency() {
    let conn = common::test_db();

    // Run 5 metric trees with different business models
    let models = vec!["SaaS", "Marketplace", "FinTech", "Gaming", "IoT"];

    for model in &models {
        let input = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({
                "business_model": model,
                "primary_objective": format!("{} Growth", model),
            }),
            force: false,
        };

        let output = run_template_full(&conn, &input).unwrap();
        assert!(output.patch_result.errors.is_empty());
    }

    // Every entity should be of type "metric" (metric-tree only creates metrics)
    let non_metric_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type != 'metric' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        non_metric_count, 0,
        "All entities from metric-tree should be type 'metric', found {} non-metric",
        non_metric_count
    );

    // Every entity should have source = 'template'
    let non_template_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE source != 'template' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        non_template_count, 0,
        "All entities should have source='template', found {} non-template",
        non_template_count
    );

    // All relations should be "measures" (metric-tree produces measures relations)
    let non_measures_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE relation_type != 'measures'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        non_measures_count, 0,
        "All relations from metric-tree should be 'measures', found {} non-measures",
        non_measures_count
    );
}

#[test]
fn test_graph_growth_projection_stats_scale() {
    let conn = common::test_db();

    // Run 3 metric trees and verify stats grow linearly
    let mut prev_entities = 0usize;
    let mut prev_relations = 0usize;

    for i in 0..3 {
        let input = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({
                "business_model": format!("Model_{}", i),
                "primary_objective": format!("Objective_{}", i),
            }),
            force: false,
        };

        run_template_full(&conn, &input).unwrap();

        let stats = graph_builder::rebuild_projection(&conn).unwrap();
        assert!(
            stats.total_entities > prev_entities,
            "Entity count should grow: {} -> {}",
            prev_entities,
            stats.total_entities
        );
        assert!(
            stats.total_relations > prev_relations,
            "Relation count should grow: {} -> {}",
            prev_relations,
            stats.total_relations
        );

        prev_entities = stats.total_entities;
        prev_relations = stats.total_relations;
    }
}

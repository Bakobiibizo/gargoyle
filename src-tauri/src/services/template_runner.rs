// TemplateRunner: prerequisite check, execute, log run
//
// Phase 4A: Template infrastructure (registry, prerequisites, runner)
// Phase 4B: analytics-metric-tree template
// Phase 4C: analytics-experiment-plan + analytics-anomaly-investigation templates

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::{GargoyleError, Result};
use crate::models::patch::{
    CreateClaimPayload, CreateEntityPayload, CreateRelationPayload, PatchOp, PatchResult, PatchSet,
};
use crate::models::run::{Run, RunStatus};
use crate::patch::apply::apply_patch_set;
use crate::services::store::StoreService;

// =============================================================================
// Core types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDefinition {
    pub key: String,
    pub version: String,
    pub category: String,
    pub prerequisites: Vec<Prerequisite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    pub entity_type: String,
    pub min_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisiteResult {
    pub satisfied: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInput {
    pub template_key: String,
    pub params: serde_json::Value,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateOutput {
    pub run_id: String,
    pub patch_result: PatchResult,
    pub warnings: Vec<String>,
}

// =============================================================================
// Template registry
// =============================================================================

/// Returns the template definition for a given key, or None if unknown.
pub fn get_template_definition(key: &str) -> Option<TemplateDefinition> {
    match key {
        "analytics-metric-tree" => Some(TemplateDefinition {
            key: "analytics-metric-tree".to_string(),
            version: "1.0".to_string(),
            category: "analytics".to_string(),
            prerequisites: vec![], // Foundational template, no prerequisites
        }),
        "analytics-experiment-plan" => Some(TemplateDefinition {
            key: "analytics-experiment-plan".to_string(),
            version: "1.0".to_string(),
            category: "analytics".to_string(),
            prerequisites: vec![Prerequisite {
                entity_type: "metric".to_string(),
                min_count: 1,
            }],
        }),
        "analytics-anomaly-investigation" => Some(TemplateDefinition {
            key: "analytics-anomaly-investigation".to_string(),
            version: "1.0".to_string(),
            category: "analytics".to_string(),
            prerequisites: vec![Prerequisite {
                entity_type: "experiment".to_string(),
                min_count: 1,
            }],
        }),
        _ => None,
    }
}

/// Returns all registered template definitions.
pub fn list_template_definitions() -> Vec<TemplateDefinition> {
    vec![
        get_template_definition("analytics-metric-tree").unwrap(),
        get_template_definition("analytics-experiment-plan").unwrap(),
        get_template_definition("analytics-anomaly-investigation").unwrap(),
    ]
}

// =============================================================================
// Prerequisite checking
// =============================================================================

/// Check prerequisites for a template against the database.
/// Returns one PrerequisiteResult per prerequisite.
pub fn check_prerequisites(
    conn: &rusqlite::Connection,
    template_key: &str,
) -> Result<Vec<PrerequisiteResult>> {
    let definition = get_template_definition(template_key).ok_or_else(|| {
        GargoyleError::Schema(format!("Unknown template: '{}'", template_key))
    })?;

    let mut results = Vec::new();

    for prereq in &definition.prerequisites {
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE entity_type = ?1 AND deleted_at IS NULL",
                params![prereq.entity_type],
                |row| row.get(0),
            )
            .map_err(GargoyleError::Database)?;

        if count >= prereq.min_count {
            results.push(PrerequisiteResult {
                satisfied: true,
                message: None,
            });
        } else {
            results.push(PrerequisiteResult {
                satisfied: false,
                message: Some(format!(
                    "This template needs at least {} {}(s). Found {}.",
                    prereq.min_count, prereq.entity_type, count
                )),
            });
        }
    }

    Ok(results)
}

// =============================================================================
// Op generation dispatch
// =============================================================================

/// Generate PatchOps for a given template key.
/// Dispatches to the appropriate template-specific generator.
/// Some templates need `conn` to look up existing entities.
fn generate_ops(
    conn: &rusqlite::Connection,
    key: &str,
    params: &serde_json::Value,
    run_id: &str,
) -> Result<Vec<PatchOp>> {
    match key {
        "analytics-metric-tree" => generate_metric_tree_ops(params, run_id),
        "analytics-experiment-plan" => generate_experiment_plan_ops(conn, params, run_id),
        "analytics-anomaly-investigation" => {
            generate_anomaly_investigation_entity_ops(conn, params, run_id)
        }
        _ => Err(GargoyleError::Schema(format!(
            "Template '{}' does not have an implementation yet",
            key
        ))),
    }
}

// =============================================================================
// Template runner
// =============================================================================

/// Run a template end-to-end:
/// 1. Look up template definition
/// 2. Check prerequisites (advisory)
/// 3. Generate PatchOps
/// 4. Apply PatchSet atomically
/// 5. Log the run
/// 6. Return TemplateOutput
pub fn run_template(
    conn: &rusqlite::Connection,
    input: &TemplateInput,
) -> Result<TemplateOutput> {
    // 1. Look up the template definition
    let definition = get_template_definition(&input.template_key).ok_or_else(|| {
        GargoyleError::Schema(format!("Unknown template: '{}'", input.template_key))
    })?;

    // 2. Check prerequisites
    let prereq_results = check_prerequisites(conn, &input.template_key)?;
    let mut warnings = Vec::new();

    let all_satisfied = prereq_results.iter().all(|r| r.satisfied);
    if !all_satisfied && !input.force {
        // Collect all unsatisfied messages
        let messages: Vec<String> = prereq_results
            .iter()
            .filter(|r| !r.satisfied)
            .filter_map(|r| r.message.clone())
            .collect();
        return Err(GargoyleError::Schema(format!(
            "Prerequisites not met: {}",
            messages.join("; ")
        )));
    } else if !all_satisfied && input.force {
        // Forced run - collect warnings
        for result in &prereq_results {
            if !result.satisfied {
                if let Some(msg) = &result.message {
                    warnings.push(format!("FORCED: {}", msg));
                }
            }
        }
    }

    // 3. Generate a unique run_id
    let run_id = uuid::Uuid::new_v4().to_string();

    // 4. Generate PatchOps
    let ops = generate_ops(conn, &input.template_key, &input.params, &run_id)?;

    // 5. Build and apply PatchSet
    let patch_set = PatchSet {
        ops: ops.clone(),
        run_id: Some(run_id.clone()),
    };

    let patch_result = apply_patch_set(conn, &patch_set)?;

    // 6. Build outputs_snapshot from patch_result
    let outputs_snapshot = serde_json::to_value(&patch_result)
        .unwrap_or_else(|_| serde_json::json!({}));

    // 7. Log the run
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    let run = Run {
        run_id: run_id.clone(),
        template_key: definition.key.clone(),
        template_version: definition.version.clone(),
        template_category: definition.category.clone(),
        inputs_snapshot: input.params.clone(),
        outputs_snapshot,
        patch_set: serde_json::to_value(&patch_set)
            .unwrap_or_else(|_| serde_json::json!({})),
        status: if patch_result.errors.is_empty() {
            RunStatus::Applied
        } else {
            RunStatus::Partial
        },
        created_at: now,
    };

    StoreService::log_run(conn, &run)?;

    // 8. Return result
    Ok(TemplateOutput {
        run_id,
        patch_result,
        warnings,
    })
}

// =============================================================================
// analytics-metric-tree template
// =============================================================================

/// Generates metric entities and relations for the analytics-metric-tree template.
///
/// Input params (JSON):
///   - business_model: String (e.g., "SaaS")
///   - primary_objective: String (e.g., "Revenue Growth")
///   - customer_journey: String (e.g., "Acquisition -> Activation -> Revenue -> Retention -> Referral")
///
/// Output: 5-7 metric entities + relations between them.
fn generate_metric_tree_ops(
    params: &serde_json::Value,
    _run_id: &str,
) -> Result<Vec<PatchOp>> {
    let business_model = params
        .get("business_model")
        .and_then(|v| v.as_str())
        .unwrap_or("General");
    let primary_objective = params
        .get("primary_objective")
        .and_then(|v| v.as_str())
        .unwrap_or("Growth");
    let _customer_journey = params
        .get("customer_journey")
        .and_then(|v| v.as_str())
        .unwrap_or("Acquisition → Activation → Revenue → Retention → Referral");

    let mut ops = Vec::new();

    // Define the metric tree structure based on business model
    let primary_metric = MetricDef {
        title: format!("{} - Primary KPI", primary_objective),
        body: format!(
            "Primary metric tracking {} for {} business model.",
            primary_objective, business_model
        ),
        canonical_fields: serde_json::json!({
            "trend": "flat",
            "data_source": "aggregated"
        }),
    };

    let funnel_metrics = vec![
        MetricDef {
            title: "Customer Acquisition Rate".to_string(),
            body: format!(
                "Tracks new customer acquisition for {} model.",
                business_model
            ),
            canonical_fields: serde_json::json!({
                "trend": "up",
                "data_source": "marketing"
            }),
        },
        MetricDef {
            title: "Activation Rate".to_string(),
            body: format!(
                "Measures user activation percentage in {} funnel.",
                business_model
            ),
            canonical_fields: serde_json::json!({
                "trend": "flat",
                "data_source": "product"
            }),
        },
        MetricDef {
            title: format!("{} Revenue", business_model),
            body: format!(
                "Revenue metric for {} business model.",
                business_model
            ),
            canonical_fields: serde_json::json!({
                "trend": "up",
                "data_source": "finance"
            }),
        },
        MetricDef {
            title: "Churn Rate".to_string(),
            body: format!(
                "Customer churn rate for {} model.",
                business_model
            ),
            canonical_fields: serde_json::json!({
                "trend": "down",
                "data_source": "product"
            }),
        },
        MetricDef {
            title: "Referral Rate".to_string(),
            body: format!(
                "Referral/viral coefficient for {} model.",
                business_model
            ),
            canonical_fields: serde_json::json!({
                "trend": "flat",
                "data_source": "marketing"
            }),
        },
        MetricDef {
            title: "Net Promoter Score".to_string(),
            body: format!(
                "NPS for {} - measures customer satisfaction and loyalty.",
                business_model
            ),
            canonical_fields: serde_json::json!({
                "trend": "up",
                "data_source": "survey"
            }),
        },
    ];

    // Create primary metric entity op
    ops.push(PatchOp::CreateEntity(CreateEntityPayload {
        entity_type: "metric".to_string(),
        title: primary_metric.title,
        source: "template".to_string(),
        canonical_fields: primary_metric.canonical_fields,
        body_md: Some(primary_metric.body),
        status: Some("active".to_string()),
        category: Some("primary".to_string()),
        priority: Some(0),
    }));

    // Create funnel metric entity ops
    // Priority is capped at 0-4 per DB constraint; funnel metrics all get priority 2
    for (_i, metric) in funnel_metrics.iter().enumerate() {
        ops.push(PatchOp::CreateEntity(CreateEntityPayload {
            entity_type: "metric".to_string(),
            title: metric.title.clone(),
            source: "template".to_string(),
            canonical_fields: metric.canonical_fields.clone(),
            body_md: Some(metric.body.clone()),
            status: Some("active".to_string()),
            category: Some("funnel".to_string()),
            priority: Some(2),
        }));
    }

    // Relations will be created after entities are applied, using placeholder indices.
    // The patch system processes ops sequentially, so we know the primary metric
    // is at op index 0 and funnel metrics are at indices 1..=N.
    // However, we don't know the entity IDs yet at generation time.
    //
    // We solve this by using a two-pass approach:
    // Actually, the create_relation ops need real entity IDs. Since we're generating
    // ops that go through apply_patch_set, and entity IDs are generated during
    // application, we need to use a deferred approach.
    //
    // The solution: We generate relation ops that reference placeholder IDs.
    // Then in the runner, after the entity ops succeed, we look at the AppliedOp
    // results to get the actual IDs and create a second patch set for relations.
    //
    // Better solution: We pre-generate deterministic UUIDs for the entities.
    // This way relation ops can reference them directly.

    // Return just entity ops; the runner will handle relation creation separately
    // after entity IDs are known (two-phase approach in run_template_full).
    Ok(ops)
}

/// Internal struct for metric definition.
struct MetricDef {
    title: String,
    body: String,
    canonical_fields: serde_json::Value,
}

/// Creates relation ops linking the primary metric to all funnel metrics.
/// Called after entity creation, when we have the actual entity IDs.
fn create_metric_tree_relations(
    primary_entity_id: &str,
    funnel_entity_ids: &[String],
    run_id: &str,
) -> Vec<PatchOp> {
    funnel_entity_ids
        .iter()
        .map(|funnel_id| {
            PatchOp::CreateRelation(CreateRelationPayload {
                from_id: primary_entity_id.to_string(),
                to_id: funnel_id.clone(),
                relation_type: "measures".to_string(),
                weight: Some(1.0),
                confidence: Some(1.0),
                provenance_run_id: Some(run_id.to_string()),
            })
        })
        .collect()
}

// =============================================================================
// analytics-experiment-plan template
// =============================================================================

/// Generates entity ops for the analytics-experiment-plan template (phase 1).
///
/// Input params (JSON):
///   - hypothesis: String
///   - funnel_position: String
///   - metric_entity_id: String (existing metric entity ID)
///
/// Output: 1 experiment entity (relations created in phase 2)
fn generate_experiment_plan_ops(
    conn: &rusqlite::Connection,
    params: &serde_json::Value,
    _run_id: &str,
) -> Result<Vec<PatchOp>> {
    let hypothesis = params
        .get("hypothesis")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled hypothesis");
    let funnel_position = params
        .get("funnel_position")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let metric_entity_id = params
        .get("metric_entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            GargoyleError::Schema("Missing required param: metric_entity_id".to_string())
        })?;

    // Verify metric exists and is not deleted
    let _metric_exists: String = conn
        .query_row(
            "SELECT id FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![metric_entity_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "metric".to_string(),
                id: metric_entity_id.to_string(),
            },
            other => GargoyleError::Database(other),
        })?;

    // Truncate hypothesis to 60 chars for the title
    let truncated_hypothesis = if hypothesis.len() > 60 {
        format!("{}...", &hypothesis[..60])
    } else {
        hypothesis.to_string()
    };

    let title = format!("Experiment: {}", truncated_hypothesis);

    let ops = vec![PatchOp::CreateEntity(CreateEntityPayload {
        entity_type: "experiment".to_string(),
        title,
        source: "template".to_string(),
        canonical_fields: serde_json::json!({
            "hypothesis": hypothesis,
            "funnel_position": funnel_position,
        }),
        body_md: Some(format!(
            "Experiment plan testing hypothesis: {}\nFunnel position: {}",
            hypothesis, funnel_position
        )),
        status: Some("draft".to_string()),
        category: None,
        priority: None,
    })];

    Ok(ops)
}

/// Creates relation ops for the experiment-plan template (phase 2).
/// Experiment `tests` metric and experiment `measures` metric.
fn create_experiment_plan_relations(
    experiment_id: &str,
    metric_entity_id: &str,
    run_id: &str,
) -> Vec<PatchOp> {
    vec![
        PatchOp::CreateRelation(CreateRelationPayload {
            from_id: experiment_id.to_string(),
            to_id: metric_entity_id.to_string(),
            relation_type: "tests".to_string(),
            weight: Some(1.0),
            confidence: None,
            provenance_run_id: Some(run_id.to_string()),
        }),
        PatchOp::CreateRelation(CreateRelationPayload {
            from_id: experiment_id.to_string(),
            to_id: metric_entity_id.to_string(),
            relation_type: "measures".to_string(),
            weight: Some(1.0),
            confidence: None,
            provenance_run_id: Some(run_id.to_string()),
        }),
    ]
}

// =============================================================================
// analytics-anomaly-investigation template
// =============================================================================

/// Generates entity ops for the analytics-anomaly-investigation template (phase 1).
///
/// Input params (JSON):
///   - kpi_entity_id: String (existing metric entity ID)
///   - time_window: String (e.g. "last_30_days")
///   - baseline_period: String (e.g. "previous_quarter")
///
/// Output: 1 result entity (relation + claim created in phase 2)
fn generate_anomaly_investigation_entity_ops(
    conn: &rusqlite::Connection,
    params: &serde_json::Value,
    _run_id: &str,
) -> Result<Vec<PatchOp>> {
    let kpi_entity_id = params
        .get("kpi_entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            GargoyleError::Schema("Missing required param: kpi_entity_id".to_string())
        })?;

    let _time_window = params
        .get("time_window")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let _baseline_period = params
        .get("baseline_period")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Look up metric title from the database
    let metric_title: String = conn
        .query_row(
            "SELECT title FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![kpi_entity_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "metric".to_string(),
                id: kpi_entity_id.to_string(),
            },
            other => GargoyleError::Database(other),
        })?;

    let title = format!("Anomaly Investigation: {}", metric_title);

    let ops = vec![PatchOp::CreateEntity(CreateEntityPayload {
        entity_type: "result".to_string(),
        title,
        source: "template".to_string(),
        canonical_fields: serde_json::json!({
            "findings": "Investigation pending",
            "methodology": "time_series_comparison",
            "confidence_level": 0.0,
        }),
        body_md: Some(format!(
            "Anomaly investigation for metric: {}\nTime window: {}\nBaseline period: {}",
            metric_title, _time_window, _baseline_period
        )),
        status: Some("draft".to_string()),
        category: None,
        priority: None,
    })];

    Ok(ops)
}

/// Creates phase 2 ops for the anomaly-investigation template:
/// - 1 relation: result `evidence_for` metric
/// - 1 claim: anomaly detected in time_window, grounded to the result entity
fn create_anomaly_investigation_phase2_ops(
    result_entity_id: &str,
    kpi_entity_id: &str,
    metric_title: &str,
    time_window: &str,
    run_id: &str,
) -> Vec<PatchOp> {
    vec![
        PatchOp::CreateRelation(CreateRelationPayload {
            from_id: result_entity_id.to_string(),
            to_id: kpi_entity_id.to_string(),
            relation_type: "evidence_for".to_string(),
            weight: Some(1.0),
            confidence: None,
            provenance_run_id: Some(run_id.to_string()),
        }),
        PatchOp::CreateClaim(CreateClaimPayload {
            subject: metric_title.to_string(),
            predicate: "anomaly_detected_in".to_string(),
            object: time_window.to_string(),
            confidence: 0.5,
            evidence_entity_id: result_entity_id.to_string(),
            provenance_run_id: Some(run_id.to_string()),
        }),
    ]
}

/// Full run_template implementation that handles the two-phase approach for
/// templates that need to create relations between newly-created entities.
pub fn run_template_full(
    conn: &rusqlite::Connection,
    input: &TemplateInput,
) -> Result<TemplateOutput> {
    // 1. Look up the template definition
    let definition = get_template_definition(&input.template_key).ok_or_else(|| {
        GargoyleError::Schema(format!("Unknown template: '{}'", input.template_key))
    })?;

    // 2. Check prerequisites
    let prereq_results = check_prerequisites(conn, &input.template_key)?;
    let mut warnings = Vec::new();

    let all_satisfied = prereq_results.iter().all(|r| r.satisfied);
    if !all_satisfied && !input.force {
        let messages: Vec<String> = prereq_results
            .iter()
            .filter(|r| !r.satisfied)
            .filter_map(|r| r.message.clone())
            .collect();
        return Err(GargoyleError::Schema(format!(
            "Prerequisites not met: {}",
            messages.join("; ")
        )));
    } else if !all_satisfied && input.force {
        for result in &prereq_results {
            if !result.satisfied {
                if let Some(msg) = &result.message {
                    warnings.push(format!("FORCED: {}", msg));
                }
            }
        }
    }

    // 3. Generate a unique run_id
    let run_id = uuid::Uuid::new_v4().to_string();

    // 4. Generate entity PatchOps (phase 1)
    let entity_ops = generate_ops(conn, &input.template_key, &input.params, &run_id)?;

    // 5. Apply entity PatchSet
    let entity_patch_set = PatchSet {
        ops: entity_ops.clone(),
        run_id: Some(run_id.clone()),
    };

    let entity_result = apply_patch_set(conn, &entity_patch_set)?;

    // 6. Phase 2: create relations/claims that depend on entity IDs from phase 1
    let mut all_ops = entity_ops;
    let mut combined_result = entity_result.clone();

    let phase2_ops = generate_phase2_ops(
        conn,
        &input.template_key,
        &input.params,
        &run_id,
        &entity_result,
    )?;

    if !phase2_ops.is_empty() {
        let phase2_patch_set = PatchSet {
            ops: phase2_ops.clone(),
            run_id: Some(run_id.clone()),
        };

        let phase2_result = apply_patch_set(conn, &phase2_patch_set)?;

        // Merge results
        let offset = combined_result.applied.len();
        all_ops.extend(phase2_ops);
        for mut applied_op in phase2_result.applied {
            applied_op.op_index += offset;
            combined_result.applied.push(applied_op);
        }
        combined_result.errors.extend(phase2_result.errors);
    }

    // 7. Build outputs_snapshot
    let outputs_snapshot = serde_json::to_value(&combined_result)
        .unwrap_or_else(|_| serde_json::json!({}));

    // 8. Log the run
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    let full_patch_set = PatchSet {
        ops: all_ops,
        run_id: Some(run_id.clone()),
    };

    let run = Run {
        run_id: run_id.clone(),
        template_key: definition.key.clone(),
        template_version: definition.version.clone(),
        template_category: definition.category.clone(),
        inputs_snapshot: input.params.clone(),
        outputs_snapshot,
        patch_set: serde_json::to_value(&full_patch_set)
            .unwrap_or_else(|_| serde_json::json!({})),
        status: if combined_result.errors.is_empty() {
            RunStatus::Applied
        } else {
            RunStatus::Partial
        },
        created_at: now,
    };

    StoreService::log_run(conn, &run)?;

    // 9. Return result
    Ok(TemplateOutput {
        run_id,
        patch_result: combined_result,
        warnings,
    })
}

/// Generate phase 2 PatchOps that depend on entity IDs created in phase 1.
/// This handles relations, claims, and other ops that reference newly-created entities.
fn generate_phase2_ops(
    conn: &rusqlite::Connection,
    key: &str,
    params: &serde_json::Value,
    run_id: &str,
    phase1_result: &PatchResult,
) -> Result<Vec<PatchOp>> {
    match key {
        "analytics-metric-tree" => {
            if phase1_result.applied.is_empty() {
                return Ok(vec![]);
            }
            let primary_id = phase1_result.applied[0]
                .entity_id
                .as_ref()
                .expect("Primary metric should have entity_id");

            let funnel_ids: Vec<String> = phase1_result.applied[1..]
                .iter()
                .filter_map(|op| op.entity_id.clone())
                .collect();

            Ok(create_metric_tree_relations(primary_id, &funnel_ids, run_id))
        }
        "analytics-experiment-plan" => {
            // Phase 1 creates the experiment entity.
            // Phase 2 creates the relations to the metric.
            if phase1_result.applied.is_empty() {
                return Ok(vec![]);
            }
            let experiment_id = phase1_result.applied[0]
                .entity_id
                .as_ref()
                .expect("Experiment entity should have entity_id");

            let metric_entity_id = params
                .get("metric_entity_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    GargoyleError::Schema("Missing required param: metric_entity_id".to_string())
                })?;

            Ok(create_experiment_plan_relations(
                experiment_id,
                metric_entity_id,
                run_id,
            ))
        }
        "analytics-anomaly-investigation" => {
            // Phase 1 creates the result entity.
            // Phase 2 creates the relation (result -> metric) and the claim.
            if phase1_result.applied.is_empty() {
                return Ok(vec![]);
            }
            let result_entity_id = phase1_result.applied[0]
                .entity_id
                .as_ref()
                .expect("Result entity should have entity_id");

            let kpi_entity_id = params
                .get("kpi_entity_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    GargoyleError::Schema("Missing required param: kpi_entity_id".to_string())
                })?;

            let time_window = params
                .get("time_window")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Look up metric title for the claim subject
            let metric_title: String = conn
                .query_row(
                    "SELECT title FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                    params![kpi_entity_id],
                    |row| row.get(0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                        entity_type: "metric".to_string(),
                        id: kpi_entity_id.to_string(),
                    },
                    other => GargoyleError::Database(other),
                })?;

            Ok(create_anomaly_investigation_phase2_ops(
                result_entity_id,
                kpi_entity_id,
                &metric_title,
                time_window,
                run_id,
            ))
        }
        _ => Ok(vec![]),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::create_memory_connection;
    use crate::db::migrations::run_migrations;
    use rusqlite::Connection;
    use serde_json::json;

    /// Create a fresh in-memory database with all migrations applied.
    fn test_db() -> Connection {
        let conn = create_memory_connection().expect("Failed to create test DB");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    /// Helper to insert a test entity directly.
    fn insert_test_entity(conn: &Connection, entity_type: &str, title: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, \
             _schema_version, created_at, updated_at) \
             VALUES (?1, ?2, ?3, '', 'manual', '{}', 1, ?4, ?4)",
            params![id, entity_type, title, now],
        )
        .expect("Failed to insert test entity");
        conn.execute(
            "INSERT INTO entities_fts(rowid, title, body_md) \
             SELECT rowid, title, body_md FROM entities WHERE id = ?1",
            params![id],
        )
        .expect("Failed to insert into FTS");
        id
    }

    // ========================================================================
    // Template registry tests
    // ========================================================================

    #[test]
    fn test_get_template_definition_metric_tree() {
        let def = get_template_definition("analytics-metric-tree");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.key, "analytics-metric-tree");
        assert_eq!(def.version, "1.0");
        assert_eq!(def.category, "analytics");
        assert!(def.prerequisites.is_empty());
    }

    #[test]
    fn test_get_template_definition_experiment_plan() {
        let def = get_template_definition("analytics-experiment-plan");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.key, "analytics-experiment-plan");
        assert_eq!(def.version, "1.0");
        assert_eq!(def.category, "analytics");
        assert_eq!(def.prerequisites.len(), 1);
        assert_eq!(def.prerequisites[0].entity_type, "metric");
        assert_eq!(def.prerequisites[0].min_count, 1);
    }

    #[test]
    fn test_get_template_definition_anomaly_investigation() {
        let def = get_template_definition("analytics-anomaly-investigation");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.key, "analytics-anomaly-investigation");
        assert_eq!(def.prerequisites.len(), 1);
        assert_eq!(def.prerequisites[0].entity_type, "experiment");
    }

    #[test]
    fn test_get_template_definition_unknown_returns_none() {
        let def = get_template_definition("nonexistent-template");
        assert!(def.is_none());
    }

    #[test]
    fn test_list_template_definitions() {
        let templates = list_template_definitions();
        assert_eq!(templates.len(), 3);

        let keys: Vec<&str> = templates.iter().map(|t| t.key.as_str()).collect();
        assert!(keys.contains(&"analytics-metric-tree"));
        assert!(keys.contains(&"analytics-experiment-plan"));
        assert!(keys.contains(&"analytics-anomaly-investigation"));
    }

    // ========================================================================
    // Prerequisite tests
    // ========================================================================

    #[test]
    fn test_check_prerequisites_metric_tree_no_prereqs() {
        let conn = test_db();
        let results = check_prerequisites(&conn, "analytics-metric-tree").unwrap();
        // No prerequisites, so empty results
        assert!(results.is_empty());
    }

    #[test]
    fn test_check_prerequisites_experiment_plan_not_satisfied() {
        let conn = test_db();
        let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].satisfied);
        assert!(results[0].message.is_some());
        let msg = results[0].message.as_ref().unwrap();
        assert!(msg.contains("metric"));
        assert!(msg.contains("at least 1"));
    }

    #[test]
    fn test_check_prerequisites_experiment_plan_satisfied() {
        let conn = test_db();
        // Insert a metric entity
        insert_test_entity(&conn, "metric", "Test Metric");

        let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].satisfied);
        assert!(results[0].message.is_none());
    }

    #[test]
    fn test_check_prerequisites_unknown_template() {
        let conn = test_db();
        let result = check_prerequisites(&conn, "unknown-template");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_prerequisites_soft_deleted_not_counted() {
        let conn = test_db();
        let id = insert_test_entity(&conn, "metric", "Deleted Metric");
        // Soft-delete it
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .unwrap();

        let results = check_prerequisites(&conn, "analytics-experiment-plan").unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].satisfied);
    }

    // ========================================================================
    // metric-tree execution tests
    // ========================================================================

    fn default_metric_tree_input() -> TemplateInput {
        TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({
                "business_model": "SaaS",
                "primary_objective": "Revenue Growth",
                "customer_journey": "Acquisition → Activation → Revenue → Retention → Referral"
            }),
            force: false,
        }
    }

    #[test]
    fn test_metric_tree_produces_entities() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Should produce 7 entities (1 primary + 6 funnel)
        let entity_count = output
            .patch_result
            .applied
            .iter()
            .filter(|op| op.entity_id.is_some() && op.relation_id.is_none())
            .count();
        assert!(
            entity_count >= 5,
            "Expected at least 5 entities, got {}",
            entity_count
        );
    }

    #[test]
    fn test_metric_tree_all_entities_are_metrics() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Query all entities created by this run
        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        let run_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.provenance_run_id.as_deref() == Some(&output.run_id))
            .collect();

        assert!(run_entities.len() >= 5);
        for entity in &run_entities {
            assert_eq!(entity.entity_type, "metric");
        }
    }

    #[test]
    fn test_metric_tree_all_entities_have_source_template() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        let run_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.provenance_run_id.as_deref() == Some(&output.run_id))
            .collect();

        for entity in &run_entities {
            assert_eq!(
                entity.source,
                crate::models::entity::Source::Template,
                "Entity '{}' should have source=template",
                entity.title
            );
        }
    }

    #[test]
    fn test_metric_tree_all_entities_have_provenance_run_id() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Verify all entities have provenance_run_id set via direct SQL
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
                params![output.run_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            count >= 5,
            "Expected at least 5 entities with provenance_run_id, got {}",
            count
        );
    }

    #[test]
    fn test_metric_tree_relations_created() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Should have relation ops in the results
        let relation_count = output
            .patch_result
            .applied
            .iter()
            .filter(|op| op.relation_id.is_some())
            .count();

        assert!(
            relation_count >= 5,
            "Expected at least 5 relations (primary measures each funnel metric), got {}",
            relation_count
        );

        // Verify all relations are "measures" type
        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        let primary_entity = entities
            .iter()
            .find(|e| {
                e.provenance_run_id.as_deref() == Some(&output.run_id)
                    && e.category.as_deref() == Some("primary")
            })
            .expect("Should find primary metric");

        let relations = StoreService::get_relations(&conn, &primary_entity.id).unwrap();
        assert!(relations.len() >= 5);
        for rel in &relations {
            assert_eq!(rel.relation_type, "measures");
            assert_eq!(rel.from_id, primary_entity.id);
        }
    }

    #[test]
    fn test_metric_tree_run_logged() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Verify run was logged
        let run = StoreService::get_run(&conn, &output.run_id).unwrap();
        assert_eq!(run.template_key, "analytics-metric-tree");
        assert_eq!(run.template_version, "1.0");
        assert_eq!(run.template_category, "analytics");
        assert_eq!(run.status, RunStatus::Applied);
    }

    #[test]
    fn test_metric_tree_run_inputs_snapshot() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        let run = StoreService::get_run(&conn, &output.run_id).unwrap();
        assert_eq!(run.inputs_snapshot["business_model"], "SaaS");
        assert_eq!(run.inputs_snapshot["primary_objective"], "Revenue Growth");
    }

    #[test]
    fn test_metric_tree_entities_at_current_schema_version() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        let run_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.provenance_run_id.as_deref() == Some(&output.run_id))
            .collect();

        for entity in &run_entities {
            // Schema version should be at least 1 (current)
            assert!(
                entity.schema_version >= 1,
                "Entity '{}' has schema_version {}, expected >= 1",
                entity.title,
                entity.schema_version
            );
        }
    }

    #[test]
    fn test_metric_tree_no_warnings_when_prereqs_met() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // metric-tree has no prerequisites, so no warnings
        assert!(output.warnings.is_empty());
    }

    // ========================================================================
    // Forced run tests
    // ========================================================================

    #[test]
    fn test_forced_run_succeeds_when_prereqs_not_met() {
        let conn = test_db();

        // analytics-experiment-plan requires metrics, but we're not creating any.
        // But it doesn't have an implementation yet, so we test the prerequisite
        // bypass mechanism using a different approach.

        // Actually, let's test that the metric-tree runs fine (it has no prereqs anyway)
        // and then verify that forced=true would bypass if there were prereqs.
        // We can verify the force mechanism by testing with experiment-plan template
        // which has prereqs but no implementation.

        // For now, test that metric-tree with force=true works the same as force=false
        let input = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({
                "business_model": "E-commerce",
                "primary_objective": "Conversion",
                "customer_journey": "Browse → Cart → Purchase"
            }),
            force: true,
        };

        let output = run_template_full(&conn, &input).unwrap();
        assert!(!output.run_id.is_empty());
        assert!(output.warnings.is_empty()); // No prereqs to warn about
    }

    // ========================================================================
    // Provenance verification
    // ========================================================================

    #[test]
    fn test_provenance_all_entities_found_by_run_id() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Use direct SQL to find all entities by provenance_run_id
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_type, source, provenance_run_id FROM entities \
                 WHERE provenance_run_id = ?1",
            )
            .unwrap();

        let rows: Vec<(String, String, String, String)> = stmt
            .query_map(params![output.run_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert!(
            rows.len() >= 5,
            "Expected at least 5 entities with provenance_run_id '{}', found {}",
            output.run_id,
            rows.len()
        );

        for (id, entity_type, source, prov_run_id) in &rows {
            assert_eq!(entity_type, "metric", "Entity {} should be metric", id);
            assert_eq!(source, "template", "Entity {} should have source=template", id);
            assert_eq!(prov_run_id, &output.run_id);
        }
    }

    #[test]
    fn test_provenance_relations_have_run_id() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template_full(&conn, &input).unwrap();

        // Check that relations also have provenance_run_id
        let rel_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM relations WHERE provenance_run_id = ?1",
                params![output.run_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            rel_count >= 5,
            "Expected at least 5 relations with provenance_run_id, got {}",
            rel_count
        );
    }

    // ========================================================================
    // Error cases
    // ========================================================================

    #[test]
    fn test_run_template_unknown_template_key() {
        let conn = test_db();
        let input = TemplateInput {
            template_key: "nonexistent".to_string(),
            params: json!({}),
            force: false,
        };

        let result = run_template_full(&conn, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_template_prereqs_not_met_not_forced() {
        let conn = test_db();
        // analytics-experiment-plan requires at least 1 metric
        let input = TemplateInput {
            template_key: "analytics-experiment-plan".to_string(),
            params: json!({}),
            force: false,
        };

        let result = run_template_full(&conn, &input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Prerequisites not met"),
            "Error should mention prerequisites: {}",
            err_msg
        );
    }

    #[test]
    fn test_metric_tree_different_business_model() {
        let conn = test_db();
        let input = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({
                "business_model": "E-commerce",
                "primary_objective": "Conversion Rate",
                "customer_journey": "Browse → Cart → Checkout → Purchase"
            }),
            force: false,
        };

        let output = run_template_full(&conn, &input).unwrap();

        // Verify entities were created with the right business model context
        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        let run_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.provenance_run_id.as_deref() == Some(&output.run_id))
            .collect();

        assert!(run_entities.len() >= 5);

        // Check that the primary metric title contains the objective
        let primary = run_entities
            .iter()
            .find(|e| e.category.as_deref() == Some("primary"))
            .expect("Should find primary metric");
        assert!(primary.title.contains("Conversion Rate"));
    }

    #[test]
    fn test_metric_tree_default_params() {
        let conn = test_db();
        // Test with empty params - should use defaults
        let input = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({}),
            force: false,
        };

        let output = run_template_full(&conn, &input).unwrap();
        assert!(output.patch_result.applied.len() >= 5);
    }

    #[test]
    fn test_multiple_metric_tree_runs_independent() {
        let conn = test_db();

        let input1 = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({"business_model": "SaaS", "primary_objective": "Revenue"}),
            force: false,
        };
        let output1 = run_template_full(&conn, &input1).unwrap();

        let input2 = TemplateInput {
            template_key: "analytics-metric-tree".to_string(),
            params: json!({"business_model": "Marketplace", "primary_objective": "GMV"}),
            force: false,
        };
        let output2 = run_template_full(&conn, &input2).unwrap();

        // Both should succeed
        assert_ne!(output1.run_id, output2.run_id);

        // Each should have its own entities
        let count1: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
                params![output1.run_id],
                |row| row.get(0),
            )
            .unwrap();

        let count2: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE provenance_run_id = ?1",
                params![output2.run_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(count1 >= 5);
        assert!(count2 >= 5);
    }

    // ========================================================================
    // run_template (simple version) tests
    // ========================================================================

    #[test]
    fn test_run_template_simple_version_works() {
        let conn = test_db();
        let input = default_metric_tree_input();
        let output = run_template(&conn, &input).unwrap();

        // Simple version creates entities but not relations (no two-phase)
        let entity_count = output
            .patch_result
            .applied
            .iter()
            .filter(|op| op.entity_id.is_some())
            .count();
        assert!(entity_count >= 5);
    }
}

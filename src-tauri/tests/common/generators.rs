use proptest::prelude::*;

// =============================================================================
// All 22 entity types
// =============================================================================

/// The complete list of 22 entity types in the knowledge graph.
pub const ALL_ENTITY_TYPES: &[&str] = &[
    "metric", "experiment", "result", "task", "project", "decision",
    "person", "note", "session", "campaign", "audience", "competitor",
    "channel", "spec", "budget", "vendor", "playbook", "taxonomy",
    "backlog", "brief", "event", "policy",
];

/// Generate a random entity type from all 22 types.
pub fn gen_entity_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("metric".to_string()),
        Just("experiment".to_string()),
        Just("result".to_string()),
        Just("task".to_string()),
        Just("project".to_string()),
        Just("decision".to_string()),
        Just("person".to_string()),
        Just("note".to_string()),
        Just("session".to_string()),
        Just("campaign".to_string()),
        Just("audience".to_string()),
        Just("competitor".to_string()),
        Just("channel".to_string()),
        Just("spec".to_string()),
        Just("budget".to_string()),
        Just("vendor".to_string()),
        Just("playbook".to_string()),
        Just("taxonomy".to_string()),
        Just("backlog".to_string()),
        Just("brief".to_string()),
        Just("event".to_string()),
        Just("policy".to_string()),
    ]
}

/// Generate random canonical fields that may or may not be valid,
/// dispatching to per-type generators for all 22 entity types.
pub fn gen_canonical_fields(entity_type: &str) -> impl Strategy<Value = serde_json::Value> {
    match entity_type {
        "metric" => gen_metric_fields().boxed(),
        "experiment" => gen_experiment_fields().boxed(),
        "result" => gen_result_fields().boxed(),
        "task" => gen_task_fields().boxed(),
        "project" => gen_project_fields().boxed(),
        "decision" => gen_decision_fields().boxed(),
        "person" => gen_person_fields().boxed(),
        "note" => gen_note_fields().boxed(),
        "session" => gen_session_fields().boxed(),
        "campaign" => gen_campaign_fields().boxed(),
        "audience" => gen_audience_fields().boxed(),
        "competitor" => gen_competitor_fields().boxed(),
        "channel" => gen_channel_fields().boxed(),
        "spec" => gen_spec_fields().boxed(),
        "budget" => gen_budget_fields().boxed(),
        "vendor" => gen_vendor_fields().boxed(),
        "playbook" => gen_playbook_fields().boxed(),
        "taxonomy" => gen_taxonomy_fields().boxed(),
        "backlog" => gen_backlog_fields().boxed(),
        "brief" => gen_brief_fields().boxed(),
        "event" => gen_event_fields().boxed(),
        "policy" => gen_policy_fields().boxed(),
        _ => Just(serde_json::json!({})).boxed(),
    }
}

/// Generate VALID canonical fields for a given entity type.
/// These always produce schema-valid values (correct types, valid enums, required fields present).
pub fn gen_valid_canonical_fields(entity_type: &str) -> impl Strategy<Value = serde_json::Value> {
    match entity_type {
        "metric" => gen_valid_metric_fields().boxed(),
        "experiment" => gen_valid_experiment_fields().boxed(),
        "result" => gen_valid_result_fields().boxed(),
        "task" => gen_valid_task_fields().boxed(),
        "project" => gen_valid_project_fields().boxed(),
        "decision" => gen_valid_decision_fields().boxed(),
        "person" => gen_valid_person_fields().boxed(),
        "note" => gen_valid_note_fields().boxed(),
        "session" => gen_valid_session_fields().boxed(),
        "campaign" => gen_valid_campaign_fields().boxed(),
        "audience" => gen_valid_audience_fields().boxed(),
        "competitor" => gen_valid_competitor_fields().boxed(),
        "channel" => gen_valid_channel_fields().boxed(),
        "spec" => gen_valid_spec_fields().boxed(),
        "budget" => gen_valid_budget_fields().boxed(),
        "vendor" => gen_valid_vendor_fields().boxed(),
        "playbook" => gen_valid_playbook_fields().boxed(),
        "taxonomy" => gen_valid_taxonomy_fields().boxed(),
        "backlog" => gen_valid_backlog_fields().boxed(),
        "brief" => gen_valid_brief_fields().boxed(),
        "event" => gen_valid_event_fields().boxed(),
        "policy" => gen_valid_policy_fields().boxed(),
        _ => Just(serde_json::json!({})).boxed(),
    }
}

/// Generate a valid status for a given entity type (always valid for that type).
pub fn gen_valid_status_for_type(entity_type: &str) -> impl Strategy<Value = String> {
    let statuses: Vec<&str> = match entity_type {
        "metric" => vec!["active", "paused", "deprecated", "archived"],
        "experiment" => vec!["draft", "running", "concluded", "archived"],
        "result" => vec!["draft", "final", "archived"],
        "task" => vec!["backlog", "todo", "in_progress", "blocked", "done", "archived"],
        "project" => vec!["planning", "active", "paused", "completed", "archived"],
        "decision" => vec!["proposed", "accepted", "deprecated", "superseded"],
        "person" => vec!["active", "inactive", "archived"],
        "note" => vec!["draft", "final", "archived"],
        "session" => vec!["scheduled", "in_progress", "completed", "cancelled"],
        "campaign" => vec!["planning", "active", "paused", "completed", "archived"],
        "audience" => vec!["draft", "validated", "active", "archived"],
        "competitor" => vec!["tracking", "dormant", "archived"],
        "channel" => vec!["evaluating", "active", "scaling", "paused", "deprecated"],
        "spec" => vec!["draft", "review", "approved", "deprecated"],
        "budget" => vec!["draft", "approved", "active", "closed"],
        "vendor" => vec!["evaluating", "active", "on_hold", "terminated"],
        "playbook" => vec!["draft", "active", "deprecated", "archived"],
        "taxonomy" => vec!["draft", "active", "archived"],
        "backlog" => vec!["open", "triaged", "scheduled", "closed"],
        "brief" => vec!["draft", "review", "approved", "archived"],
        "event" => vec!["proposed", "confirmed", "in_progress", "completed", "cancelled"],
        "policy" => vec!["draft", "active", "under_review", "deprecated"],
        _ => vec!["draft"],
    };
    let owned: Vec<String> = statuses.into_iter().map(|s| s.to_string()).collect();
    proptest::sample::select(owned)
}

/// Returns the first (initial) valid status for a given entity type.
pub fn initial_status_for_type(entity_type: &str) -> &'static str {
    match entity_type {
        "metric" => "active",
        "experiment" => "draft",
        "result" => "draft",
        "task" => "backlog",
        "project" => "planning",
        "decision" => "proposed",
        "person" => "active",
        "note" => "draft",
        "session" => "scheduled",
        "campaign" => "planning",
        "audience" => "draft",
        "competitor" => "tracking",
        "channel" => "evaluating",
        "spec" => "draft",
        "budget" => "draft",
        "vendor" => "evaluating",
        "playbook" => "draft",
        "taxonomy" => "draft",
        "backlog" => "open",
        "brief" => "draft",
        "event" => "proposed",
        "policy" => "draft",
        _ => "draft",
    }
}

/// Returns the second valid status for a given entity type (for forward transition tests).
pub fn second_status_for_type(entity_type: &str) -> &'static str {
    match entity_type {
        "metric" => "paused",
        "experiment" => "running",
        "result" => "final",
        "task" => "todo",
        "project" => "active",
        "decision" => "accepted",
        "person" => "inactive",
        "note" => "final",
        "session" => "in_progress",
        "campaign" => "active",
        "audience" => "validated",
        "competitor" => "dormant",
        "channel" => "active",
        "spec" => "review",
        "budget" => "approved",
        "vendor" => "active",
        "playbook" => "active",
        "taxonomy" => "active",
        "backlog" => "triaged",
        "brief" => "review",
        "event" => "confirmed",
        "policy" => "active",
        _ => "active",
    }
}

// =============================================================================
// Original 3 types (metric, experiment, result) -- fuzz generators with invalid data
// =============================================================================

fn gen_metric_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            any::<f64>().prop_map(|v| Some(serde_json::Value::from(v))),
            Just(Some(serde_json::Value::from("not_a_number"))), // invalid type
        ],
        prop_oneof![
            Just(None),
            any::<f64>().prop_map(|v| Some(serde_json::Value::from(v))),
        ],
        prop_oneof![
            Just(None),
            Just(Some("up".to_string())),
            Just(Some("down".to_string())),
            Just(Some("flat".to_string())),
            Just(Some("sideways".to_string())), // invalid enum
            "\\PC{1,20}".prop_map(|s| Some(s)), // random string
        ],
        prop_oneof![
            Just(None),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
    ).prop_map(|(current_value, target_value, trend, data_source)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = current_value { map.insert("current_value".to_string(), v); }
        if let Some(v) = target_value { map.insert("target_value".to_string(), v); }
        if let Some(v) = trend { map.insert("trend".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = data_source { map.insert("data_source".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_experiment_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-id".to_string())),
            "[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}".prop_map(|s| Some(s)),
        ],
    ).prop_map(|(hypothesis, funnel_position, source_experiment_id)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = hypothesis { map.insert("hypothesis".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = funnel_position { map.insert("funnel_position".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = source_experiment_id { map.insert("source_experiment_id".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_result_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,100}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![
            Just(None),
            (0.0..1.0f64).prop_map(|v| Some(v)),
            any::<f64>().prop_map(|v| Some(v)), // may be out of range
        ],
    ).prop_map(|(findings, methodology, confidence_level)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = findings { map.insert("findings".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = methodology { map.insert("methodology".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = confidence_level { map.insert("confidence_level".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

// =============================================================================
// Wave 1C types: task, project, decision, person, note, session
// =============================================================================

fn gen_task_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), Just(Some("S".to_string())), Just(Some("M".to_string())), Just(Some("L".to_string()))],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-project-id".to_string())),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
    ).prop_map(|(assignee, effort_estimate, project_id, acceptance_criteria)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = assignee { map.insert("assignee".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = effort_estimate { map.insert("effort_estimate".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = project_id { map.insert("project_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = acceptance_criteria { map.insert("acceptance_criteria".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_project_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
    ).prop_map(|(owner_id, objective, success_criteria, timeline)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = owner_id { map.insert("owner_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = objective { map.insert("objective".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = success_criteria { map.insert("success_criteria".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = timeline { map.insert("timeline".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_decision_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        // owner_id (required) -- sometimes omit to test validation
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        // rationale (required) -- sometimes omit to test validation
        prop_oneof![Just(None), "\\PC{1,80}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,80}".prop_map(|s| Some(s))],
    ).prop_map(|(owner_id, decided_at, rationale, revisit_triggers, options_considered)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = owner_id { map.insert("owner_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = decided_at { map.insert("decided_at".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = rationale { map.insert("rationale".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = revisit_triggers { map.insert("revisit_triggers".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = options_considered { map.insert("options_considered".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_person_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,40}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![
            Just(None),
            Just(Some(true)),
            Just(Some(false)),
        ],
    ).prop_map(|(email, role, team, external)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = email { map.insert("email".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = role { map.insert("role".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = team { map.insert("team".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = external { map.insert("external".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_note_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,40}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-entity-id".to_string())),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
    ).prop_map(|(context, tags, linked_entity_id)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = context { map.insert("context".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = tags { map.insert("tags".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = linked_entity_id { map.insert("linked_entity_id".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_session_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("planning".to_string())),
            Just(Some("review".to_string())),
            Just(Some("standup".to_string())),
            Just(Some("workshop".to_string())),
            Just(Some("retro".to_string())),
            Just(Some("invalid_type".to_string())), // invalid enum
        ],
        prop_oneof![Just(None), "\\PC{1,40}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
    ).prop_map(|(session_type, participants, agenda, outcomes)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = session_type { map.insert("session_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = participants { map.insert("participants".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = agenda { map.insert("agenda".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = outcomes { map.insert("outcomes".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

// =============================================================================
// Wave 2C types: campaign, audience, competitor, channel, spec, budget,
//                vendor, playbook, taxonomy, backlog, brief, event, policy
// =============================================================================

fn gen_campaign_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
        prop_oneof![
            Just(None),
            Just(Some("email".to_string())),
            Just(Some("paid_social".to_string())),
            Just(Some("paid_search".to_string())),
            Just(Some("organic".to_string())),
            Just(Some("events".to_string())),
            Just(Some("partnerships".to_string())),
            Just(Some("invalid_channel".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-audience-id".to_string())),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
    ).prop_map(|(objective, budget, channel, start_date, end_date, target_audience_id)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = objective { map.insert("objective".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = budget { map.insert("budget".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = channel { map.insert("channel".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = start_date { map.insert("start_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = end_date { map.insert("end_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = target_audience_id { map.insert("target_audience_id".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_audience_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-icp-id".to_string())),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
        prop_oneof![Just(None), "\\PC{1,40}".prop_map(|s| Some(s))],
    ).prop_map(|(segment_criteria, estimated_size, icp_id, channels)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = segment_criteria { map.insert("segment_criteria".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = estimated_size { map.insert("estimated_size".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = icp_id { map.insert("icp_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = channels { map.insert("channels".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_competitor_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), "\\PC{1,40}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
    ).prop_map(|(website, positioning, strengths, weaknesses, market_share)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = website { map.insert("website".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = positioning { map.insert("positioning".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = strengths { map.insert("strengths".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = weaknesses { map.insert("weaknesses".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = market_share { map.insert("market_share".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_channel_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("email".to_string())),
            Just(Some("social".to_string())),
            Just(Some("search".to_string())),
            Just(Some("display".to_string())),
            Just(Some("events".to_string())),
            Just(Some("partnerships".to_string())),
            Just(Some("content".to_string())),
            Just(Some("referral".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-metric-id".to_string())),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
    ).prop_map(|(channel_type, cost_model, primary_metric_id, budget_allocation)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = channel_type { map.insert("channel_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = cost_model { map.insert("cost_model".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = primary_metric_id { map.insert("primary_metric_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = budget_allocation { map.insert("budget_allocation".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_spec_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("technical".to_string())),
            Just(Some("product".to_string())),
            Just(Some("design".to_string())),
            Just(Some("process".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,10}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
    ).prop_map(|(spec_type, version, approval_status, author)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = spec_type { map.insert("spec_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = version { map.insert("version".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = approval_status { map.insert("approval_status".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = author { map.insert("author".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_budget_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("USD".to_string())), Just(Some("EUR".to_string())), "\\PC{1,5}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
    ).prop_map(|(total_amount, currency, period, allocated, spent)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = total_amount { map.insert("total_amount".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = currency { map.insert("currency".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = period { map.insert("period".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = allocated { map.insert("allocated".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = spent { map.insert("spent".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_vendor_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("agency".to_string())),
            Just(Some("saas".to_string())),
            Just(Some("contractor".to_string())),
            Just(Some("platform".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
    ).prop_map(|(vendor_type, contract_value, contract_end, primary_contact)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = vendor_type { map.insert("vendor_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = contract_value { map.insert("contract_value".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = contract_end { map.insert("contract_end".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = primary_contact { map.insert("primary_contact".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_playbook_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("sales".to_string())),
            Just(Some("marketing".to_string())),
            Just(Some("ops".to_string())),
            Just(Some("cs".to_string())),
            Just(Some("dev".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
    ).prop_map(|(playbook_type, trigger_conditions, expected_outcome, owner)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = playbook_type { map.insert("playbook_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = trigger_conditions { map.insert("trigger_conditions".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = expected_outcome { map.insert("expected_outcome".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = owner { map.insert("owner".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_taxonomy_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("category".to_string())),
            Just(Some("tag".to_string())),
            Just(Some("hierarchy".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![
            Just(None),
            Just(Some("nonexistent-parent-id".to_string())),
            "\\PC{1,30}".prop_map(|s| Some(s)),
        ],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
    ).prop_map(|(taxonomy_type, parent_id, level)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = taxonomy_type { map.insert("taxonomy_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = parent_id { map.insert("parent_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = level { map.insert("level".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_backlog_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
    ).prop_map(|(priority_score, effort, requester, target_sprint)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = priority_score { map.insert("priority_score".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = effort { map.insert("effort".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = requester { map.insert("requester".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = target_sprint { map.insert("target_sprint".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_brief_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("creative".to_string())),
            Just(Some("campaign".to_string())),
            Just(Some("product".to_string())),
            Just(Some("event".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,50}".prop_map(|s| Some(s))],
    ).prop_map(|(brief_type, deadline, stakeholders, deliverables)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = brief_type { map.insert("brief_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = deadline { map.insert("deadline".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = stakeholders { map.insert("stakeholders".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = deliverables { map.insert("deliverables".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_event_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("conference".to_string())),
            Just(Some("webinar".to_string())),
            Just(Some("meetup".to_string())),
            Just(Some("workshop".to_string())),
            Just(Some("launch".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), any::<f64>().prop_map(|v| Some(v))],
    ).prop_map(|(event_type, venue, start_date, end_date, expected_attendees)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = event_type { map.insert("event_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = venue { map.insert("venue".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = start_date { map.insert("start_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = end_date { map.insert("end_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = expected_attendees { map.insert("expected_attendees".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_policy_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![
            Just(None),
            Just(Some("security".to_string())),
            Just(Some("hr".to_string())),
            Just(Some("compliance".to_string())),
            Just(Some("operational".to_string())),
            Just(Some("invalid_type".to_string())), // invalid
        ],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,20}".prop_map(|s| Some(s))],
        prop_oneof![Just(None), "\\PC{1,30}".prop_map(|s| Some(s))],
    ).prop_map(|(policy_type, effective_date, review_date, owner)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = policy_type { map.insert("policy_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = effective_date { map.insert("effective_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = review_date { map.insert("review_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = owner { map.insert("owner".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

// =============================================================================
// VALID field generators -- always produce schema-valid values
// =============================================================================

fn gen_valid_metric_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), (0.0..1000.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), (0.0..1000.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("up".to_string())), Just(Some("down".to_string())), Just(Some("flat".to_string()))],
        prop_oneof![Just(None), Just(Some("analytics".to_string())), Just(Some("manual".to_string()))],
    ).prop_map(|(current_value, target_value, trend, data_source)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = current_value { map.insert("current_value".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = target_value { map.insert("target_value".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = trend { map.insert("trend".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = data_source { map.insert("data_source".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_experiment_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("Test hypothesis".to_string()))],
        prop_oneof![Just(None), Just(Some("checkout".to_string()))],
    ).prop_map(|(hypothesis, funnel_position)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = hypothesis { map.insert("hypothesis".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = funnel_position { map.insert("funnel_position".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_result_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("Test findings".to_string()))],
        prop_oneof![Just(None), Just(Some("A/B test".to_string()))],
        prop_oneof![Just(None), (0.0..1.0f64).prop_map(|v| Some(v))],
    ).prop_map(|(findings, methodology, confidence_level)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = findings { map.insert("findings".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = methodology { map.insert("methodology".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = confidence_level { map.insert("confidence_level".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_task_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("Alice".to_string()))],
        prop_oneof![Just(None), Just(Some("M".to_string()))],
        prop_oneof![Just(None), Just(Some("Done when tests pass".to_string()))],
    ).prop_map(|(assignee, effort_estimate, acceptance_criteria)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = assignee { map.insert("assignee".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = effort_estimate { map.insert("effort_estimate".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = acceptance_criteria { map.insert("acceptance_criteria".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_project_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("team-lead".to_string()))],
        prop_oneof![Just(None), Just(Some("Launch v2".to_string()))],
        prop_oneof![Just(None), Just(Some("Revenue up 10%".to_string()))],
        prop_oneof![Just(None), Just(Some("Q1 2025".to_string()))],
    ).prop_map(|(owner_id, objective, success_criteria, timeline)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = owner_id { map.insert("owner_id".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = objective { map.insert("objective".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = success_criteria { map.insert("success_criteria".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = timeline { map.insert("timeline".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_decision_fields() -> impl Strategy<Value = serde_json::Value> {
    // decision requires owner_id and rationale
    (
        "\\PC{1,20}".prop_map(|s| s), // owner_id always present
        "\\PC{1,50}".prop_map(|s| s), // rationale always present
        prop_oneof![Just(None), Just(Some("2025-01-01".to_string()))],
    ).prop_map(|(owner_id, rationale, decided_at)| {
        let mut map = serde_json::Map::new();
        map.insert("owner_id".to_string(), serde_json::Value::from(owner_id));
        map.insert("rationale".to_string(), serde_json::Value::from(rationale));
        if let Some(v) = decided_at { map.insert("decided_at".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_person_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("alice@example.com".to_string()))],
        prop_oneof![Just(None), Just(Some("Engineer".to_string()))],
        prop_oneof![Just(None), Just(Some("Platform".to_string()))],
        prop_oneof![Just(None), Just(Some(false))],
    ).prop_map(|(email, role, team, external)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = email { map.insert("email".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = role { map.insert("role".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = team { map.insert("team".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = external { map.insert("external".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_note_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("meeting notes".to_string()))],
        prop_oneof![Just(None), Just(Some("planning,strategy".to_string()))],
    ).prop_map(|(context, tags)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = context { map.insert("context".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = tags { map.insert("tags".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_session_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("planning".to_string())), Just(Some("review".to_string())), Just(Some("standup".to_string()))],
        prop_oneof![Just(None), Just(Some("Alice, Bob".to_string()))],
        prop_oneof![Just(None), Just(Some("Sprint planning".to_string()))],
    ).prop_map(|(session_type, participants, agenda)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = session_type { map.insert("session_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = participants { map.insert("participants".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = agenda { map.insert("agenda".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_campaign_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("Increase signups".to_string()))],
        prop_oneof![Just(None), (100.0..100000.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("email".to_string())), Just(Some("paid_social".to_string()))],
    ).prop_map(|(objective, budget, channel)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = objective { map.insert("objective".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = budget { map.insert("budget".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = channel { map.insert("channel".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_audience_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("Enterprise SaaS buyers".to_string()))],
        prop_oneof![Just(None), (100.0..1000000.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("email,linkedin".to_string()))],
    ).prop_map(|(segment_criteria, estimated_size, channels)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = segment_criteria { map.insert("segment_criteria".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = estimated_size { map.insert("estimated_size".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = channels { map.insert("channels".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_competitor_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("https://example.com".to_string()))],
        prop_oneof![Just(None), Just(Some("Enterprise leader".to_string()))],
        prop_oneof![Just(None), Just(Some("Strong brand".to_string()))],
    ).prop_map(|(website, positioning, strengths)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = website { map.insert("website".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = positioning { map.insert("positioning".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = strengths { map.insert("strengths".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_channel_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("email".to_string())), Just(Some("social".to_string())), Just(Some("search".to_string()))],
        prop_oneof![Just(None), Just(Some("CPC".to_string()))],
        prop_oneof![Just(None), (100.0..50000.0f64).prop_map(|v| Some(v))],
    ).prop_map(|(channel_type, cost_model, budget_allocation)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = channel_type { map.insert("channel_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = cost_model { map.insert("cost_model".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = budget_allocation { map.insert("budget_allocation".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_spec_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("technical".to_string())), Just(Some("product".to_string()))],
        prop_oneof![Just(None), Just(Some("1.0".to_string()))],
        prop_oneof![Just(None), Just(Some("Alice".to_string()))],
    ).prop_map(|(spec_type, version, author)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = spec_type { map.insert("spec_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = version { map.insert("version".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = author { map.insert("author".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_budget_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), (1000.0..1000000.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("USD".to_string()))],
        prop_oneof![Just(None), Just(Some("Q1 2025".to_string()))],
    ).prop_map(|(total_amount, currency, period)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = total_amount { map.insert("total_amount".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = currency { map.insert("currency".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = period { map.insert("period".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_vendor_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("agency".to_string())), Just(Some("saas".to_string()))],
        prop_oneof![Just(None), (1000.0..500000.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("Jane Doe".to_string()))],
    ).prop_map(|(vendor_type, contract_value, primary_contact)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = vendor_type { map.insert("vendor_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = contract_value { map.insert("contract_value".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = primary_contact { map.insert("primary_contact".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_playbook_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("sales".to_string())), Just(Some("marketing".to_string()))],
        prop_oneof![Just(None), Just(Some("Lead qualifies".to_string()))],
        prop_oneof![Just(None), Just(Some("Close deal".to_string()))],
    ).prop_map(|(playbook_type, trigger_conditions, expected_outcome)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = playbook_type { map.insert("playbook_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = trigger_conditions { map.insert("trigger_conditions".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = expected_outcome { map.insert("expected_outcome".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_taxonomy_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("category".to_string())), Just(Some("tag".to_string()))],
        prop_oneof![Just(None), (0.0..10.0f64).prop_map(|v| Some(v))],
    ).prop_map(|(taxonomy_type, level)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = taxonomy_type { map.insert("taxonomy_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = level { map.insert("level".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_backlog_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), (1.0..100.0f64).prop_map(|v| Some(v))],
        prop_oneof![Just(None), Just(Some("S".to_string())), Just(Some("M".to_string()))],
        prop_oneof![Just(None), Just(Some("Product team".to_string()))],
    ).prop_map(|(priority_score, effort, requester)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = priority_score { map.insert("priority_score".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = effort { map.insert("effort".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = requester { map.insert("requester".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_brief_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("creative".to_string())), Just(Some("campaign".to_string()))],
        prop_oneof![Just(None), Just(Some("2025-03-01".to_string()))],
        prop_oneof![Just(None), Just(Some("Marketing, Design".to_string()))],
    ).prop_map(|(brief_type, deadline, stakeholders)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = brief_type { map.insert("brief_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = deadline { map.insert("deadline".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = stakeholders { map.insert("stakeholders".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_event_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("conference".to_string())), Just(Some("webinar".to_string()))],
        prop_oneof![Just(None), Just(Some("Virtual".to_string()))],
        prop_oneof![Just(None), (10.0..10000.0f64).prop_map(|v| Some(v))],
    ).prop_map(|(event_type, venue, expected_attendees)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = event_type { map.insert("event_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = venue { map.insert("venue".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = expected_attendees { map.insert("expected_attendees".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

fn gen_valid_policy_fields() -> impl Strategy<Value = serde_json::Value> {
    (
        prop_oneof![Just(None), Just(Some("security".to_string())), Just(Some("compliance".to_string()))],
        prop_oneof![Just(None), Just(Some("2025-01-01".to_string()))],
        prop_oneof![Just(None), Just(Some("Legal team".to_string()))],
    ).prop_map(|(policy_type, effective_date, owner)| {
        let mut map = serde_json::Map::new();
        if let Some(v) = policy_type { map.insert("policy_type".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = effective_date { map.insert("effective_date".to_string(), serde_json::Value::from(v)); }
        if let Some(v) = owner { map.insert("owner".to_string(), serde_json::Value::from(v)); }
        serde_json::Value::Object(map)
    })
}

// =============================================================================
// Shared generators
// =============================================================================

/// Generate a valid or invalid source
pub fn gen_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("manual".to_string()),
        Just("clipboard".to_string()),
        Just("web".to_string()),
        Just("import".to_string()),
        Just("agent".to_string()),
        Just("template".to_string()),
        Just("bootstrap".to_string()),
        "\\PC{3,10}".prop_map(|s| s), // random invalid source
    ]
}

/// Generate a valid source (only values that pass the CHECK constraint)
pub fn gen_valid_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("manual".to_string()),
        Just("clipboard".to_string()),
        Just("web".to_string()),
        Just("import".to_string()),
        Just("agent".to_string()),
        Just("template".to_string()),
        Just("bootstrap".to_string()),
    ]
}

/// Generate a valid or invalid status for an entity type
pub fn gen_status() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some("draft".to_string())),
        Just(Some("running".to_string())),
        Just(Some("concluded".to_string())),
        Just(Some("archived".to_string())),
        Just(Some("active".to_string())),
        Just(Some("paused".to_string())),
        Just(Some("deprecated".to_string())),
        Just(Some("final".to_string())),
        Just(Some("completed".to_string())),
        Just(Some("backlog".to_string())),
        Just(Some("todo".to_string())),
        Just(Some("in_progress".to_string())),
        Just(Some("blocked".to_string())),
        Just(Some("done".to_string())),
        Just(Some("planning".to_string())),
        Just(Some("proposed".to_string())),
        Just(Some("accepted".to_string())),
        Just(Some("superseded".to_string())),
        Just(Some("inactive".to_string())),
        Just(Some("scheduled".to_string())),
        Just(Some("cancelled".to_string())),
        Just(Some("validated".to_string())),
        Just(Some("tracking".to_string())),
        Just(Some("dormant".to_string())),
        Just(Some("evaluating".to_string())),
        Just(Some("scaling".to_string())),
        Just(Some("review".to_string())),
        Just(Some("approved".to_string())),
        Just(Some("closed".to_string())),
        Just(Some("on_hold".to_string())),
        Just(Some("terminated".to_string())),
        Just(Some("open".to_string())),
        Just(Some("triaged".to_string())),
        Just(Some("confirmed".to_string())),
        Just(Some("under_review".to_string())),
        Just(Some("totally_bogus".to_string())),    // invalid for all types
        "\\PC{3,15}".prop_map(|s| Some(s)),          // random
    ]
}

/// Generate a relation type (valid or invalid)
pub fn gen_relation_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("measures".to_string()),
        Just("tests".to_string()),
        Just("evidence_for".to_string()),
        Just("supports".to_string()),
        Just("related_to".to_string()),
        Just("custom:correlates_with".to_string()), // unapproved custom
        "\\PC{3,20}".prop_map(|s| s), // random
    ]
}

/// Generate a title
pub fn gen_title() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("MRR".to_string()),
        Just("Monthly Recurring Revenue".to_string()),
        Just("Pricing Test".to_string()),
        Just("Q1 Results".to_string()),
        Just("Sprint Planning".to_string()),
        Just("ICP Definition".to_string()),
        Just("Content Strategy".to_string()),
        "\\PC{1,50}".prop_map(|s| s),
    ]
}

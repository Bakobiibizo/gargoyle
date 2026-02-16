use proptest::prelude::*;

/// Generate a random entity type from the 3 stress test types
pub fn gen_entity_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("metric".to_string()),
        Just("experiment".to_string()),
        Just("result".to_string()),
    ]
}

/// Generate random canonical fields that may or may not be valid
pub fn gen_canonical_fields(entity_type: &str) -> impl Strategy<Value = serde_json::Value> {
    match entity_type {
        "metric" => gen_metric_fields().boxed(),
        "experiment" => gen_experiment_fields().boxed(),
        "result" => gen_result_fields().boxed(),
        _ => Just(serde_json::json!({})).boxed(),
    }
}

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
        Just(Some("completed".to_string())), // invalid for all 3 types
        "\\PC{3,15}".prop_map(|s| Some(s)), // random
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
        "\\PC{1,50}".prop_map(|s| s),
    ]
}

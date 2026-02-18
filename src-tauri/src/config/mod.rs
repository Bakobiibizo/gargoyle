pub mod entity_type_config;
pub mod template_loader;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::config::entity_type_config::{EntityTypeDef, load_entity_types};

pub use entity_type_config::FieldDefConfig;
pub use template_loader::{GenericConfig, LoadedTemplate};

/// Global singleton config, initialized on first access.
static CONFIG: OnceLock<GargoyleConfig> = OnceLock::new();

/// Top-level gargoyle.toml deserialization structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GargoyleToml {
    #[serde(default)]
    pub relation_types: RelationTypesConfig,
    #[serde(default)]
    pub dedup: DedupConfig,
    #[serde(default)]
    pub indexer: IndexerConfig,
    #[serde(default)]
    pub graph: GraphConfig,
    #[serde(default)]
    pub llm: LlmTuningConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelationTypesConfig {
    #[serde(default = "default_canonical_relation_types")]
    pub canonical: Vec<String>,
}

impl Default for RelationTypesConfig {
    fn default() -> Self {
        Self {
            canonical: default_canonical_relation_types(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DedupConfig {
    #[serde(default = "default_exact_match_confidence")]
    pub exact_match_confidence: f64,
    #[serde(default = "default_fuzzy_match_confidence")]
    pub fuzzy_match_confidence: f64,
    #[serde(default = "default_embedding_match_confidence")]
    pub embedding_match_confidence: f64,
    #[serde(default = "default_levenshtein_max_distance")]
    pub levenshtein_max_distance: usize,
    #[serde(default = "default_trigram_similarity_threshold")]
    pub trigram_similarity_threshold: f64,
    #[serde(default = "default_min_title_length_for_embedding")]
    pub min_title_length_for_embedding: usize,
    #[serde(default = "default_embedding_proximity_threshold")]
    pub embedding_proximity_threshold: f64,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            exact_match_confidence: default_exact_match_confidence(),
            fuzzy_match_confidence: default_fuzzy_match_confidence(),
            embedding_match_confidence: default_embedding_match_confidence(),
            levenshtein_max_distance: default_levenshtein_max_distance(),
            trigram_similarity_threshold: default_trigram_similarity_threshold(),
            min_title_length_for_embedding: default_min_title_length_for_embedding(),
            embedding_proximity_threshold: default_embedding_proximity_threshold(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexerConfig {
    #[serde(default = "default_embedding_dimensions")]
    pub embedding_dimensions: usize,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            embedding_dimensions: default_embedding_dimensions(),
            embedding_model: default_embedding_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphConfig {
    #[serde(default = "default_related_to_threshold")]
    pub related_to_threshold: f64,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            related_to_threshold: default_related_to_threshold(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmTuningConfig {
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: usize,
}

impl Default for LlmTuningConfig {
    fn default() -> Self {
        Self {
            max_tool_iterations: default_max_tool_iterations(),
        }
    }
}

// Default value functions
fn default_canonical_relation_types() -> Vec<String> {
    vec![
        "part_of", "derived_from", "depends_on", "blocks", "duplicate_of",
        "implements", "mentions", "supports", "contradicts", "tests",
        "decides", "evidence_for", "assigned_to", "created_in", "related_to",
        "targets", "competes_with", "measures", "funds", "enables",
        "mitigates", "promotes",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_exact_match_confidence() -> f64 { 0.95 }
fn default_fuzzy_match_confidence() -> f64 { 0.70 }
fn default_embedding_match_confidence() -> f64 { 0.80 }
fn default_levenshtein_max_distance() -> usize { 3 }
fn default_trigram_similarity_threshold() -> f64 { 0.7 }
fn default_min_title_length_for_embedding() -> usize { 4 }
fn default_embedding_proximity_threshold() -> f64 { 0.92 }
fn default_embedding_dimensions() -> usize { 128 }
fn default_embedding_model() -> String { "mock-hash-v1".to_string() }
fn default_related_to_threshold() -> f64 { 0.20 }
fn default_max_tool_iterations() -> usize { 10 }

/// The main configuration struct for Gargoyle.
///
/// Combines entity type definitions (from TOML files) with tuning parameters
/// (from gargoyle.toml) into a single runtime configuration.
pub struct GargoyleConfig {
    pub entity_types: HashMap<String, EntityTypeDef>,
    pub canonical_relation_types: Vec<String>,
    pub dedup: DedupConfig,
    pub indexer: IndexerConfig,
    pub graph: GraphConfig,
    pub llm_tuning: LlmTuningConfig,
}

impl GargoyleConfig {
    /// Load configuration from the given directory.
    ///
    /// Looks for:
    /// - `{dir}/gargoyle.toml` — main config
    /// - `{dir}/entity_types/*.toml` — entity type definitions
    pub fn load(dir: &Path) -> Result<Self, String> {
        // Load gargoyle.toml
        let toml_path = dir.join("gargoyle.toml");
        let gargoyle_toml: GargoyleToml = if toml_path.exists() {
            let content = std::fs::read_to_string(&toml_path)
                .map_err(|e| format!("Failed to read gargoyle.toml: {}", e))?;
            toml::from_str(&content)
                .map_err(|e| format!("Failed to parse gargoyle.toml: {}", e))?
        } else {
            GargoyleToml::default()
        };

        // Load entity types
        let entity_types_dir = dir.join("entity_types");
        let entity_types = if entity_types_dir.exists() {
            load_entity_types(&entity_types_dir)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            entity_types,
            canonical_relation_types: gargoyle_toml.relation_types.canonical,
            dedup: gargoyle_toml.dedup,
            indexer: gargoyle_toml.indexer,
            graph: gargoyle_toml.graph,
            llm_tuning: gargoyle_toml.llm,
        })
    }

    /// Returns hardcoded defaults — safety net for tests with no config dir.
    pub fn defaults() -> Self {
        Self {
            entity_types: default_entity_types(),
            canonical_relation_types: default_canonical_relation_types(),
            dedup: DedupConfig::default(),
            indexer: IndexerConfig::default(),
            graph: GraphConfig::default(),
            llm_tuning: LlmTuningConfig::default(),
        }
    }

    /// Returns a reference to the global singleton config.
    ///
    /// Tries to load from `./config/` first, falls back to defaults.
    pub fn global() -> &'static GargoyleConfig {
        CONFIG.get_or_init(|| {
            let config_dir = PathBuf::from("./config");
            match Self::load(&config_dir) {
                Ok(config) if !config.entity_types.is_empty() => config,
                _ => Self::defaults(),
            }
        })
    }
}

/// Build the hardcoded default entity type definitions.
/// This is the safety net that preserves all 27 current entity types
/// when no config directory is present (e.g., in tests).
fn default_entity_types() -> HashMap<String, EntityTypeDef> {
    use crate::config::entity_type_config::{FieldDefConfig, FieldTypeConfig, TaggedFieldType};

    let mut types = HashMap::new();

    // Helper macros to reduce boilerplate
    macro_rules! field {
        ($name:expr, $ft:expr) => {
            FieldDefConfig {
                name: $name.to_string(),
                field_type: $ft,
                required: false,
                description: None,
            }
        };
        ($name:expr, $ft:expr, required) => {
            FieldDefConfig {
                name: $name.to_string(),
                field_type: $ft,
                required: true,
                description: None,
            }
        };
    }

    macro_rules! s { () => { FieldTypeConfig::Simple("String".to_string()) }; }
    macro_rules! n { () => { FieldTypeConfig::Simple("Number".to_string()) }; }
    macro_rules! b { () => { FieldTypeConfig::Simple("Boolean".to_string()) }; }
    macro_rules! eref { ($t:expr) => { FieldTypeConfig::Tagged(TaggedFieldType::EntityRef($t.to_string())) }; }
    macro_rules! en {
        ($($v:expr),+) => {
            FieldTypeConfig::Tagged(TaggedFieldType::Enum(vec![$($v.to_string()),+]))
        };
    }

    // metric
    types.insert("metric".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["active".into(), "paused".into(), "deprecated".into(), "archived".into()],
        fields: vec![
            field!("current_value", n!()), field!("target_value", n!()),
            field!("trend", en!("up", "down", "flat")), field!("data_source", s!()),
        ],
    });

    // experiment
    types.insert("experiment".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "running".into(), "concluded".into(), "archived".into()],
        fields: vec![
            field!("hypothesis", s!()), field!("funnel_position", s!()),
            field!("primary_metric", s!()),
        ],
    });

    // result
    types.insert("result".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["preliminary".into(), "final".into(), "invalidated".into()],
        fields: vec![
            field!("outcome", s!()), field!("confidence_level", s!()),
            field!("data_summary", s!()),
        ],
    });

    // task
    types.insert("task".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["open".into(), "in_progress".into(), "blocked".into(), "done".into(), "cancelled".into()],
        fields: vec![
            field!("assignee_id", s!()), field!("effort_estimate", s!()),
            field!("project_id", eref!("project")), field!("acceptance_criteria", s!()),
        ],
    });

    // project
    types.insert("project".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["planning".into(), "active".into(), "paused".into(), "completed".into(), "archived".into()],
        fields: vec![
            field!("owner_id", s!()), field!("objective", s!()),
            field!("success_criteria", s!()), field!("timeline", s!()),
        ],
    });

    // decision
    types.insert("decision".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["pending".into(), "decided".into(), "revisited".into(), "superseded".into()],
        fields: vec![
            field!("owner_id", s!(), required), field!("decided_at", s!()),
            field!("rationale", s!(), required), field!("revisit_triggers", s!()),
            field!("options_considered", s!()),
        ],
    });

    // person
    types.insert("person".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["active".into(), "departed".into()],
        fields: vec![
            field!("email", s!()), field!("role", s!()),
            field!("department", s!()), field!("is_external", b!()),
        ],
    });

    // note
    types.insert("note".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec![],
        fields: vec![
            field!("context", s!()), field!("tags", s!()),
            field!("linked_entity_id", eref!("*")),
        ],
    });

    // session
    types.insert("session".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["active".into(), "ended".into()],
        fields: vec![
            field!("session_type", en!("planning", "review", "standup", "workshop", "retro")),
            field!("participants", s!()), field!("agenda", s!()), field!("outcomes", s!()),
        ],
    });

    // campaign
    types.insert("campaign".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["planning".into(), "live".into(), "paused".into(), "completed".into(), "cancelled".into()],
        fields: vec![
            field!("objective", s!()), field!("budget", n!()),
            field!("channel", en!("email", "paid_social", "paid_search", "organic", "events", "partnerships")),
            field!("start_date", s!()), field!("end_date", s!()),
            field!("target_audience_id", eref!("audience")),
        ],
    });

    // audience
    types.insert("audience".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "validated".into(), "deprecated".into()],
        fields: vec![
            field!("segment_criteria", s!()), field!("estimated_size", n!()),
            field!("icp_id", eref!("person")), field!("channels", s!()),
        ],
    });

    // competitor
    types.insert("competitor".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["active".into(), "acquired".into(), "defunct".into(), "irrelevant".into()],
        fields: vec![
            field!("website", s!()), field!("positioning", s!()),
            field!("strengths", s!()), field!("weaknesses", s!()),
            field!("market_share", s!()),
        ],
    });

    // channel
    types.insert("channel".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["active".into(), "paused".into(), "retired".into()],
        fields: vec![
            field!("channel_type", en!("email", "social", "search", "display", "events", "partnerships", "content", "referral")),
            field!("cost_model", s!()), field!("primary_metric_id", eref!("metric")),
            field!("budget_allocation", n!()),
        ],
    });

    // spec
    types.insert("spec".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "review".into(), "approved".into(), "superseded".into()],
        fields: vec![
            field!("spec_type", en!("technical", "product", "design", "process")),
            field!("version", s!()), field!("approval_status", s!()),
            field!("author", s!()),
        ],
    });

    // budget
    types.insert("budget".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "approved".into(), "active".into(), "exhausted".into(), "closed".into()],
        fields: vec![
            field!("total_amount", n!()), field!("currency", s!()),
            field!("period", s!()), field!("allocated", n!()), field!("spent", n!()),
        ],
    });

    // vendor
    types.insert("vendor".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["evaluating".into(), "active".into(), "paused".into(), "terminated".into()],
        fields: vec![
            field!("vendor_type", en!("agency", "saas", "contractor", "platform")),
            field!("contract_value", n!()), field!("contract_end", s!()),
            field!("primary_contact", s!()),
        ],
    });

    // playbook
    types.insert("playbook".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "active".into(), "deprecated".into()],
        fields: vec![
            field!("playbook_type", en!("sales", "marketing", "ops", "cs", "dev")),
            field!("trigger_conditions", s!()), field!("expected_outcome", s!()),
            field!("owner", s!()),
        ],
    });

    // taxonomy
    types.insert("taxonomy".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "active".into(), "superseded".into()],
        fields: vec![
            field!("taxonomy_type", en!("category", "tag", "hierarchy")),
            field!("parent_id", eref!("taxonomy")), field!("level", n!()),
        ],
    });

    // backlog
    types.insert("backlog".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["needs_triage".into(), "triaged".into(), "stale".into()],
        fields: vec![
            field!("priority_score", n!()), field!("effort", s!()),
            field!("requester", s!()), field!("target_sprint", s!()),
        ],
    });

    // brief
    types.insert("brief".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "approved".into(), "in_progress".into(), "completed".into()],
        fields: vec![
            field!("brief_type", en!("creative", "campaign", "product", "event")),
            field!("deadline", s!()), field!("stakeholders", s!()),
            field!("deliverables", s!()),
        ],
    });

    // event
    types.insert("event".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["concept".into(), "planning".into(), "confirmed".into(), "completed".into(), "cancelled".into()],
        fields: vec![
            field!("event_type", en!("conference", "webinar", "meetup", "workshop", "launch")),
            field!("venue", s!()), field!("start_date", s!()), field!("end_date", s!()),
            field!("expected_attendees", n!()),
        ],
    });

    // policy
    types.insert("policy".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["draft".into(), "review".into(), "active".into(), "superseded".into()],
        fields: vec![
            field!("policy_type", en!("security", "hr", "compliance", "operational")),
            field!("effective_date", s!()), field!("review_date", s!()),
            field!("owner", s!()),
        ],
    });

    // inbox_item
    types.insert("inbox_item".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["unprocessed".into(), "triaged".into(), "archived".into()],
        fields: vec![
            field!("source_text", s!(), required), field!("source_url", s!()),
            field!("suggested_type", s!()), field!("suggested_title", s!()),
        ],
    });

    // artifact_type
    types.insert("artifact_type".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec![],
        fields: vec![
            field!("artifact_kind", en!("attachment", "link", "export", "rendered_doc")),
            field!("uri_or_path", s!(), required), field!("hash", s!()),
            field!("mime", s!()), field!("parent_entity_id", eref!("*")),
        ],
    });

    // concept
    types.insert("concept".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec![],
        fields: vec![
            field!("definition", s!()), field!("domain", s!()),
        ],
    });

    // commitment
    types.insert("commitment".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["on_track".into(), "at_risk".into(), "blocked".into(), "fulfilled".into(), "broken".into()],
        fields: vec![
            field!("owner_id", s!(), required), field!("deadline", s!()),
            field!("source_context", s!()), field!("tracking_tool", s!()),
        ],
    });

    // issue
    types.insert("issue".to_string(), EntityTypeDef {
        version: 1,
        statuses: vec!["open".into(), "investigating".into(), "mitigated".into(), "resolved".into(), "wont_fix".into()],
        fields: vec![
            field!("severity", en!("critical", "high", "medium", "low")),
            field!("first_observed", s!()), field!("affected_area", s!()),
            field!("owner_id", s!()), field!("resolution_notes", s!()),
        ],
    });

    types
}

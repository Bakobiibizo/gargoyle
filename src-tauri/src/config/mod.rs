pub mod entity_type_config;
pub mod template_loader;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::config::entity_type_config::{load_entity_types, EntityTypeDef};

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
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_enabled")]
    pub enabled: bool,
    #[serde(default = "default_embedder_url")]
    pub embedder_url: String,
    #[serde(default = "default_embed_model")]
    pub embed_model: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: default_memory_enabled(),
            embedder_url: default_embedder_url(),
            embed_model: default_embed_model(),
        }
    }
}

fn default_memory_enabled() -> bool {
    true
}

fn default_embedder_url() -> String {
    "https://erasmus.ngrok.dev".to_string()
}

fn default_embed_model() -> String {
    "BAAI/bge-small-en-v1.5".to_string()
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
    #[serde(default = "default_use_real_embeddings")]
    pub use_real_embeddings: bool,
    #[serde(default = "default_embedder_url")]
    pub embedder_url: String,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            embedding_dimensions: default_embedding_dimensions(),
            embedding_model: default_embedding_model(),
            use_real_embeddings: default_use_real_embeddings(),
            embedder_url: default_embedder_url(),
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

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file_enabled")]
    pub file_enabled: bool,
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
    #[serde(default = "default_log_file_prefix")]
    pub file_prefix: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_enabled: default_log_file_enabled(),
            log_dir: default_log_dir(),
            file_prefix: default_log_file_prefix(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file_enabled() -> bool {
    true
}

fn default_log_dir() -> String {
    "./logs".to_string()
}

fn default_log_file_prefix() -> String {
    "gargoyle".to_string()
}

// Default value functions
fn default_canonical_relation_types() -> Vec<String> {
    vec![
        "part_of",
        "derived_from",
        "depends_on",
        "blocks",
        "duplicate_of",
        "implements",
        "mentions",
        "supports",
        "contradicts",
        "tests",
        "decides",
        "evidence_for",
        "assigned_to",
        "created_in",
        "related_to",
        "targets",
        "competes_with",
        "measures",
        "funds",
        "enables",
        "mitigates",
        "promotes",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_exact_match_confidence() -> f64 {
    0.95
}
fn default_fuzzy_match_confidence() -> f64 {
    0.70
}
fn default_embedding_match_confidence() -> f64 {
    0.80
}
fn default_levenshtein_max_distance() -> usize {
    3
}
fn default_trigram_similarity_threshold() -> f64 {
    0.7
}
fn default_min_title_length_for_embedding() -> usize {
    4
}
fn default_embedding_proximity_threshold() -> f64 {
    0.92
}
fn default_embedding_dimensions() -> usize {
    128
}
fn default_embedding_model() -> String {
    default_embed_model()
}
fn default_use_real_embeddings() -> bool {
    true
}
fn default_related_to_threshold() -> f64 {
    0.20
}
fn default_max_tool_iterations() -> usize {
    10
}

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
    pub logging: LoggingConfig,
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
            toml::from_str(&content).map_err(|e| format!("Failed to parse gargoyle.toml: {}", e))?
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
            logging: gargoyle_toml.logging,
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
            logging: LoggingConfig::default(),
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

    /// Initialize the global config from a specific directory.
    /// Must be called before any call to `global()`. If `global()` was
    /// already called, this is a no-op (OnceLock is already set).
    pub fn init_from_dir(dir: &Path) {
        CONFIG.get_or_init(|| {
            match Self::load(dir) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Warning: failed to load config from {}: {}", dir.display(), e);
                    Self::defaults()
                }
            }
        });
    }
}

/// Build the core entity type definitions.
///
/// **Rigid tier** (strict schema, factual): user_profile, company, person, commitment, metric
/// **Flexible tier** (loose schema, conceptual): doc, note, idea, task
fn default_entity_types() -> HashMap<String, EntityTypeDef> {
    use crate::config::entity_type_config::{FieldDefConfig, FieldTypeConfig, TaggedFieldType};

    let mut types = HashMap::new();

    macro_rules! field {
        ($name:expr, $ft:expr) => {
            FieldDefConfig {
                name: $name.to_string(),
                field_type: $ft,
                required: false,
                description: None,
            }
        };
    }

    macro_rules! s {
        () => {
            FieldTypeConfig::Simple("String".to_string())
        };
    }
    macro_rules! d {
        () => {
            FieldTypeConfig::Simple("Date".to_string())
        };
    }
    macro_rules! en {
        ($($v:expr),+) => {
            FieldTypeConfig::Tagged(TaggedFieldType::Enum(vec![$($v.to_string()),+]))
        };
    }
    macro_rules! arr {
        ($t:expr) => {
            FieldTypeConfig::Tagged(TaggedFieldType::Array($t.to_string()))
        };
    }

    // doc - Long-form content, documents
    types.insert(
        "doc".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec!["draft".into(), "published".into(), "archived".into()],
            fields: vec![
                field!("doc_type", s!()),
                field!("tags", arr!("string")),
                field!("source_url", s!()),
            ],
        },
    );

    // note - Quick captures, observations
    types.insert(
        "note".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec![],
            fields: vec![field!("context", s!()), field!("tags", arr!("string"))],
        },
    );

    // idea - Concepts to explore
    types.insert(
        "idea".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec![
                "seed".into(),
                "exploring".into(),
                "validated".into(),
                "parked".into(),
            ],
            fields: vec![
                field!("stage", en!("seed", "exploring", "validated", "parked")),
                field!("tags", arr!("string")),
                field!("potential_value", s!()),
            ],
        },
    );

    // task - Actionable items
    types.insert(
        "task".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec![
                "open".into(),
                "in_progress".into(),
                "blocked".into(),
                "done".into(),
                "cancelled".into(),
            ],
            fields: vec![
                field!("assignee", s!()),
                field!("due_date", d!()),
                field!("effort", en!("XS", "S", "M", "L", "XL")),
                field!("tags", arr!("string")),
            ],
        },
    );

    // =========================================================================
    // RIGID TIER - Factual entities with strict schemas
    // =========================================================================

    // user_profile - The user's identity (singleton, one per system)
    types.insert(
        "user_profile".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec!["active".into()],
            fields: vec![
                field!("full_name", s!()),
                field!("email", s!()),
                field!("timezone", s!()),
                field!("role", s!()),
                field!("department", s!()),
                field!("company_id", s!()),
                field!("communication_style", en!("formal", "casual", "technical")),
                field!("working_hours_start", s!()),
                field!("working_hours_end", s!()),
            ],
        },
    );

    // company - Organization context
    types.insert(
        "company".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec!["active".into(), "inactive".into()],
            fields: vec![
                field!("legal_name", s!()),
                field!("industry", s!()),
                field!(
                    "size",
                    en!("solo", "small", "medium", "large", "enterprise")
                ),
                field!("website", s!()),
                field!("mission", s!()),
                field!("values", arr!("string")),
            ],
        },
    );

    // person - People the user interacts with
    types.insert(
        "person".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec!["active".into(), "inactive".into()],
            fields: vec![
                field!("full_name", s!()),
                field!("email", s!()),
                field!("role", s!()),
                field!("company", s!()),
                field!(
                    "relationship",
                    en!(
                        "colleague",
                        "manager",
                        "report",
                        "client",
                        "vendor",
                        "partner",
                        "other"
                    )
                ),
                field!("notes", s!()),
            ],
        },
    );

    // commitment - Deadlines, promises, scheduled events
    types.insert(
        "commitment".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec![
                "scheduled".into(),
                "in_progress".into(),
                "completed".into(),
                "missed".into(),
                "cancelled".into(),
            ],
            fields: vec![
                field!(
                    "commitment_type",
                    en!("deadline", "meeting", "delivery", "review", "milestone")
                ),
                field!("due_date", d!()),
                field!("due_time", s!()),
                field!("stakeholders", arr!("string")),
                field!("deliverable", s!()),
                field!(
                    "recurrence",
                    en!("once", "daily", "weekly", "monthly", "quarterly")
                ),
            ],
        },
    );

    // metric - Tracked numbers and KPIs
    types.insert(
        "metric".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec!["active".into(), "paused".into(), "archived".into()],
            fields: vec![
                field!(
                    "metric_type",
                    en!("kpi", "okr", "target", "budget", "quota")
                ),
                field!(
                    "current_value",
                    FieldTypeConfig::Simple("Number".to_string())
                ),
                field!(
                    "target_value",
                    FieldTypeConfig::Simple("Number".to_string())
                ),
                field!("unit", s!()),
                field!(
                    "period",
                    en!("daily", "weekly", "monthly", "quarterly", "yearly")
                ),
                field!("trend", en!("up", "down", "flat", "unknown")),
            ],
        },
    );

    // project - Container for tasks and work streams
    types.insert(
        "project".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec![
                "planning".into(),
                "active".into(),
                "on_hold".into(),
                "completed".into(),
                "cancelled".into(),
            ],
            fields: vec![
                field!("owner", s!()),
                field!("start_date", d!()),
                field!("target_end_date", d!()),
                field!("actual_end_date", d!()),
                field!("priority", en!("low", "medium", "high", "critical")),
                field!("tags", arr!("string")),
            ],
        },
    );

    // =========================================================================
    // LEGACY TYPES - Required for existing tests
    // =========================================================================

    // experiment - For A/B tests and trials (legacy)
    types.insert(
        "experiment".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec![
                "draft".into(),
                "running".into(),
                "completed".into(),
                "archived".into(),
            ],
            fields: vec![
                field!("hypothesis", s!()),
                field!("start_date", d!()),
                field!("end_date", d!()),
                field!("tags", arr!("string")),
            ],
        },
    );

    // result - Outcomes and findings (legacy)
    types.insert(
        "result".to_string(),
        EntityTypeDef {
            version: 1,
            statuses: vec!["preliminary".into(), "final".into(), "archived".into()],
            fields: vec![
                field!("outcome", s!()),
                field!("confidence", FieldTypeConfig::Simple("Number".to_string())),
                field!("tags", arr!("string")),
            ],
        },
    );

    types
}

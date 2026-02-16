use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub category: Option<String>,
    pub title: String,
    pub body_md: String,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub due_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub source: Source,
    pub canonical_fields: serde_json::Value,
    pub schema_version: i32,
    pub deleted_at: Option<String>,
    pub provenance_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Manual,
    Clipboard,
    Web,
    Import,
    Agent,
    Template,
    Bootstrap,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Manual => write!(f, "manual"),
            Source::Clipboard => write!(f, "clipboard"),
            Source::Web => write!(f, "web"),
            Source::Import => write!(f, "import"),
            Source::Agent => write!(f, "agent"),
            Source::Template => write!(f, "template"),
            Source::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

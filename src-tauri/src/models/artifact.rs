use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    pub entity_id: String,
    pub kind: ArtifactKind,
    pub uri_or_path: String,
    pub hash: Option<String>,
    pub mime: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Attachment,
    Link,
    Export,
    RenderedDoc,
}

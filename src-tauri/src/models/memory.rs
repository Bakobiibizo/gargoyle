use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub message_count: i32,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            ended_at: None,
            title: None,
            summary: None,
            message_count: 0,
            metadata: HashMap::new(),
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSegment {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub token_count: Option<i32>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ConversationSegment {
    pub fn new(conversation_id: String, role: MessageRole, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            role,
            content,
            created_at: Utc::now(),
            token_count: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortTermMemoryType {
    Observation,
    Insight,
    Fact,
    Preference,
    Task,
}

impl std::fmt::Display for ShortTermMemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortTermMemoryType::Observation => write!(f, "observation"),
            ShortTermMemoryType::Insight => write!(f, "insight"),
            ShortTermMemoryType::Fact => write!(f, "fact"),
            ShortTermMemoryType::Preference => write!(f, "preference"),
            ShortTermMemoryType::Task => write!(f, "task"),
        }
    }
}

impl std::str::FromStr for ShortTermMemoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "observation" => Ok(ShortTermMemoryType::Observation),
            "insight" => Ok(ShortTermMemoryType::Insight),
            "fact" => Ok(ShortTermMemoryType::Fact),
            "preference" => Ok(ShortTermMemoryType::Preference),
            "task" => Ok(ShortTermMemoryType::Task),
            _ => Err(format!("Invalid short-term memory type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    pub id: String,
    pub content: String,
    pub memory_type: ShortTermMemoryType,
    pub source_conversation_id: Option<String>,
    pub source_segment_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub access_count: i32,
    pub relevance_score: f64,
    pub decay_rate: f64,
    pub expires_at: Option<DateTime<Utc>>,
    pub promoted_to_ltm_id: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ShortTermMemory {
    pub fn new(content: String, memory_type: ShortTermMemoryType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            memory_type,
            source_conversation_id: None,
            source_segment_id: None,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
            access_count: 1,
            relevance_score: 1.0,
            decay_rate: 0.1,
            expires_at: None,
            promoted_to_ltm_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_source(mut self, conversation_id: String, segment_id: Option<String>) -> Self {
        self.source_conversation_id = Some(conversation_id);
        self.source_segment_id = segment_id;
        self
    }

    pub fn current_relevance(&self) -> f64 {
        let age_hours = (Utc::now() - self.created_at).num_hours() as f64;
        let decay = (-self.decay_rate * age_hours).exp();
        let access_boost = (self.access_count as f64).ln_1p() * 0.1;
        (self.relevance_score * decay + access_boost).min(1.0).max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LongTermMemoryType {
    Fact,
    Preference,
    Pattern,
    Relationship,
    Skill,
    Context,
}

impl std::fmt::Display for LongTermMemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LongTermMemoryType::Fact => write!(f, "fact"),
            LongTermMemoryType::Preference => write!(f, "preference"),
            LongTermMemoryType::Pattern => write!(f, "pattern"),
            LongTermMemoryType::Relationship => write!(f, "relationship"),
            LongTermMemoryType::Skill => write!(f, "skill"),
            LongTermMemoryType::Context => write!(f, "context"),
        }
    }
}

impl std::str::FromStr for LongTermMemoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fact" => Ok(LongTermMemoryType::Fact),
            "preference" => Ok(LongTermMemoryType::Preference),
            "pattern" => Ok(LongTermMemoryType::Pattern),
            "relationship" => Ok(LongTermMemoryType::Relationship),
            "skill" => Ok(LongTermMemoryType::Skill),
            "context" => Ok(LongTermMemoryType::Context),
            _ => Err(format!("Invalid long-term memory type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemory {
    pub id: String,
    pub content: String,
    pub memory_type: LongTermMemoryType,
    pub category: Option<String>,
    pub importance: f64,
    pub confidence: f64,
    pub source_stm_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub access_count: i32,
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LongTermMemory {
    pub fn new(content: String, memory_type: LongTermMemoryType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            memory_type,
            category: None,
            importance: 0.5,
            confidence: 1.0,
            source_stm_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed_at: Utc::now(),
            access_count: 1,
            embedding: None,
            embedding_model: None,
            metadata: HashMap::new(),
        }
    }

    pub fn from_stm(stm: &ShortTermMemory, ltm_type: LongTermMemoryType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: stm.content.clone(),
            memory_type: ltm_type,
            category: None,
            importance: stm.relevance_score,
            confidence: 1.0,
            source_stm_ids: vec![stm.id.clone()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed_at: Utc::now(),
            access_count: 1,
            embedding: None,
            embedding_model: None,
            metadata: stm.metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntityLink {
    pub memory_id: String,
    pub memory_table: String,
    pub entity_id: String,
    pub link_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub memory_id: String,
    pub content: String,
    pub memory_type: String,
    pub score: f64,
    pub source: String,
}

use serde::{Deserialize, Serialize};

use super::memory_agent::{MemoryAgentRequest, MemoryAgentResponse};
use super::pipeline::{IntakeSummary, PipelineStatus};
use crate::models::template::{
    CreateTemplatePayload, Template, TemplateIndex, UpdateTemplatePayload,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "agent", content = "request")]
pub enum AgentRequest {
    TemplateCurator(TemplateCuratorRequest),
    Intake(IntakeRequest),
    GraphQuery(GraphQueryRequest),
    EntityManager(EntityManagerRequest),
    Memory(MemoryAgentRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "agent", content = "response")]
pub enum AgentResponse {
    TemplateCurator(TemplateCuratorResponse),
    Intake(IntakeResponse),
    GraphQuery(GraphQueryResponse),
    EntityManager(EntityManagerResponse),
    Memory(MemoryAgentResponse),
}

// =============================================================================
// TemplateCuratorAgent Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TemplateCuratorRequest {
    Search {
        query: String,
        limit: Option<usize>,
    },
    ListByCategory {
        category: String,
    },
    GetSummaries {
        keys: Vec<String>,
    },
    Create {
        payload: CreateTemplatePayload,
    },
    Update {
        key: String,
        payload: UpdateTemplatePayload,
    },
    Delete {
        key: String,
    },
    Get {
        key: String,
    },
    List {
        limit: Option<usize>,
    },
    ComposeTemplate {
        description: String,
        produces_entities: Vec<String>,
        similar_to: Option<String>,
    },
    GetRelevantContext {
        user_query: String,
        max_tokens: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TemplateCuratorResponse {
    TemplateList {
        templates: Vec<TemplateIndex>,
    },
    Template {
        template: Template,
    },
    Created {
        key: String,
    },
    Updated,
    Deleted,
    Context {
        context: String,
    },
    ComposePrompt {
        system_prompt: String,
        user_prompt: String,
    },
    Error {
        message: String,
    },
}

// =============================================================================
// IntakePipelineAgent Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum IntakeRequest {
    StartSession,
    GetSystemPrompt,
    ProcessUserMessage {
        status: PipelineStatus,
        message: String,
    },
    ProcessAssistantResponse {
        status: PipelineStatus,
        response: String,
    },
    BuildGraph {
        status: PipelineStatus,
    },
    ProcessGraphResponse {
        status: PipelineStatus,
        response: String,
    },
    SyncToDb {
        status: PipelineStatus,
    },
    GetSummary {
        status: PipelineStatus,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum IntakeResponse {
    SessionStarted {
        status: PipelineStatus,
    },
    SystemPrompt {
        prompt: String,
    },
    MessageProcessed {
        status: PipelineStatus,
    },
    ConversationReply {
        status: PipelineStatus,
        reply: String,
        complete: bool,
    },
    GraphPrompt {
        system_prompt: String,
        user_prompt: String,
    },
    GraphProcessed {
        status: PipelineStatus,
    },
    Synced {
        status: PipelineStatus,
        entities: usize,
        relations: usize,
    },
    Summary {
        summary: IntakeSummary,
    },
    Error {
        message: String,
    },
}

// =============================================================================
// GraphQueryAgent Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum GraphQueryRequest {
    GetNeighbors {
        entity_id: String,
        depth: Option<usize>,
    },
    FindPath {
        from_id: String,
        to_id: String,
    },
    SearchEntities {
        query: String,
        entity_type: Option<String>,
        limit: Option<usize>,
    },
    SimilarEntities {
        entity_id: String,
        limit: Option<usize>,
    },
    GetStatistics {
        entity_type: Option<String>,
    },
    GetEntityContext {
        entity_id: String,
        max_tokens: usize,
    },
    GetRelevantEntities {
        query: String,
        max_tokens: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub status: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_entities: usize,
    pub total_relations: usize,
    pub entities_by_type: std::collections::HashMap<String, usize>,
    pub relations_by_type: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum GraphQueryResponse {
    Entities { entities: Vec<EntitySummary> },
    Path { path: Vec<String> },
    Statistics { stats: GraphStats },
    Context { context: String },
    Error { message: String },
}

// =============================================================================
// EntityManagerAgent Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum EntityManagerRequest {
    Create {
        entity_type: String,
        title: String,
        body: Option<String>,
        canonical: serde_json::Value,
    },
    Update {
        entity_id: String,
        title: Option<String>,
        body: Option<String>,
        status: Option<String>,
        canonical: Option<serde_json::Value>,
    },
    Delete {
        entity_id: String,
    },
    ChangeStatus {
        entity_id: String,
        new_status: String,
    },
    Get {
        entity_id: String,
    },
    ValidateCanonical {
        entity_type: String,
        canonical: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDetail {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub body: Option<String>,
    pub status: Option<String>,
    pub canonical: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum EntityManagerResponse {
    Created { entity_id: String },
    Updated,
    Deleted,
    StatusChanged,
    Entity { entity: EntityDetail },
    ValidationResult { valid: bool, errors: Vec<String> },
    Error { message: String },
}

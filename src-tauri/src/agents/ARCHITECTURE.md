# Gargoyle Agent Architecture

## Overview

Gargoyle uses a **multi-agent architecture** where specialized sub-agents handle discrete domains of responsibility. The **Primary Agent** (user-facing LLM) delegates to sub-agents for specific tasks, receiving concise, relevant context back.

```
┌─────────────────────────────────────────────────────────────────┐
│                        Primary Agent                             │
│                    (User-facing LLM interface)                   │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                    ┌───────▼───────┐
                    │  AgentRouter  │
                    │  (Dispatcher) │
                    └───────┬───────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│TemplateCurator│   │  IntakePipe   │   │  GraphQuery   │
│    Agent      │   │    Agent      │   │    Agent      │
└───────────────┘   └───────────────┘   └───────────────┘
```

## Design Principles

1. **Single Responsibility**: Each agent owns one domain
2. **Context Minimization**: Sub-agents return only what's needed, not everything
3. **Explicit Boundaries**: Agents cannot directly modify each other's data
4. **Stateless Operations**: Agents don't maintain session state (DB is source of truth)
5. **Composable**: Agents can call other agents through the router

---

## Agent Definitions

### 1. TemplateCuratorAgent

**Domain**: Template lifecycle management and discovery

**Responsibilities**:
- CRUD operations on templates (create, read, update, delete)
- Search templates by keyword, category, or semantic similarity
- Compose new templates from patterns and user requirements
- Provide concise template summaries for primary agent context
- Track template usage statistics

**Control Surfaces** (what it can modify):
- `templates` table (full CRUD)
- `templates_fts` (automatically via triggers)

**Interfaces**:
```rust
pub enum TemplateCuratorRequest {
    // Discovery
    Search { query: String, limit: usize },
    ListByCategory { category: String },
    GetSummaries { keys: Vec<String> },
    
    // CRUD
    Create { payload: CreateTemplatePayload },
    Update { key: String, payload: UpdateTemplatePayload },
    Delete { key: String },
    Get { key: String },
    
    // Composition
    ComposeTemplate { 
        description: String,
        produces_entities: Vec<String>,
        similar_to: Option<String>,
    },
    
    // Context for Primary Agent
    GetRelevantContext { 
        user_query: String, 
        max_tokens: usize 
    },
}

pub enum TemplateCuratorResponse {
    TemplateList(Vec<TemplateIndex>),
    Template(Template),
    Created { key: String },
    Updated,
    Deleted,
    Context(String),  // Condensed context for primary agent
    Error(String),
}
```

**Boundaries**:
- Cannot create/modify entities directly
- Cannot access chat history
- Read-only access to entity types schema (for template composition)

---

### 2. IntakePipelineAgent

**Domain**: User onboarding and context collection

**Responsibilities**:
- Conduct conversational intake interviews
- Extract structured key-value pairs from conversation
- Build knowledge graphs from collected data
- Sync graphs to entity/relation storage
- Generate intake summaries

**Control Surfaces**:
- `entities` table (create only, via intake)
- `relations` table (create only, via intake)
- `chat_sessions` / `chat_messages` (own session only)

**Interfaces**:
```rust
pub enum IntakeRequest {
    StartSession,
    ProcessUserMessage { session_id: String, message: String },
    ProcessAssistantResponse { session_id: String, response: String },
    BuildGraph { session_id: String },
    ProcessGraphResponse { session_id: String, response: String },
    SyncToDb { session_id: String },
    GetSummary { session_id: String },
}

pub enum IntakeResponse {
    SessionStarted { session_id: String, system_prompt: String },
    MessageStored,
    ConversationReply { reply: String, complete: bool },
    GraphPrompt { system_prompt: String, user_prompt: String },
    GraphBuilt,
    Synced { entities: usize, relations: usize },
    Summary(IntakeSummary),
    Error(String),
}
```

**Boundaries**:
- Cannot modify existing entities (create only)
- Cannot delete anything
- Isolated to its own chat session

---

### 3. GraphQueryAgent

**Domain**: Knowledge graph traversal and analysis

**Responsibilities**:
- Execute graph queries (neighbors, paths, clusters)
- Find related entities by various criteria
- Aggregate statistics across entity types
- Identify patterns and anomalies
- Provide graph context to primary agent

**Control Surfaces**:
- None (read-only agent)

**Interfaces**:
```rust
pub enum GraphQueryRequest {
    // Traversal
    GetNeighbors { entity_id: String, depth: usize },
    FindPath { from_id: String, to_id: String },
    GetCluster { entity_id: String },
    
    // Search
    SearchEntities { query: String, entity_type: Option<String>, limit: usize },
    SimilarEntities { entity_id: String, limit: usize },
    
    // Analysis
    GetStatistics { entity_type: Option<String> },
    FindOrphans,
    DetectDuplicates { threshold: f64 },
    
    // Context
    GetEntityContext { entity_id: String, max_tokens: usize },
    GetRelevantEntities { query: String, max_tokens: usize },
}

pub enum GraphQueryResponse {
    Entities(Vec<EntitySummary>),
    Path(Vec<String>),
    Statistics(GraphStats),
    Context(String),
    Error(String),
}
```

**Boundaries**:
- Strictly read-only
- Cannot modify entities, relations, or any other data
- Rate-limited on expensive queries

---

### 4. EntityManagerAgent

**Domain**: Entity lifecycle and mutations

**Responsibilities**:
- Create, update, delete entities
- Validate entity data against schemas
- Handle entity migrations between types
- Manage entity status transitions
- Merge duplicate entities

**Control Surfaces**:
- `entities` table (full CRUD)
- `relations` table (cascade deletes)
- `dedup_suggestions` table

**Interfaces**:
```rust
pub enum EntityManagerRequest {
    Create { entity_type: String, title: String, body: Option<String>, canonical: Value },
    Update { entity_id: String, updates: EntityUpdates },
    Delete { entity_id: String },
    ChangeStatus { entity_id: String, new_status: String },
    Migrate { entity_id: String, to_type: String },
    Merge { source_id: String, target_id: String },
    ValidateCanonical { entity_type: String, canonical: Value },
}

pub enum EntityManagerResponse {
    Created { entity_id: String },
    Updated,
    Deleted,
    StatusChanged,
    Migrated,
    Merged { kept_id: String },
    ValidationResult { valid: bool, errors: Vec<String> },
    Error(String),
}
```

**Boundaries**:
- Cannot modify templates
- Cannot access chat sessions
- Must respect schema validation

---

## Agent Router

The **AgentRouter** is the single entry point for all agent operations, replacing the sprawl of individual Tauri commands.

```rust
pub struct AgentRouter;

impl AgentRouter {
    pub fn dispatch(conn: &Connection, request: AgentRequest) -> Result<AgentResponse>;
}

pub enum AgentRequest {
    TemplateCurator(TemplateCuratorRequest),
    Intake(IntakeRequest),
    GraphQuery(GraphQueryRequest),
    EntityManager(EntityManagerRequest),
}

pub enum AgentResponse {
    TemplateCurator(TemplateCuratorResponse),
    Intake(IntakeResponse),
    GraphQuery(GraphQueryResponse),
    EntityManager(EntityManagerResponse),
}
```

### Single Tauri Command

```rust
#[tauri::command]
pub fn agent_dispatch(
    state: State<AppState>,
    request: AgentRequest,
) -> Result<AgentResponse, String> {
    let guard = state.db.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    AgentRouter::dispatch(conn, request).map_err(|e| e.to_string())
}
```

---

## Inter-Agent Communication

Agents can request context from other agents through the router:

```rust
// Example: Primary agent needs template context
let relevant_templates = AgentRouter::dispatch(conn, AgentRequest::TemplateCurator(
    TemplateCuratorRequest::GetRelevantContext {
        user_query: "help me plan a project".to_string(),
        max_tokens: 500,
    }
))?;

// Example: TemplateCurator needs entity type info for composition
let entity_types = AgentRouter::dispatch(conn, AgentRequest::GraphQuery(
    GraphQueryRequest::GetStatistics { entity_type: None }
))?;
```

---

## Context Provision Protocol

When the primary agent needs context from a sub-agent, the response should be:

1. **Concise**: Under the requested token limit
2. **Relevant**: Filtered to the query
3. **Actionable**: Includes IDs/keys needed for follow-up
4. **Structured**: Consistent format for LLM parsing

Example context response:
```
## Relevant Templates (3 of 47)
1. **project-charter** (org) - Creates project charter with scope, goals, stakeholders
2. **task-breakdown** (workflow) - Breaks down projects into actionable tasks  
3. **weekly-review** (workflow) - Weekly review and planning template

Use `get_template("key")` for full content.
```

---

## Future Agents (Planned)

| Agent | Domain | Status |
|-------|--------|--------|
| **ClaimAgent** | Fact extraction and verification | Planned |
| **ScheduleAgent** | Time-based operations and reminders | Planned |
| **ExportAgent** | Data export and reporting | Planned |
| **IntegrationAgent** | External service connections | Planned |

---

## Implementation Priority

1. **AgentRouter** - Single dispatch point
2. **TemplateCuratorAgent** - Template lifecycle
3. **Refactor existing IntakePipelineAgent** - Conform to new interface
4. **GraphQueryAgent** - Read-only graph operations
5. **EntityManagerAgent** - Entity mutations

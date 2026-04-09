use std::sync::{Arc, Mutex};

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, InitializeResult, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use rusqlite::Connection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::models::patch::{
    CreateEntityPayload, CreateRelationPayload, PatchOp, PatchSet,
    UpdateEntityPayload,
};
use crate::services::{
    claim_service::ClaimService, context_manager::ContextManager, dedup::DedupPipeline,
    graph_builder, indexer::IndexerService, store::StoreService,
    template_service::TemplateService,
};

// ── Helper ──────────────────────────────────────────────────────────

fn err(msg: String) -> McpError {
    McpError::internal_error(msg, None)
}

fn json_result<T: Serialize>(v: T) -> Result<CallToolResult, McpError> {
    let content = Content::json(v).map_err(|e| err(format!("{e}")))?;
    Ok(CallToolResult::success(vec![content]))
}

fn text_result(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

// ── Parameter structs (schemars for MCP tool schemas) ───────────────

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for creating an entity")]
pub struct CreateEntityParams {
    #[schemars(description = "Entity type (e.g. person, project, note)")]
    pub entity_type: String,
    #[schemars(description = "Entity title")]
    pub title: String,
    #[schemars(description = "Source: manual, clipboard, web, import, agent, template, bootstrap")]
    pub source: String,
    #[schemars(description = "Type-specific structured fields as JSON object")]
    pub canonical_fields: serde_json::Value,
    #[schemars(description = "Optional markdown body")]
    pub body_md: Option<String>,
    #[schemars(description = "Optional status (e.g. active, archived)")]
    pub status: Option<String>,
    #[schemars(description = "Optional category")]
    pub category: Option<String>,
    #[schemars(description = "Optional priority (integer)")]
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for getting an entity by ID")]
pub struct GetEntityParams {
    #[schemars(description = "Entity UUID")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for listing entities")]
pub struct ListEntitiesParams {
    #[schemars(description = "Optional entity type filter")]
    pub entity_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for updating an entity")]
pub struct UpdateEntityParams {
    #[schemars(description = "Entity UUID to update")]
    pub entity_id: String,
    #[schemars(description = "Current updated_at timestamp for optimistic locking")]
    pub expected_updated_at: String,
    #[schemars(description = "New title")]
    pub title: Option<String>,
    #[schemars(description = "New markdown body")]
    pub body_md: Option<String>,
    #[schemars(description = "New status")]
    pub status: Option<String>,
    #[schemars(description = "New canonical fields")]
    pub canonical_fields: Option<serde_json::Value>,
    #[schemars(description = "New category")]
    pub category: Option<String>,
    #[schemars(description = "New priority")]
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for deleting an entity")]
pub struct DeleteEntityParams {
    #[schemars(description = "Entity UUID to soft-delete")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for creating a relation between entities")]
pub struct CreateRelationParams {
    #[schemars(description = "Source entity UUID")]
    pub from_id: String,
    #[schemars(description = "Target entity UUID")]
    pub to_id: String,
    #[schemars(description = "Relation type (e.g. related_to, depends_on)")]
    pub relation_type: String,
    #[schemars(description = "Optional weight (0.0-1.0)")]
    pub weight: Option<f64>,
    #[schemars(description = "Optional confidence (0.0-1.0)")]
    pub confidence: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for getting relations of an entity")]
pub struct GetRelationsParams {
    #[schemars(description = "Entity UUID")]
    pub entity_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for full-text search")]
pub struct SearchFtsParams {
    #[schemars(description = "Search query string")]
    pub query: String,
    #[schemars(description = "Maximum results to return (default 10)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for semantic similarity search")]
pub struct SearchSimilarParams {
    #[schemars(description = "Query text to find similar entities")]
    pub query: String,
    #[schemars(description = "Maximum results to return (default 10)")]
    pub limit: Option<usize>,
    #[schemars(description = "Minimum similarity threshold (0.0-1.0)")]
    pub threshold: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for graph traversal")]
pub struct GetEntityGraphParams {
    #[schemars(description = "Starting entity UUID")]
    pub entity_id: String,
    #[schemars(description = "Traversal depth (default 2)")]
    pub depth: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for listing claims")]
pub struct ListClaimsParams {
    #[schemars(description = "Optional: filter by evidence entity UUID")]
    pub evidence_entity_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for getting a specific claim")]
pub struct GetClaimParams {
    #[schemars(description = "Claim UUID")]
    pub claim_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for promoting a claim to an entity")]
pub struct PromoteClaimParams {
    #[schemars(description = "Claim UUID to promote")]
    pub claim_id: String,
    #[schemars(description = "Target entity type")]
    pub entity_type: String,
    #[schemars(description = "Source attribution")]
    pub source: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for listing templates")]
pub struct ListTemplatesParams {
    #[schemars(description = "Optional category filter")]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for getting a template by key")]
pub struct GetTemplateParams {
    #[schemars(description = "Template key")]
    pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for getting operational context")]
pub struct GetContextParams {
    #[schemars(description = "Context key")]
    pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for setting operational context")]
pub struct SetContextParams {
    #[schemars(description = "Context key")]
    pub key: String,
    #[schemars(description = "Context value (JSON)")]
    pub value: serde_json::Value,
    #[schemars(description = "Optional run ID for provenance")]
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for applying a batch of patch operations")]
pub struct ApplyPatchSetParams {
    #[schemars(description = "Run ID for provenance tracking")]
    pub run_id: String,
    #[schemars(description = "Array of patch operations")]
    pub ops: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for checking duplicates")]
pub struct CheckDuplicatesParams {
    #[schemars(description = "Entity UUID to check for duplicates")]
    pub entity_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Parameters for listing dedup suggestions")]
pub struct ListDedupSuggestionsParams {
    #[schemars(description = "Optional status filter (e.g. pending, accepted, rejected)")]
    pub status: Option<String>,
}

// ── MCP Server ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GargoyleMcp {
    db: Arc<Mutex<Connection>>,
    tool_router: ToolRouter<Self>,
}

impl GargoyleMcp {
    pub fn new(conn: Connection) -> Self {
        let db = Arc::new(Mutex::new(conn));
        Self {
            db,
            tool_router: Self::tool_router(),
        }
    }
}

// ── Tool implementations ────────────────────────────────────────────

#[tool_router]
impl GargoyleMcp {
    // ── Entity CRUD ─────────────────────────────────────────────────

    #[tool(description = "Create a typed entity in the knowledge graph with canonical fields")]
    async fn create_entity(
        &self,
        params: Parameters<CreateEntityParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let payload = CreateEntityPayload {
            entity_type: p.entity_type,
            title: p.title,
            source: p.source,
            canonical_fields: p.canonical_fields,
            body_md: p.body_md,
            status: p.status,
            category: p.category,
            priority: p.priority,
            reason: None,
        };
        let result = StoreService::create_entity(&conn, payload).map_err(|e| err(format!("{e}")))?;
        json_result(result)
    }

    #[tool(description = "Get an entity by its UUID")]
    async fn get_entity(
        &self,
        params: Parameters<GetEntityParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let entity =
            StoreService::get_entity(&conn, &params.0.id).map_err(|e| err(format!("{e}")))?;
        json_result(entity)
    }

    #[tool(description = "List all entities, optionally filtered by entity type")]
    async fn list_entities(
        &self,
        params: Parameters<ListEntitiesParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let entities = StoreService::list_entities(&conn, params.0.entity_type.as_deref())
            .map_err(|e| err(format!("{e}")))?;
        json_result(entities)
    }

    #[tool(description = "Update an entity's fields (uses optimistic locking via expected_updated_at)")]
    async fn update_entity(
        &self,
        params: Parameters<UpdateEntityParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let payload = UpdateEntityPayload {
            entity_id: p.entity_id,
            expected_updated_at: p.expected_updated_at,
            title: p.title,
            body_md: p.body_md,
            status: p.status,
            canonical_fields: p.canonical_fields,
            category: p.category,
            priority: p.priority,
            reason: None,
        };
        let result = StoreService::update_entity(&conn, payload).map_err(|e| err(format!("{e}")))?;
        json_result(result)
    }

    #[tool(description = "Soft-delete an entity by UUID")]
    async fn delete_entity(
        &self,
        params: Parameters<DeleteEntityParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = &params.0.id;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        StoreService::delete_entity(&conn, id).map_err(|e| err(format!("{e}")))?;
        text_result(format!("Entity {id} deleted"))
    }

    // ── Relations ───────────────────────────────────────────────────

    #[tool(description = "Create a typed relation (edge) between two entities")]
    async fn create_relation(
        &self,
        params: Parameters<CreateRelationParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let payload = CreateRelationPayload {
            from_id: p.from_id,
            to_id: p.to_id,
            relation_type: p.relation_type,
            weight: p.weight,
            confidence: p.confidence,
            provenance_run_id: None,
            reason: None,
        };
        let result =
            StoreService::create_relation(&conn, payload).map_err(|e| err(format!("{e}")))?;
        json_result(result)
    }

    #[tool(description = "Get all relations for an entity (both incoming and outgoing)")]
    async fn get_relations(
        &self,
        params: Parameters<GetRelationsParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let relations = StoreService::get_relations(&conn, &params.0.entity_id)
            .map_err(|e| err(format!("{e}")))?;
        json_result(relations)
    }

    // ── Search ──────────────────────────────────────────────────────

    #[tool(description = "Full-text search across all entities (title, body, canonical fields)")]
    async fn search_fts(
        &self,
        params: Parameters<SearchFtsParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let limit = p.limit.unwrap_or(10);
        let results =
            IndexerService::search_fts(&conn, &p.query, limit).map_err(|e| err(format!("{e}")))?;
        json_result(results)
    }

    #[tool(description = "Semantic similarity search using embeddings (requires embedding endpoint)")]
    async fn search_similar(
        &self,
        params: Parameters<SearchSimilarParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let limit = p.limit.unwrap_or(10);

        // Step 1: embed the query text (async, no DB needed)
        let (query_vector, _, _) = IndexerService::embed_text_async(&p.query)
            .await
            .map_err(|e| err(format!("{e}")))?;

        // Step 2: scan DB embeddings (sync, needs conn)
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let results = IndexerService::search_similar_with_vector(&conn, &query_vector, limit, p.threshold)
            .map_err(|e| err(format!("{e}")))?;
        json_result(results)
    }

    // ── Graph ───────────────────────────────────────────────────────

    #[tool(description = "BFS traversal of the entity graph from a starting node")]
    async fn get_entity_graph(
        &self,
        params: Parameters<GetEntityGraphParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let depth = p.depth.unwrap_or(2);
        let graph = graph_builder::get_entity_graph(&conn, &p.entity_id, depth)
            .map_err(|e| err(format!("{e}")))?;
        json_result(graph)
    }

    #[tool(description = "Audit all relation types for consistency and report issues")]
    async fn audit_relations(&self) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let audit = graph_builder::audit_related_to(&conn).map_err(|e| err(format!("{e}")))?;
        json_result(audit)
    }

    // ── Claims ──────────────────────────────────────────────────────

    #[tool(description = "List claims (subject-predicate-object triples), optionally filtered by evidence entity")]
    async fn list_claims(
        &self,
        params: Parameters<ListClaimsParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let claims = ClaimService::list_claims(&conn, params.0.evidence_entity_id.as_deref())
            .map_err(|e| err(format!("{e}")))?;
        json_result(claims)
    }

    #[tool(description = "Get a specific claim by UUID")]
    async fn get_claim(
        &self,
        params: Parameters<GetClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let claim =
            ClaimService::get_claim(&conn, &params.0.claim_id).map_err(|e| err(format!("{e}")))?;
        json_result(claim)
    }

    #[tool(description = "Promote a claim to a full entity in the knowledge graph")]
    async fn promote_claim(
        &self,
        params: Parameters<PromoteClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let entity_id =
            ClaimService::promote_claim(&conn, &p.claim_id, &p.entity_type, &p.source)
                .map_err(|e| err(format!("{e}")))?;
        text_result(format!("Claim promoted to entity {entity_id}"))
    }

    // ── Templates ───────────────────────────────────────────────────

    #[tool(description = "List available templates, optionally filtered by category")]
    async fn list_templates(
        &self,
        params: Parameters<ListTemplatesParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let templates = TemplateService::list(&conn, params.0.category.as_deref())
            .map_err(|e| err(format!("{e}")))?;
        json_result(templates)
    }

    #[tool(description = "Get a template definition by its key")]
    async fn get_template(
        &self,
        params: Parameters<GetTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let template =
            TemplateService::get_by_key(&conn, &params.0.key).map_err(|e| err(format!("{e}")))?;
        json_result(template)
    }

    // ── Context ─────────────────────────────────────────────────────

    #[tool(description = "Get operational context value by key")]
    async fn get_context(
        &self,
        params: Parameters<GetContextParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let ctx = ContextManager::get(&conn, &params.0.key).map_err(|e| err(format!("{e}")))?;
        json_result(ctx)
    }

    #[tool(description = "Set operational context (key-value store for workflow state)")]
    async fn set_context(
        &self,
        params: Parameters<SetContextParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        ContextManager::set(&conn, &p.key, &p.value, p.run_id.as_deref())
            .map_err(|e| err(format!("{e}")))?;
        text_result(format!("Context '{}' set", p.key))
    }

    // ── Dedup ───────────────────────────────────────────────────────

    #[tool(description = "Check an entity for potential duplicates in the knowledge graph")]
    async fn check_duplicates(
        &self,
        params: Parameters<CheckDuplicatesParams>,
    ) -> Result<CallToolResult, McpError> {
        let entity_id = params.0.entity_id;

        // Read entity text and generate embedding BEFORE the long DB lock.
        // This splits the async embed call from the sync DB operations.
        let text = {
            let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
            IndexerService::read_entity_text(&conn, &entity_id)
                .map_err(|e| err(format!("{e}")))?
        }; // conn dropped here

        // Async embed (no conn held)
        let embed_result = IndexerService::embed_text_async(&text).await.ok();

        // Now run dedup with pre-computed embedding
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;

        // Store embedding if we got one
        if let Some((vector, model, dims)) = &embed_result {
            let _ = IndexerService::store_embedding(&conn, &entity_id, vector, model, *dims);
        }

        // Run the sync dedup pipeline (Stage 3 will find the embedding we just stored)
        let suggestions = DedupPipeline::check_for_duplicates(&conn, &entity_id)
            .map_err(|e| err(format!("{e}")))?;
        json_result(suggestions)
    }

    #[tool(description = "List dedup suggestions, optionally filtered by status")]
    async fn list_dedup_suggestions(
        &self,
        params: Parameters<ListDedupSuggestionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let suggestions = DedupPipeline::get_suggestions(&conn, params.0.status.as_deref())
            .map_err(|e| err(format!("{e}")))?;
        json_result(suggestions)
    }

    // ── Patch ───────────────────────────────────────────────────────

    #[tool(description = "Apply a batch of atomic patch operations (create/update entities, relations, claims)")]
    async fn apply_patch_set(
        &self,
        params: Parameters<ApplyPatchSetParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let conn = self.db.lock().map_err(|e| err(format!("{e}")))?;
        let ops: Vec<PatchOp> = p
            .ops
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(|e| err(format!("Invalid patch op: {e}"))))
            .collect::<Result<Vec<_>, _>>()?;
        let patch_set = PatchSet {
            ops,
            run_id: p.run_id,
        };
        let result =
            StoreService::apply_patch_set(&conn, &patch_set).map_err(|e| err(format!("{e}")))?;
        json_result(result)
    }
}

// ── ServerHandler ───────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for GargoyleMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        .with_server_info(Implementation::new("gargoyle-mcp", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "Gargoyle knowledge graph MCP server. Provides tools for managing entities, \
             relations, claims, templates, and operational context in a typed knowledge graph. \
             Use search_fts for keyword search, search_similar for semantic search, \
             get_entity_graph for graph traversal.",
        )
    }

    async fn initialize(
        &self,
        _request: rmcp::model::InitializeRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}

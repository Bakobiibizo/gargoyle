// Tool executor: defines LLM tool schemas and dispatches tool calls against the database.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{GargoyleError, Result};
use crate::models::patch::{CreateEntityPayload, CreateRelationPayload, UpdateEntityPayload};
use crate::services::indexer::IndexerService;
use crate::services::llm::{FunctionDefinition, ToolDefinition};
use crate::services::store::StoreService;
use crate::services::template_runner::{self, TemplateInput};

/// Return all tool definitions for the LLM function-calling protocol.
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_entities".to_string(),
                description: "Full-text search across all entities in the knowledge graph.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query text" },
                        "limit": { "type": "integer", "description": "Max results to return (default 10)", "default": 10 }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_entities".to_string(),
                description: "List entities, optionally filtered by type. Returns id, title, type, and status.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "entity_type": { "type": "string", "description": "Filter by entity type (e.g. task, note, project, metric). Omit to list all." }
                    }
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_entity".to_string(),
                description: "Get full details of an entity by its ID.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The entity ID" }
                    },
                    "required": ["id"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "create_entity".to_string(),
                description: "Create a new entity in the knowledge graph.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "entity_type": { "type": "string", "description": "Type of entity (task, note, project, decision, metric, experiment, budget, campaign, event, person, spec, session, playbook, policy, competitor, vendor, audience, channel, backlog, taxonomy, brief, result, artifact_type, commitment, concept, inbox_item, issue)" },
                        "title": { "type": "string", "description": "Title of the entity" },
                        "body_md": { "type": "string", "description": "Markdown body content (optional)" },
                        "status": { "type": "string", "description": "Initial status (optional, type-dependent)" },
                        "canonical_fields": { "type": "object", "description": "Type-specific structured fields (optional)" }
                    },
                    "required": ["entity_type", "title"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "update_entity".to_string(),
                description: "Update an existing entity. Only include fields you want to change.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "entity_id": { "type": "string", "description": "ID of the entity to update" },
                        "title": { "type": "string", "description": "New title (optional)" },
                        "body_md": { "type": "string", "description": "New markdown body (optional)" },
                        "status": { "type": "string", "description": "New status (optional)" },
                        "canonical_fields": { "type": "object", "description": "Updated type-specific fields (optional, merged with existing)" }
                    },
                    "required": ["entity_id"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "create_relation".to_string(),
                description: "Create a directional relation (link) between two entities.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "from_id": { "type": "string", "description": "Source entity ID" },
                        "to_id": { "type": "string", "description": "Target entity ID" },
                        "relation_type": { "type": "string", "description": "Type of relation (e.g. related_to, depends_on, parent_of, derived_from, informs, owned_by, blocked_by, tracks)" },
                        "weight": { "type": "number", "description": "Relation weight 0.0-1.0 (default 1.0)" }
                    },
                    "required": ["from_id", "to_id", "relation_type"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "run_template".to_string(),
                description: "Execute a registered template to generate entities and relations.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "template_key": { "type": "string", "description": "Template key to run" },
                        "params": { "type": "object", "description": "Template parameters (varies by template)" }
                    },
                    "required": ["template_key", "params"]
                }),
            },
        },
    ]
}

/// Execute a single tool call and return a JSON string result.
pub fn execute_tool(conn: &Connection, tool_name: &str, args_json: &str) -> Result<String> {
    let args: Value = serde_json::from_str(args_json)
        .map_err(|e| GargoyleError::Schema(format!("Invalid tool arguments JSON: {}", e)))?;

    match tool_name {
        "search_entities" => exec_search_entities(conn, &args),
        "list_entities" => exec_list_entities(conn, &args),
        "get_entity" => exec_get_entity(conn, &args),
        "create_entity" => exec_create_entity(conn, &args),
        "update_entity" => exec_update_entity(conn, &args),
        "create_relation" => exec_create_relation(conn, &args),
        "run_template" => exec_run_template(conn, &args),
        _ => Err(GargoyleError::Schema(format!("Unknown tool: {}", tool_name))),
    }
}

fn exec_search_entities(conn: &Connection, args: &Value) -> Result<String> {
    let query = args["query"].as_str()
        .ok_or_else(|| GargoyleError::Schema("search_entities requires 'query' string".into()))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    let results = IndexerService::search_fts(conn, query, limit)?;
    let output: Vec<Value> = results.iter().map(|r| json!({
        "entity_id": r.entity_id,
        "title": r.title,
        "entity_type": r.entity_type,
        "score": r.score,
    })).collect();

    Ok(serde_json::to_string(&output)?)
}

fn exec_list_entities(conn: &Connection, args: &Value) -> Result<String> {
    let entity_type = args["entity_type"].as_str();
    let entities = StoreService::list_entities(conn, entity_type)?;

    let output: Vec<Value> = entities.iter().map(|e| json!({
        "id": e.id,
        "title": e.title,
        "entity_type": e.entity_type,
        "status": e.status,
        "created_at": e.created_at,
    })).collect();

    Ok(serde_json::to_string(&output)?)
}

fn exec_get_entity(conn: &Connection, args: &Value) -> Result<String> {
    let id = args["id"].as_str()
        .ok_or_else(|| GargoyleError::Schema("get_entity requires 'id' string".into()))?;

    let entity = StoreService::get_entity(conn, id)?;
    let relations = StoreService::get_relations(conn, id)?;

    let rel_output: Vec<Value> = relations.iter().map(|r| json!({
        "id": r.id,
        "from_id": r.from_id,
        "to_id": r.to_id,
        "relation_type": r.relation_type,
        "weight": r.weight,
    })).collect();

    let output = json!({
        "id": entity.id,
        "entity_type": entity.entity_type,
        "title": entity.title,
        "body_md": entity.body_md,
        "status": entity.status,
        "category": entity.category,
        "priority": entity.priority,
        "canonical_fields": entity.canonical_fields,
        "created_at": entity.created_at,
        "updated_at": entity.updated_at,
        "relations": rel_output,
    });

    Ok(serde_json::to_string(&output)?)
}

fn exec_create_entity(conn: &Connection, args: &Value) -> Result<String> {
    let entity_type = args["entity_type"].as_str()
        .ok_or_else(|| GargoyleError::Schema("create_entity requires 'entity_type'".into()))?;
    let title = args["title"].as_str()
        .ok_or_else(|| GargoyleError::Schema("create_entity requires 'title'".into()))?;

    let payload = CreateEntityPayload {
        entity_type: entity_type.to_string(),
        title: title.to_string(),
        source: "agent".to_string(),
        canonical_fields: args.get("canonical_fields").cloned().unwrap_or(json!({})),
        body_md: args["body_md"].as_str().map(|s| s.to_string()),
        status: args["status"].as_str().map(|s| s.to_string()),
        category: None,
        priority: None,
        reason: None,
    };

    let result = StoreService::create_entity(conn, payload)?;

    let entity_id = result.applied.first()
        .and_then(|a| a.entity_id.clone())
        .unwrap_or_default();

    Ok(serde_json::to_string(&json!({
        "success": true,
        "entity_id": entity_id,
        "message": format!("Created {} '{}'", entity_type, title),
    }))?)
}

fn exec_update_entity(conn: &Connection, args: &Value) -> Result<String> {
    let entity_id = args["entity_id"].as_str()
        .ok_or_else(|| GargoyleError::Schema("update_entity requires 'entity_id'".into()))?;

    // Auto-fetch expected_updated_at for optimistic locking
    let entity = StoreService::get_entity(conn, entity_id)?;

    let payload = UpdateEntityPayload {
        entity_id: entity_id.to_string(),
        expected_updated_at: entity.updated_at.clone(),
        title: args["title"].as_str().map(|s| s.to_string()),
        body_md: args["body_md"].as_str().map(|s| s.to_string()),
        status: args["status"].as_str().map(|s| s.to_string()),
        canonical_fields: args.get("canonical_fields").cloned(),
        category: None,
        priority: None,
        reason: None,
    };

    StoreService::update_entity(conn, payload)?;

    Ok(serde_json::to_string(&json!({
        "success": true,
        "entity_id": entity_id,
        "message": format!("Updated entity '{}'", entity.title),
    }))?)
}

fn exec_create_relation(conn: &Connection, args: &Value) -> Result<String> {
    let from_id = args["from_id"].as_str()
        .ok_or_else(|| GargoyleError::Schema("create_relation requires 'from_id'".into()))?;
    let to_id = args["to_id"].as_str()
        .ok_or_else(|| GargoyleError::Schema("create_relation requires 'to_id'".into()))?;
    let relation_type = args["relation_type"].as_str()
        .ok_or_else(|| GargoyleError::Schema("create_relation requires 'relation_type'".into()))?;

    let payload = CreateRelationPayload {
        from_id: from_id.to_string(),
        to_id: to_id.to_string(),
        relation_type: relation_type.to_string(),
        weight: args["weight"].as_f64(),
        confidence: None,
        provenance_run_id: None,
        reason: None,
    };

    let result = StoreService::create_relation(conn, payload)?;
    let relation_id = result.applied.first()
        .and_then(|a| a.relation_id.clone())
        .unwrap_or_default();

    Ok(serde_json::to_string(&json!({
        "success": true,
        "relation_id": relation_id,
        "message": format!("Created {} relation from {} to {}", relation_type, from_id, to_id),
    }))?)
}

fn exec_run_template(conn: &Connection, args: &Value) -> Result<String> {
    let template_key = args["template_key"].as_str()
        .ok_or_else(|| GargoyleError::Schema("run_template requires 'template_key'".into()))?;
    let params = args.get("params").cloned().unwrap_or(json!({}));

    let input = TemplateInput {
        template_key: template_key.to_string(),
        params,
        force: false,
    };

    let output = template_runner::run_template_full(conn, &input)?;

    Ok(serde_json::to_string(&json!({
        "success": true,
        "run_id": output.run_id,
        "template_key": output.template_key,
        "produced_entities": output.produced_entities.len(),
        "produced_relations": output.produced_relations.len(),
        "action_items": output.action_items,
    }))?)
}

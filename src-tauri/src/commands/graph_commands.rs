// Tauri commands for graph queries

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::services::graph_builder;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %entity_id, depth = ?depth))]
pub fn get_entity_graph(
    state: State<'_, AppState>,
    entity_id: String,
    depth: Option<usize>,
) -> Result<serde_json::Value> {
    debug!("Fetching entity graph");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let graph = graph_builder::get_entity_graph(conn, &entity_id, depth.unwrap_or(1))?;
    debug!(
        node_count = graph.nodes.len(),
        edge_count = graph.edges.len(),
        "Entity graph fetched"
    );
    serde_json::to_value(graph).map_err(|e| e.into())
}

#[tauri::command]
#[instrument(skip(state))]
pub fn audit_related_to(state: State<'_, AppState>) -> Result<serde_json::Value> {
    info!("Auditing related_to relations");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = graph_builder::audit_related_to(conn)?;
    info!("Related_to audit complete");
    serde_json::to_value(result).map_err(|e| e.into())
}

#[tauri::command]
#[instrument(skip(state))]
pub fn rebuild_projection(state: State<'_, AppState>) -> Result<serde_json::Value> {
    info!("Rebuilding graph projection");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = graph_builder::rebuild_projection(conn)?;
    info!("Graph projection rebuilt");
    serde_json::to_value(result).map_err(|e| e.into())
}

#[tauri::command]
#[instrument(skip(state), fields(type_key = %type_key))]
pub fn approve_custom_relation_type(
    state: State<'_, AppState>,
    type_key: String,
    description: String,
    expected_from_types: Option<Vec<String>>,
    expected_to_types: Option<Vec<String>>,
) -> Result<()> {
    info!("Approving custom relation type");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let from_types = expected_from_types.unwrap_or_default();
    let to_types = expected_to_types.unwrap_or_default();
    let result =
        graph_builder::approve_custom_type(conn, &type_key, &description, &from_types, &to_types);
    match &result {
        Ok(_) => info!("Custom relation type approved"),
        Err(e) => error!(error = %e, "Failed to approve custom relation type"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(relation_count = relation_ids.len(), new_type = %new_relation_type))]
pub fn reclassify_relations(
    state: State<'_, AppState>,
    relation_ids: Vec<String>,
    new_relation_type: String,
) -> Result<usize> {
    info!("Reclassifying relations");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result =
        graph_builder::reclassify_relations(conn, "related_to", &new_relation_type, &relation_ids);
    match &result {
        Ok(count) => info!(reclassified_count = count, "Relations reclassified"),
        Err(e) => error!(error = %e, "Failed to reclassify relations"),
    }
    result
}

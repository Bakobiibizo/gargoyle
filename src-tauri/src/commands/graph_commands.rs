// Tauri commands for graph queries

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::services::graph_builder;
use crate::AppState;

#[tauri::command]
pub fn get_entity_graph(
    state: State<'_, AppState>,
    entity_id: String,
    depth: Option<usize>,
) -> Result<serde_json::Value> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let graph = graph_builder::get_entity_graph(conn, &entity_id, depth.unwrap_or(1))?;
    serde_json::to_value(graph).map_err(|e| e.into())
}

#[tauri::command]
pub fn audit_related_to(
    state: State<'_, AppState>,
) -> Result<serde_json::Value> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = graph_builder::audit_related_to(conn)?;
    serde_json::to_value(result).map_err(|e| e.into())
}

#[tauri::command]
pub fn rebuild_projection(
    state: State<'_, AppState>,
) -> Result<serde_json::Value> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = graph_builder::rebuild_projection(conn)?;
    serde_json::to_value(result).map_err(|e| e.into())
}

#[tauri::command]
pub fn approve_custom_relation_type(
    state: State<'_, AppState>,
    type_key: String,
    description: String,
    expected_from_types: Option<Vec<String>>,
    expected_to_types: Option<Vec<String>>,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let from_types = expected_from_types.unwrap_or_default();
    let to_types = expected_to_types.unwrap_or_default();
    graph_builder::approve_custom_type(
        conn,
        &type_key,
        &description,
        &from_types,
        &to_types,
    )
}

#[tauri::command]
pub fn reclassify_relations(
    state: State<'_, AppState>,
    relation_ids: Vec<String>,
    new_relation_type: String,
) -> Result<usize> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    graph_builder::reclassify_relations(conn, "related_to", &new_relation_type, &relation_ids)
}

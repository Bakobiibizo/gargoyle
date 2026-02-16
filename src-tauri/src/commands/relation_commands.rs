// Tauri commands for relations

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::models::patch::{CreateRelationPayload, PatchResult};
use crate::models::relation::Relation;
use crate::services::store::StoreService;
use crate::AppState;

#[tauri::command]
pub fn create_relation(
    state: State<'_, AppState>,
    payload: CreateRelationPayload,
) -> Result<PatchResult> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::create_relation(conn, payload)
}

#[tauri::command]
pub fn get_relations(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<Vec<Relation>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::get_relations(conn, &entity_id)
}

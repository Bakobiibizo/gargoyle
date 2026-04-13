// Tauri commands for relations

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::models::patch::{CreateRelationPayload, PatchResult};
use crate::models::relation::Relation;
use crate::services::store::StoreService;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(from_id = %payload.from_id, to_id = %payload.to_id, relation_type = %payload.relation_type))]
pub fn create_relation(
    state: State<'_, AppState>,
    payload: CreateRelationPayload,
) -> Result<PatchResult> {
    info!("Creating relation");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::create_relation(conn, payload);
    match &result {
        Ok(r) => info!(applied_count = r.applied.len(), "Relation created"),
        Err(e) => error!(error = %e, "Failed to create relation"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %entity_id))]
pub fn get_relations(state: State<'_, AppState>, entity_id: String) -> Result<Vec<Relation>> {
    debug!("Fetching relations");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::get_relations(conn, &entity_id);
    match &result {
        Ok(relations) => debug!(count = relations.len(), "Relations fetched"),
        Err(e) => error!(error = %e, "Failed to fetch relations"),
    }
    result
}

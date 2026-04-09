// Tauri commands for entity CRUD and schema migration

use tauri::State;
use tracing::{debug, error, info, instrument, warn};

use crate::error::{GargoyleError, Result};
use crate::models::entity::Entity;
use crate::models::patch::{CreateEntityPayload, PatchResult, PatchSet, UpdateEntityPayload};
use crate::schema::SchemaMigrator;
use crate::services::store::StoreService;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(entity_type = %payload.entity_type, title = %payload.title))]
pub fn create_entity(
    state: State<'_, AppState>,
    payload: CreateEntityPayload,
) -> Result<PatchResult> {
    info!("Creating entity");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::create_entity(conn, payload);
    match &result {
        Ok(r) => info!(
            applied_count = r.applied.len(),
            "Entity created successfully"
        ),
        Err(e) => error!(error = %e, "Failed to create entity"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %payload.entity_id))]
pub fn update_entity(
    state: State<'_, AppState>,
    payload: UpdateEntityPayload,
) -> Result<PatchResult> {
    info!("Updating entity");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::update_entity(conn, payload);
    match &result {
        Ok(_) => info!("Entity updated successfully"),
        Err(e) => error!(error = %e, "Failed to update entity"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %id))]
pub fn get_entity(state: State<'_, AppState>, id: String) -> Result<Entity> {
    debug!("Fetching entity");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::get_entity(conn, &id);
    match &result {
        Ok(e) => debug!(entity_type = %e.entity_type, title = %e.title, "Entity fetched"),
        Err(e) => warn!(error = %e, "Entity not found"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_type = ?entity_type))]
pub fn list_entities(
    state: State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<Vec<Entity>> {
    debug!("Listing entities");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::list_entities(conn, entity_type.as_deref());
    match &result {
        Ok(entities) => debug!(count = entities.len(), "Entities listed"),
        Err(e) => error!(error = %e, "Failed to list entities"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %id))]
pub fn delete_entity(state: State<'_, AppState>, id: String) -> Result<()> {
    info!("Deleting entity");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::delete_entity(conn, &id);
    match &result {
        Ok(_) => info!("Entity deleted successfully"),
        Err(e) => error!(error = %e, "Failed to delete entity"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state, patch_set), fields(patch_count = patch_set.ops.len()))]
pub fn apply_patch_set(state: State<'_, AppState>, patch_set: PatchSet) -> Result<PatchResult> {
    info!("Applying patch set");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::apply_patch_set(conn, &patch_set);
    match &result {
        Ok(r) => info!(applied_count = r.applied.len(), "Patch set applied"),
        Err(e) => error!(error = %e, "Failed to apply patch set"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %entity_id))]
pub fn migrate_entity(state: State<'_, AppState>, entity_id: String) -> Result<()> {
    info!("Migrating entity schema");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = SchemaMigrator::migrate_entity(conn, &entity_id);
    match &result {
        Ok(_) => info!("Entity migrated successfully"),
        Err(e) => error!(error = %e, "Failed to migrate entity"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_type = %entity_type))]
pub fn migrate_all_entities(state: State<'_, AppState>, entity_type: String) -> Result<usize> {
    info!("Migrating all entities of type");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = SchemaMigrator::migrate_all_entities(conn, &entity_type);
    match &result {
        Ok(count) => info!(migrated_count = count, "Entities migrated"),
        Err(e) => error!(error = %e, "Failed to migrate entities"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_type = %entity_type))]
pub fn find_stale_entities(
    state: State<'_, AppState>,
    entity_type: String,
) -> Result<Vec<(String, i32)>> {
    debug!("Finding stale entities");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = SchemaMigrator::find_stale_entities(conn, &entity_type);
    match &result {
        Ok(stale) => debug!(stale_count = stale.len(), "Stale entities found"),
        Err(e) => error!(error = %e, "Failed to find stale entities"),
    }
    result
}

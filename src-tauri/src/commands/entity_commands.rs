// Tauri commands for entity CRUD and schema migration

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::models::entity::Entity;
use crate::models::patch::{
    CreateEntityPayload, PatchResult, PatchSet, UpdateEntityPayload,
};
use crate::schema::SchemaMigrator;
use crate::services::store::StoreService;
use crate::AppState;

#[tauri::command]
pub fn create_entity(
    state: State<'_, AppState>,
    payload: CreateEntityPayload,
) -> Result<PatchResult> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::create_entity(conn, payload)
}

#[tauri::command]
pub fn update_entity(
    state: State<'_, AppState>,
    payload: UpdateEntityPayload,
) -> Result<PatchResult> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::update_entity(conn, payload)
}

#[tauri::command]
pub fn get_entity(
    state: State<'_, AppState>,
    id: String,
) -> Result<Entity> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::get_entity(conn, &id)
}

#[tauri::command]
pub fn list_entities(
    state: State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<Vec<Entity>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::list_entities(conn, entity_type.as_deref())
}

#[tauri::command]
pub fn delete_entity(
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::delete_entity(conn, &id)
}

#[tauri::command]
pub fn apply_patch_set(
    state: State<'_, AppState>,
    patch_set: PatchSet,
) -> Result<PatchResult> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::apply_patch_set(conn, &patch_set)
}

#[tauri::command]
pub fn migrate_entity(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    SchemaMigrator::migrate_entity(conn, &entity_id)
}

#[tauri::command]
pub fn migrate_all_entities(
    state: State<'_, AppState>,
    entity_type: String,
) -> Result<usize> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    SchemaMigrator::migrate_all_entities(conn, &entity_type)
}

#[tauri::command]
pub fn find_stale_entities(
    state: State<'_, AppState>,
    entity_type: String,
) -> Result<Vec<(String, i32)>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    SchemaMigrator::find_stale_entities(conn, &entity_type)
}

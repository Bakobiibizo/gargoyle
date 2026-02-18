use crate::models::patch::UpdateContextPayload;
use crate::services::context_manager::ContextManager;

/// Updates a key-value pair in the operational context store.
///
/// Delegates to `ContextManager::set` which performs an upsert on the
/// `operational_context` table. The `run_id` parameter is optional and
/// will be set to `None` since the patch op does not carry a run_id
/// (the run_id is tracked at the PatchSet level, not per-op).
pub fn execute_update_context(
    conn: &rusqlite::Connection,
    payload: &UpdateContextPayload,
) -> crate::error::Result<()> {
    ContextManager::set(conn, &payload.key, &payload.value, None)
}

/// Extended version that accepts an optional run_id for provenance tracking.
pub fn execute_update_context_with_run_id(
    conn: &rusqlite::Connection,
    payload: &UpdateContextPayload,
    run_id: Option<&str>,
) -> crate::error::Result<()> {
    ContextManager::set(conn, &payload.key, &payload.value, run_id)
}

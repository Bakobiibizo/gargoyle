use rusqlite::params;
use crate::models::patch::CreateEntityPayload;

/// Creates a new entity in the database.
///
/// Generates a UUID for the entity ID and timestamps using chrono.
/// Inserts into the entities table and updates the FTS5 index.
/// Returns the generated entity ID.
pub fn execute_create_entity(
    conn: &rusqlite::Connection,
    payload: &CreateEntityPayload,
    run_id: Option<&str>,
) -> crate::error::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let body_md = payload.body_md.as_deref().unwrap_or("");
    let canonical_fields_str = serde_json::to_string(&payload.canonical_fields)?;
    let schema_version: i32 = 1;

    conn.execute(
        "INSERT INTO entities (id, entity_type, category, title, body_md, status, priority, created_at, updated_at, source, canonical_fields, _schema_version, provenance_run_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            payload.entity_type,
            payload.category,
            payload.title,
            body_md,
            payload.status,
            payload.priority,
            now,
            now,
            payload.source,
            canonical_fields_str,
            schema_version,
            run_id,
        ],
    )?;

    // Update FTS5 content-sync index
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
        params![id],
    )?;

    Ok(id)
}

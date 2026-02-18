use rusqlite::params;

use crate::error::GargoyleError;
use crate::models::patch::AttachArtifactPayload;

/// Attaches an artifact (file, link, etc.) to an existing entity.
///
/// Validates that the referenced entity exists and is not deleted, then inserts
/// a new row into the `artifacts` table. Returns the generated artifact ID.
pub fn execute_attach_artifact(
    conn: &rusqlite::Connection,
    payload: &AttachArtifactPayload,
) -> crate::error::Result<String> {
    // Validate that the target entity exists and is not soft-deleted.
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            params![payload.entity_id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !exists {
        return Err(GargoyleError::NotFound {
            entity_type: "entity".to_string(),
            id: payload.entity_id.clone(),
        });
    }

    let artifact_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    conn.execute(
        "INSERT INTO artifacts (artifact_id, entity_id, kind, uri_or_path, hash, mime, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            artifact_id,
            payload.entity_id,
            payload.kind,
            payload.uri_or_path,
            payload.hash,
            payload.mime,
            now,
        ],
    )?;

    Ok(artifact_id)
}

use rusqlite::params;
use crate::models::patch::CreateRelationPayload;

/// Creates a new relation between two entities in the database.
///
/// Generates a UUID for the relation ID and a timestamp using chrono.
/// Inserts into the relations table with all fields.
/// Returns the generated relation ID.
pub fn execute_create_relation(
    conn: &rusqlite::Connection,
    payload: &CreateRelationPayload,
) -> crate::error::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let weight = payload.weight.unwrap_or(1.0);

    conn.execute(
        "INSERT INTO relations (id, from_id, to_id, relation_type, weight, confidence, provenance_run_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            payload.from_id,
            payload.to_id,
            payload.relation_type,
            weight,
            payload.confidence,
            payload.provenance_run_id,
            now,
        ],
    )?;

    Ok(id)
}

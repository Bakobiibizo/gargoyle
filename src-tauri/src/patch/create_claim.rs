use rusqlite::params;
use crate::models::patch::CreateClaimPayload;

/// Creates a new claim in the database.
///
/// Generates a UUID for the claim ID and a timestamp using chrono.
/// Inserts into the claims table with all fields.
/// Returns the generated claim ID.
pub fn execute_create_claim(
    conn: &rusqlite::Connection,
    payload: &CreateClaimPayload,
) -> crate::error::Result<String> {
    let claim_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    conn.execute(
        "INSERT INTO claims (claim_id, subject, predicate, object, confidence, evidence_entity_id, provenance_run_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            claim_id,
            payload.subject,
            payload.predicate,
            payload.object,
            payload.confidence,
            payload.evidence_entity_id,
            payload.provenance_run_id,
            now,
        ],
    )?;

    Ok(claim_id)
}

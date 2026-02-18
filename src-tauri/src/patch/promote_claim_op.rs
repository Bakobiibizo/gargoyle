use rusqlite::params;

use crate::error::GargoyleError;
use crate::models::patch::PromoteClaimPayload;
use crate::services::claim_service::ClaimService;

/// Promotes a claim to a first-class entity (spec §2.6.1).
///
/// Steps:
///   1. Read the claim by claim_id (fail if not found).
///   2. Verify the claim hasn't already been promoted.
///   3. Create a new entity of type `concept` (or target_entity_type if specified)
///      with title = "{subject} {predicate} {object}" and body = object.
///   4. Set `promoted_to_entity_id` on the claim record.
///   5. Create an `evidence_for` relation from the new entity to the claim's evidence_entity_id.
///   6. Return the new entity ID.
pub fn execute_promote_claim(
    conn: &rusqlite::Connection,
    payload: &PromoteClaimPayload,
) -> crate::error::Result<String> {
    // 1. Read the claim
    let claim = ClaimService::get_claim(conn, &payload.claim_id)?;

    // 2. Verify not already promoted
    if claim.promoted_to_entity_id.is_some() {
        return Err(GargoyleError::Schema(format!(
            "Claim {} has already been promoted",
            payload.claim_id
        )));
    }

    // 3. Create a new entity
    let entity_type = payload
        .target_entity_type
        .as_deref()
        .unwrap_or("concept");
    let entity_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    let title = format!("{} {} {}", claim.subject, claim.predicate, claim.object);
    let body_md = &claim.object;

    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'agent', '{}', 1, ?5, ?5)",
        params![entity_id, entity_type, title, body_md, now],
    )?;

    // Update FTS5 content-sync index
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
        params![entity_id],
    )?;

    // 4. Set promoted_to_entity_id on the claim
    conn.execute(
        "UPDATE claims SET promoted_to_entity_id = ?1 WHERE claim_id = ?2",
        params![entity_id, payload.claim_id],
    )?;

    // 5. Create an evidence_for relation from the new entity to the evidence entity
    let relation_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO relations (id, from_id, to_id, relation_type, weight, created_at)
         VALUES (?1, ?2, ?3, 'evidence_for', 1.0, ?4)",
        params![relation_id, entity_id, claim.evidence_entity_id, now],
    )?;

    // 6. Return the new entity ID
    Ok(entity_id)
}

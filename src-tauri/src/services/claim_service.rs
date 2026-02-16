// ClaimService: grounding enforcement + primary entity resolution

use rusqlite::{params, Connection};

use crate::error::{ErrorCode, GargoyleError, Result, ValidationError};
use crate::models::claim::Claim;
use crate::models::patch::{CreateEntityPayload, PatchOp, PatchSet};
use crate::patch::apply::apply_patch_set;

pub struct ClaimService;

impl ClaimService {
    /// Retrieve a single claim by ID. Returns NotFound if missing.
    pub fn get_claim(conn: &Connection, claim_id: &str) -> Result<Claim> {
        conn.query_row(
            "SELECT claim_id, subject, predicate, object, confidence, evidence_entity_id, \
             provenance_run_id, promoted_to_entity_id, created_at \
             FROM claims WHERE claim_id = ?1",
            params![claim_id],
            claim_from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "claim".to_string(),
                id: claim_id.to_string(),
            },
            other => GargoyleError::Database(other),
        })
    }

    /// List claims, optionally filtered by evidence_entity_id. Ordered by created_at DESC.
    pub fn list_claims(
        conn: &Connection,
        evidence_entity_id: Option<&str>,
    ) -> Result<Vec<Claim>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match evidence_entity_id {
                Some(eid) => (
                    "SELECT claim_id, subject, predicate, object, confidence, evidence_entity_id, \
                     provenance_run_id, promoted_to_entity_id, created_at \
                     FROM claims WHERE evidence_entity_id = ?1 ORDER BY created_at DESC"
                        .to_string(),
                    vec![Box::new(eid.to_string())],
                ),
                None => (
                    "SELECT claim_id, subject, predicate, object, confidence, evidence_entity_id, \
                     provenance_run_id, promoted_to_entity_id, created_at \
                     FROM claims ORDER BY created_at DESC"
                        .to_string(),
                    vec![],
                ),
            };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), claim_from_row)?;

        let mut claims = Vec::new();
        for row_result in rows {
            claims.push(row_result?);
        }
        Ok(claims)
    }

    /// Verify that the evidence entity exists and is not soft-deleted.
    /// Returns GargoyleError::Validation with ErrorCode::UngroundedClaim if
    /// entity doesn't exist or is deleted.
    pub fn validate_grounding(conn: &Connection, evidence_entity_id: &str) -> Result<()> {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                params![evidence_entity_id],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count > 0)
                },
            )
            .map_err(GargoyleError::Database)?;

        if !exists {
            return Err(GargoyleError::Validation(ValidationError {
                code: ErrorCode::UngroundedClaim,
                field_path: "evidence_entity_id".to_string(),
                message: format!(
                    "Evidence entity {} does not exist or is deleted",
                    evidence_entity_id
                ),
                expected: None,
                actual: Some(evidence_entity_id.to_string()),
            }));
        }

        Ok(())
    }

    /// "Promote" a claim to a full entity.
    ///
    /// Steps:
    ///   a. Read the claim by ID (return NotFound if missing)
    ///   b. Verify the claim hasn't already been promoted (promoted_to_entity_id IS NULL).
    ///      If already promoted, return a Schema error.
    ///   c. Create a new entity using the patch system.
    ///   d. Update the claim's promoted_to_entity_id to the new entity ID.
    ///   e. Return the new entity ID.
    pub fn promote_claim(
        conn: &Connection,
        claim_id: &str,
        entity_type: &str,
        source: &str,
    ) -> Result<String> {
        // a. Read the claim
        let claim = Self::get_claim(conn, claim_id)?;

        // b. Verify not already promoted
        if claim.promoted_to_entity_id.is_some() {
            return Err(GargoyleError::Schema(format!(
                "Claim {} has already been promoted",
                claim_id
            )));
        }

        // c. Create a new entity via the patch system
        let title = format!("{} {} {}", claim.subject, claim.predicate, claim.object);
        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: entity_type.to_string(),
                title,
                source: source.to_string(),
                canonical_fields: serde_json::json!({}),
                body_md: None,
                status: None,
                category: None,
                priority: None,
            })],
            run_id: None,
        };
        let result = apply_patch_set(conn, &patch_set)?;
        let entity_id = result.applied[0].entity_id.clone().unwrap();

        // d. Update the claim's promoted_to_entity_id
        conn.execute(
            "UPDATE claims SET promoted_to_entity_id = ?1 WHERE claim_id = ?2",
            params![entity_id, claim_id],
        )?;

        // e. Return the new entity ID
        Ok(entity_id)
    }

    /// Get all claims where evidence_entity_id = entity_id. Ordered by created_at DESC.
    pub fn get_claims_for_entity(conn: &Connection, entity_id: &str) -> Result<Vec<Claim>> {
        let mut stmt = conn.prepare(
            "SELECT claim_id, subject, predicate, object, confidence, evidence_entity_id, \
             provenance_run_id, promoted_to_entity_id, created_at \
             FROM claims WHERE evidence_entity_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![entity_id], claim_from_row)?;

        let mut claims = Vec::new();
        for row_result in rows {
            claims.push(row_result?);
        }
        Ok(claims)
    }
}

/// Parse a Claim from a rusqlite Row.
///
/// Expected column order:
///   0: claim_id, 1: subject, 2: predicate, 3: object, 4: confidence,
///   5: evidence_entity_id, 6: provenance_run_id, 7: promoted_to_entity_id, 8: created_at
fn claim_from_row(row: &rusqlite::Row) -> rusqlite::Result<Claim> {
    Ok(Claim {
        claim_id: row.get(0)?,
        subject: row.get(1)?,
        predicate: row.get(2)?,
        object: row.get(3)?,
        confidence: row.get(4)?,
        evidence_entity_id: row.get(5)?,
        provenance_run_id: row.get(6)?,
        promoted_to_entity_id: row.get(7)?,
        created_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::create_memory_connection;
    use crate::db::migrations::run_migrations;

    /// Create a fresh in-memory database with all migrations applied.
    fn test_db() -> Connection {
        let conn = create_memory_connection().expect("Failed to create test DB");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    /// Helper: insert a test entity directly for setup purposes.
    fn insert_test_entity(
        conn: &Connection,
        id: &str,
        entity_type: &str,
        title: &str,
        source: &str,
        canonical_fields: &str,
    ) -> String {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at) \
             VALUES (?1, ?2, ?3, '', ?4, ?5, 1, ?6, ?6)",
            params![id, entity_type, title, source, canonical_fields, now],
        )
        .expect("Failed to insert test entity");
        // Also insert into FTS5 index so update/delete operations on FTS work correctly
        conn.execute(
            "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
            params![id],
        )
        .expect("Failed to insert test entity into FTS");
        now
    }

    /// Helper: soft-delete an entity.
    fn soft_delete_entity(conn: &Connection, id: &str) {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .expect("Failed to soft-delete entity");
    }

    /// Helper: insert a test claim directly for setup purposes.
    fn insert_test_claim(
        conn: &Connection,
        claim_id: &str,
        subject: &str,
        predicate: &str,
        object: &str,
        evidence_id: &str,
    ) {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO claims (claim_id, subject, predicate, object, confidence, evidence_entity_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, 0.9, ?5, ?6)",
            params![claim_id, subject, predicate, object, evidence_id, now],
        )
        .unwrap();
    }

    // ========================================================================
    // get_claim
    // ========================================================================

    #[test]
    fn test_get_claim_found() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence", "manual", "{}");
        insert_test_claim(&conn, "claim-1", "Revenue", "increased_by", "10%", "ev-1");

        let claim = ClaimService::get_claim(&conn, "claim-1").unwrap();
        assert_eq!(claim.claim_id, "claim-1");
        assert_eq!(claim.subject, "Revenue");
        assert_eq!(claim.predicate, "increased_by");
        assert_eq!(claim.object, "10%");
        assert!((claim.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(claim.evidence_entity_id, "ev-1");
        assert!(claim.provenance_run_id.is_none());
        assert!(claim.promoted_to_entity_id.is_none());
    }

    #[test]
    fn test_get_claim_not_found() {
        let conn = test_db();
        let result = ClaimService::get_claim(&conn, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "claim");
                assert_eq!(id, "nonexistent");
            }
            other => panic!("Expected NotFound, got: {:?}", other),
        }
    }

    // ========================================================================
    // list_claims
    // ========================================================================

    #[test]
    fn test_list_claims_all() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence 1", "manual", "{}");
        insert_test_entity(&conn, "ev-2", "result", "Evidence 2", "manual", "{}");
        insert_test_claim(&conn, "c-1", "A", "is", "B", "ev-1");
        insert_test_claim(&conn, "c-2", "C", "is", "D", "ev-2");

        let claims = ClaimService::list_claims(&conn, None).unwrap();
        assert_eq!(claims.len(), 2);
    }

    #[test]
    fn test_list_claims_by_evidence() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence 1", "manual", "{}");
        insert_test_entity(&conn, "ev-2", "result", "Evidence 2", "manual", "{}");
        insert_test_claim(&conn, "c-1", "A", "is", "B", "ev-1");
        insert_test_claim(&conn, "c-2", "C", "is", "D", "ev-2");
        insert_test_claim(&conn, "c-3", "E", "is", "F", "ev-1");

        let claims = ClaimService::list_claims(&conn, Some("ev-1")).unwrap();
        assert_eq!(claims.len(), 2);
        assert!(claims.iter().all(|c| c.evidence_entity_id == "ev-1"));

        let claims2 = ClaimService::list_claims(&conn, Some("ev-2")).unwrap();
        assert_eq!(claims2.len(), 1);
        assert_eq!(claims2[0].evidence_entity_id, "ev-2");
    }

    #[test]
    fn test_list_claims_empty() {
        let conn = test_db();
        let claims = ClaimService::list_claims(&conn, None).unwrap();
        assert!(claims.is_empty());
    }

    // ========================================================================
    // validate_grounding
    // ========================================================================

    #[test]
    fn test_validate_grounding_valid() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence", "manual", "{}");

        let result = ClaimService::validate_grounding(&conn, "ev-1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_grounding_not_found() {
        let conn = test_db();

        let result = ClaimService::validate_grounding(&conn, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::Validation(v) => {
                assert!(matches!(v.code, ErrorCode::UngroundedClaim));
                assert_eq!(v.field_path, "evidence_entity_id");
                assert_eq!(v.actual, Some("nonexistent".to_string()));
            }
            other => panic!("Expected Validation(UngroundedClaim), got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_grounding_deleted() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-del", "result", "Deleted Evidence", "manual", "{}");
        soft_delete_entity(&conn, "ev-del");

        let result = ClaimService::validate_grounding(&conn, "ev-del");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::Validation(v) => {
                assert!(matches!(v.code, ErrorCode::UngroundedClaim));
                assert_eq!(v.field_path, "evidence_entity_id");
                assert_eq!(v.actual, Some("ev-del".to_string()));
            }
            other => panic!("Expected Validation(UngroundedClaim), got: {:?}", other),
        }
    }

    // ========================================================================
    // promote_claim
    // ========================================================================

    #[test]
    fn test_promote_claim_success() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence", "manual", "{}");
        insert_test_claim(&conn, "claim-p", "Revenue", "grew_by", "20%", "ev-1");

        let entity_id =
            ClaimService::promote_claim(&conn, "claim-p", "metric", "agent").unwrap();

        // Verify the new entity was created
        assert!(!entity_id.is_empty());

        // Verify the claim was updated with promoted_to_entity_id
        let claim = ClaimService::get_claim(&conn, "claim-p").unwrap();
        assert_eq!(claim.promoted_to_entity_id, Some(entity_id.clone()));

        // Verify the entity exists in the database with correct title
        let entity_title: String = conn
            .query_row(
                "SELECT title FROM entities WHERE id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(entity_title, "Revenue grew_by 20%");

        // Verify the entity type is correct
        let entity_type: String = conn
            .query_row(
                "SELECT entity_type FROM entities WHERE id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(entity_type, "metric");

        // Verify the source is correct
        let entity_source: String = conn
            .query_row(
                "SELECT source FROM entities WHERE id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(entity_source, "agent");
    }

    #[test]
    fn test_promote_claim_not_found() {
        let conn = test_db();

        let result = ClaimService::promote_claim(&conn, "nonexistent", "metric", "agent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "claim");
                assert_eq!(id, "nonexistent");
            }
            other => panic!("Expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_promote_claim_already_promoted() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence", "manual", "{}");
        insert_test_claim(&conn, "claim-ap", "Revenue", "grew_by", "20%", "ev-1");

        // Promote once
        ClaimService::promote_claim(&conn, "claim-ap", "metric", "agent").unwrap();

        // Attempt to promote again
        let result = ClaimService::promote_claim(&conn, "claim-ap", "metric", "agent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::Schema(msg) => {
                assert!(
                    msg.contains("already been promoted"),
                    "Expected 'already been promoted' in message, got: {}",
                    msg
                );
            }
            other => panic!("Expected Schema error, got: {:?}", other),
        }
    }

    // ========================================================================
    // get_claims_for_entity
    // ========================================================================

    #[test]
    fn test_get_claims_for_entity() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-1", "result", "Evidence 1", "manual", "{}");
        insert_test_entity(&conn, "ev-2", "result", "Evidence 2", "manual", "{}");
        insert_test_claim(&conn, "c-1", "A", "is", "B", "ev-1");
        insert_test_claim(&conn, "c-2", "C", "is", "D", "ev-1");
        insert_test_claim(&conn, "c-3", "E", "is", "F", "ev-2");

        let claims = ClaimService::get_claims_for_entity(&conn, "ev-1").unwrap();
        assert_eq!(claims.len(), 2);
        assert!(claims.iter().all(|c| c.evidence_entity_id == "ev-1"));
    }

    #[test]
    fn test_get_claims_for_entity_empty() {
        let conn = test_db();
        insert_test_entity(&conn, "ev-lonely", "result", "Lonely", "manual", "{}");

        let claims = ClaimService::get_claims_for_entity(&conn, "ev-lonely").unwrap();
        assert!(claims.is_empty());
    }
}

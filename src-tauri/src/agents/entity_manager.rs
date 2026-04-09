use rusqlite::{params, Connection};
use tracing::{debug, instrument};

use crate::error::{GargoyleError, Result};
use crate::models::patch::{CreateEntityPayload, PatchOp, PatchSet, UpdateEntityPayload};
use crate::patch::apply::apply_patch_set;
use crate::schema::registry::SchemaRegistry;

use super::types::{EntityDetail, EntityManagerRequest, EntityManagerResponse};

pub struct EntityManagerAgent;

impl EntityManagerAgent {
    #[instrument(skip(conn), fields(action = ?std::mem::discriminant(&request)))]
    pub fn handle(
        conn: &Connection,
        request: EntityManagerRequest,
    ) -> Result<EntityManagerResponse> {
        debug!("Handling entity manager request");
        match request {
            EntityManagerRequest::Create {
                entity_type,
                title,
                body,
                canonical,
            } => {
                let patch_set = PatchSet {
                    run_id: uuid::Uuid::new_v4().to_string(),
                    ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                        entity_type,
                        title,
                        source: "agent".to_string(),
                        canonical_fields: canonical,
                        body_md: body,
                        status: None,
                        category: None,
                        priority: None,
                        reason: Some("Created via EntityManagerAgent".to_string()),
                    })],
                };

                let result = apply_patch_set(conn, &patch_set)?;
                let entity_id = result
                    .applied
                    .first()
                    .and_then(|a| a.entity_id.clone())
                    .ok_or_else(|| GargoyleError::Schema("Failed to create entity".to_string()))?;

                Ok(EntityManagerResponse::Created { entity_id })
            }

            EntityManagerRequest::Update {
                entity_id,
                title,
                body,
                status,
                canonical,
            } => {
                let updated_at: String = conn.query_row(
                    "SELECT updated_at FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                    params![entity_id],
                    |row| row.get(0),
                )?;

                let patch_set = PatchSet {
                    run_id: uuid::Uuid::new_v4().to_string(),
                    ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                        entity_id,
                        expected_updated_at: updated_at,
                        title,
                        body_md: body,
                        status,
                        canonical_fields: canonical,
                        category: None,
                        priority: None,
                        reason: Some("Updated via EntityManagerAgent".to_string()),
                    })],
                };

                apply_patch_set(conn, &patch_set)?;
                Ok(EntityManagerResponse::Updated)
            }

            EntityManagerRequest::Delete { entity_id } => {
                let now = chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                    .to_string();
                conn.execute(
                    "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
                    params![now, entity_id],
                )?;
                Ok(EntityManagerResponse::Deleted)
            }

            EntityManagerRequest::ChangeStatus {
                entity_id,
                new_status,
            } => {
                let updated_at: String = conn.query_row(
                    "SELECT updated_at FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                    params![entity_id],
                    |row| row.get(0),
                )?;

                let patch_set = PatchSet {
                    run_id: uuid::Uuid::new_v4().to_string(),
                    ops: vec![PatchOp::UpdateEntity(UpdateEntityPayload {
                        entity_id,
                        expected_updated_at: updated_at,
                        title: None,
                        body_md: None,
                        status: Some(new_status),
                        canonical_fields: None,
                        category: None,
                        priority: None,
                        reason: Some("Status changed via EntityManagerAgent".to_string()),
                    })],
                };

                apply_patch_set(conn, &patch_set)?;
                Ok(EntityManagerResponse::StatusChanged)
            }

            EntityManagerRequest::Get { entity_id } => {
                let entity = conn.query_row(
                    "SELECT id, entity_type, title, body, status, canonical_json, created_at, updated_at 
                     FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                    params![entity_id],
                    |row| {
                        let canonical_str: String = row.get(5)?;
                        let canonical: serde_json::Value = serde_json::from_str(&canonical_str)
                            .unwrap_or(serde_json::json!({}));
                        Ok(EntityDetail {
                            id: row.get(0)?,
                            entity_type: row.get(1)?,
                            title: row.get(2)?,
                            body: row.get(3)?,
                            status: row.get(4)?,
                            canonical,
                            created_at: row.get(6)?,
                            updated_at: row.get(7)?,
                        })
                    },
                )?;

                Ok(EntityManagerResponse::Entity { entity })
            }

            EntityManagerRequest::ValidateCanonical {
                entity_type,
                canonical,
            } => {
                let registry = SchemaRegistry::global();
                let errors = registry.validate_canonical_fields(&entity_type, 1, &canonical);

                if errors.is_empty() {
                    Ok(EntityManagerResponse::ValidationResult {
                        valid: true,
                        errors: vec![],
                    })
                } else {
                    Ok(EntityManagerResponse::ValidationResult {
                        valid: false,
                        errors: errors.into_iter().map(|e| e.message).collect(),
                    })
                }
            }
        }
    }
}

use rusqlite::params;

use crate::error::GargoyleError;
use crate::models::patch::DeleteRelationPayload;

/// Deletes an existing relation by its ID.
///
/// Performs a hard DELETE because the `relations` table does not currently have
/// a `deleted_at` column. If soft-delete is needed in the future, add a
/// `deleted_at TEXT` column to the relations table and change this to an UPDATE.
pub fn execute_delete_relation(
    conn: &rusqlite::Connection,
    payload: &DeleteRelationPayload,
) -> crate::error::Result<String> {
    let rows_affected = conn.execute(
        "DELETE FROM relations WHERE id = ?1",
        params![payload.relation_id],
    )?;

    if rows_affected == 0 {
        return Err(GargoyleError::NotFound {
            entity_type: "relation".to_string(),
            id: payload.relation_id.clone(),
        });
    }

    Ok(payload.relation_id.clone())
}

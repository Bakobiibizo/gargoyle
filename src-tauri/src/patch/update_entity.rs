use rusqlite::params;
use crate::error::GargoyleError;
use crate::models::patch::UpdateEntityPayload;

/// Updates an existing entity with optimistic locking.
///
/// Reads the current entity to verify the expected_updated_at matches.
/// If there is a mismatch, returns GargoyleError::LockConflict.
/// Builds a dynamic UPDATE SET clause for only the fields that are Some.
/// Updates the FTS5 index after the update.
/// Returns the new updated_at timestamp.
pub fn execute_update_entity(
    conn: &rusqlite::Connection,
    payload: &UpdateEntityPayload,
    current_schema_version: i32,
) -> crate::error::Result<String> {
    // Read the current entity's updated_at (for optimistic lock check) and
    // title/body_md (needed for FTS5 external-content delete command).
    let (actual_updated_at, old_title, old_body_md, old_rowid): (String, String, String, i64) = conn
        .query_row(
            "SELECT updated_at, title, body_md, rowid FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            params![payload.entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: payload.entity_id.clone(),
            },
            other => GargoyleError::Database(other),
        })?;

    // Optimistic lock check: compare expected vs actual
    if payload.expected_updated_at != actual_updated_at {
        return Err(GargoyleError::LockConflict {
            expected: payload.expected_updated_at.clone(),
            found: actual_updated_at,
        });
    }

    let new_updated_at = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    // Build dynamic UPDATE statement
    // We always set updated_at and _schema_version
    let mut set_clauses: Vec<String> = vec![
        "updated_at = ?".to_string(),
        "_schema_version = ?".to_string(),
    ];
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(new_updated_at.clone()),
        Box::new(current_schema_version),
    ];

    if let Some(ref title) = payload.title {
        set_clauses.push("title = ?".to_string());
        param_values.push(Box::new(title.clone()));
    }

    if let Some(ref body_md) = payload.body_md {
        set_clauses.push("body_md = ?".to_string());
        param_values.push(Box::new(body_md.clone()));
    }

    if let Some(ref status) = payload.status {
        set_clauses.push("status = ?".to_string());
        param_values.push(Box::new(status.clone()));
    }

    if let Some(ref canonical_fields) = payload.canonical_fields {
        let cf_str = serde_json::to_string(canonical_fields)?;
        set_clauses.push("canonical_fields = ?".to_string());
        param_values.push(Box::new(cf_str));
    }

    if let Some(ref category) = payload.category {
        set_clauses.push("category = ?".to_string());
        param_values.push(Box::new(category.clone()));
    }

    if let Some(priority) = payload.priority {
        set_clauses.push("priority = ?".to_string());
        param_values.push(Box::new(priority));
    }

    // Add the WHERE clause parameter (entity_id)
    param_values.push(Box::new(payload.entity_id.clone()));

    let sql = format!(
        "UPDATE entities SET {} WHERE id = ?",
        set_clauses.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    conn.execute(&sql, params_refs.as_slice())?;

    // Update FTS5 external content index.
    // For external-content FTS5 tables (content=entities), we must use the
    // special 'delete' command with the OLD values, then re-insert with new values.
    // A plain DELETE FROM entities_fts WHERE rowid=... does NOT work for
    // external-content tables and causes SQLITE_CORRUPT_VTAB.
    conn.execute(
        "INSERT INTO entities_fts(entities_fts, rowid, title, body_md) VALUES('delete', ?1, ?2, ?3)",
        params![old_rowid, old_title, old_body_md],
    )?;

    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
        params![payload.entity_id],
    )?;

    Ok(new_updated_at)
}

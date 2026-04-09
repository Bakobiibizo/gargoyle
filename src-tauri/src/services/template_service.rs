use rusqlite::{params, Connection};

use crate::error::{GargoyleError, Result};
use crate::models::template::{
    CreateTemplatePayload, Template, TemplateIndex, UpdateTemplatePayload,
};

pub struct TemplateService;

impl TemplateService {
    pub fn create(conn: &Connection, payload: CreateTemplatePayload) -> Result<Template> {
        let id = uuid::Uuid::new_v4().to_string();
        let produces_entities =
            serde_json::to_string(&payload.produces_entities.unwrap_or_default())
                .unwrap_or_else(|_| "[]".to_string());
        let produces_relations =
            serde_json::to_string(&payload.produces_relations.unwrap_or_default())
                .unwrap_or_else(|_| "[]".to_string());
        let generator_config =
            serde_json::to_string(&payload.generator_config.unwrap_or(serde_json::json!({})))
                .unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            r#"
            INSERT INTO templates (
                id, key, category, description, content, response_format,
                produces_entities, produces_relations, generator_type, 
                generator_config, created_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                id,
                payload.key,
                payload.category,
                payload.description,
                payload.content,
                payload.response_format,
                produces_entities,
                produces_relations,
                payload.generator_type,
                generator_config,
                payload.created_by,
            ],
        )
        .map_err(GargoyleError::Database)?;

        Self::get_by_key(conn, &payload.key)
    }

    pub fn get_by_key(conn: &Connection, key: &str) -> Result<Template> {
        conn.query_row(
            r#"
            SELECT id, key, version, category, description, content, response_format,
                   produces_entities, produces_relations, generator_type, generator_config,
                   created_at, updated_at, created_by, usage_count, last_used_at
            FROM templates
            WHERE key = ?1 AND deleted_at IS NULL
            "#,
            params![key],
            |row| {
                let produces_entities_str: String = row.get(7)?;
                let produces_relations_str: String = row.get(8)?;
                let generator_config_str: String = row.get(10)?;

                Ok(Template {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    version: row.get(2)?,
                    category: row.get(3)?,
                    description: row.get(4)?,
                    content: row.get(5)?,
                    response_format: row.get(6)?,
                    produces_entities: serde_json::from_str(&produces_entities_str)
                        .unwrap_or_default(),
                    produces_relations: serde_json::from_str(&produces_relations_str)
                        .unwrap_or_default(),
                    generator_type: row.get(9)?,
                    generator_config: serde_json::from_str(&generator_config_str)
                        .unwrap_or(serde_json::json!({})),
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    created_by: row.get(13)?,
                    usage_count: row.get(14)?,
                    last_used_at: row.get(15)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "template".to_string(),
                id: key.to_string(),
            },
            other => GargoyleError::Database(other),
        })
    }

    pub fn update(conn: &Connection, payload: UpdateTemplatePayload) -> Result<Template> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref category) = payload.category {
            updates.push("category = ?");
            params_vec.push(Box::new(category.clone()));
        }
        if let Some(ref description) = payload.description {
            updates.push("description = ?");
            params_vec.push(Box::new(description.clone()));
        }
        if let Some(ref content) = payload.content {
            updates.push("content = ?");
            params_vec.push(Box::new(content.clone()));
        }
        if let Some(ref response_format) = payload.response_format {
            updates.push("response_format = ?");
            params_vec.push(Box::new(response_format.clone()));
        }
        if let Some(ref produces_entities) = payload.produces_entities {
            updates.push("produces_entities = ?");
            params_vec.push(Box::new(
                serde_json::to_string(produces_entities).unwrap_or_default(),
            ));
        }
        if let Some(ref produces_relations) = payload.produces_relations {
            updates.push("produces_relations = ?");
            params_vec.push(Box::new(
                serde_json::to_string(produces_relations).unwrap_or_default(),
            ));
        }
        if let Some(ref generator_type) = payload.generator_type {
            updates.push("generator_type = ?");
            params_vec.push(Box::new(generator_type.clone()));
        }
        if let Some(ref generator_config) = payload.generator_config {
            updates.push("generator_config = ?");
            params_vec.push(Box::new(
                serde_json::to_string(generator_config).unwrap_or_default(),
            ));
        }

        if updates.is_empty() {
            return Self::get_by_key(conn, &payload.key);
        }

        updates.push("updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')");
        params_vec.push(Box::new(payload.key.clone()));

        let sql = format!(
            "UPDATE templates SET {} WHERE key = ? AND deleted_at IS NULL",
            updates.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())
            .map_err(GargoyleError::Database)?;

        Self::get_by_key(conn, &payload.key)
    }

    pub fn delete(conn: &Connection, key: &str) -> Result<()> {
        let affected = conn.execute(
            "UPDATE templates SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE key = ?1 AND deleted_at IS NULL",
            params![key],
        ).map_err(GargoyleError::Database)?;

        if affected == 0 {
            return Err(GargoyleError::NotFound {
                entity_type: "template".to_string(),
                id: key.to_string(),
            });
        }
        Ok(())
    }

    pub fn list(conn: &Connection, category: Option<&str>) -> Result<Vec<TemplateIndex>> {
        let sql = match category {
            Some(_) => {
                r#"
                SELECT key, category, description, produces_entities, usage_count
                FROM templates
                WHERE deleted_at IS NULL AND category = ?1
                ORDER BY usage_count DESC, key ASC
            "#
            }
            None => {
                r#"
                SELECT key, category, description, produces_entities, usage_count
                FROM templates
                WHERE deleted_at IS NULL
                ORDER BY usage_count DESC, key ASC
            "#
            }
        };

        let mut stmt = conn.prepare(sql).map_err(GargoyleError::Database)?;

        let rows = if let Some(cat) = category {
            stmt.query_map(params![cat], Self::map_index_row)
        } else {
            stmt.query_map([], Self::map_index_row)
        }
        .map_err(GargoyleError::Database)?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row.map_err(GargoyleError::Database)?);
        }
        Ok(templates)
    }

    pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<TemplateIndex>> {
        let mut stmt = conn
            .prepare(
                r#"
            SELECT t.key, t.category, t.description, t.produces_entities, t.usage_count
            FROM templates t
            JOIN templates_fts fts ON t.rowid = fts.rowid
            WHERE templates_fts MATCH ?1 AND t.deleted_at IS NULL
            ORDER BY rank, t.usage_count DESC
            LIMIT ?2
        "#,
            )
            .map_err(GargoyleError::Database)?;

        let rows = stmt
            .query_map(params![query, limit as i64], Self::map_index_row)
            .map_err(GargoyleError::Database)?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row.map_err(GargoyleError::Database)?);
        }
        Ok(templates)
    }

    pub fn record_usage(conn: &Connection, key: &str) -> Result<()> {
        conn.execute(
            r#"
            UPDATE templates 
            SET usage_count = usage_count + 1,
                last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE key = ?1 AND deleted_at IS NULL
            "#,
            params![key],
        )
        .map_err(GargoyleError::Database)?;
        Ok(())
    }

    fn map_index_row(row: &rusqlite::Row) -> rusqlite::Result<TemplateIndex> {
        let produces_entities_str: String = row.get(3)?;
        Ok(TemplateIndex {
            key: row.get(0)?,
            category: row.get(1)?,
            description: row.get(2)?,
            produces_entities: serde_json::from_str(&produces_entities_str).unwrap_or_default(),
            usage_count: row.get(4)?,
        })
    }
}

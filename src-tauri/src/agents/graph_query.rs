use rusqlite::{params, Connection};
use tracing::{debug, instrument};

use crate::error::Result;

use super::types::{EntitySummary, GraphQueryRequest, GraphQueryResponse, GraphStats};

pub struct GraphQueryAgent;

impl GraphQueryAgent {
    #[instrument(skip(conn), fields(action = ?std::mem::discriminant(&request)))]
    pub fn handle(conn: &Connection, request: GraphQueryRequest) -> Result<GraphQueryResponse> {
        debug!("Handling graph query request");
        match request {
            GraphQueryRequest::GetNeighbors { entity_id, depth } => {
                let max_depth = depth.unwrap_or(1);
                let entities = Self::get_neighbors(conn, &entity_id, max_depth)?;
                Ok(GraphQueryResponse::Entities { entities })
            }

            GraphQueryRequest::FindPath { from_id, to_id } => {
                let path = Self::find_path(conn, &from_id, &to_id)?;
                Ok(GraphQueryResponse::Path { path })
            }

            GraphQueryRequest::SearchEntities {
                query,
                entity_type,
                limit,
            } => {
                let entities = Self::search_entities(
                    conn,
                    &query,
                    entity_type.as_deref(),
                    limit.unwrap_or(10),
                )?;
                Ok(GraphQueryResponse::Entities { entities })
            }

            GraphQueryRequest::SimilarEntities { entity_id, limit } => {
                let entities = Self::get_similar(conn, &entity_id, limit.unwrap_or(5))?;
                Ok(GraphQueryResponse::Entities { entities })
            }

            GraphQueryRequest::GetStatistics { entity_type } => {
                let stats = Self::get_statistics(conn, entity_type.as_deref())?;
                Ok(GraphQueryResponse::Statistics { stats })
            }

            GraphQueryRequest::GetEntityContext {
                entity_id,
                max_tokens,
            } => {
                let context = Self::get_entity_context(conn, &entity_id, max_tokens)?;
                Ok(GraphQueryResponse::Context { context })
            }

            GraphQueryRequest::GetRelevantEntities { query, max_tokens } => {
                let context = Self::get_relevant_context(conn, &query, max_tokens)?;
                Ok(GraphQueryResponse::Context { context })
            }
        }
    }

    fn get_neighbors(
        conn: &Connection,
        entity_id: &str,
        depth: usize,
    ) -> Result<Vec<EntitySummary>> {
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut to_visit = vec![entity_id.to_string()];
        let mut results = Vec::new();

        for _ in 0..depth {
            let mut next_level = Vec::new();

            for id in &to_visit {
                if visited.contains(id) {
                    continue;
                }
                visited.insert(id.clone());

                let mut stmt = conn.prepare(
                    "SELECT e.id, e.entity_type, e.title, e.status, e.created_at
                     FROM entities e
                     JOIN relations r ON (r.from_id = e.id OR r.to_id = e.id)
                     WHERE (r.from_id = ?1 OR r.to_id = ?1)
                     AND e.id != ?1
                     AND e.deleted_at IS NULL",
                )?;

                let neighbors = stmt.query_map(params![id], |row| {
                    Ok(EntitySummary {
                        id: row.get(0)?,
                        entity_type: row.get(1)?,
                        title: row.get(2)?,
                        status: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                })?;

                for neighbor in neighbors {
                    let n = neighbor?;
                    if !visited.contains(&n.id) {
                        next_level.push(n.id.clone());
                        results.push(n);
                    }
                }
            }

            to_visit = next_level;
        }

        Ok(results)
    }

    fn find_path(conn: &Connection, from_id: &str, to_id: &str) -> Result<Vec<String>> {
        // Simple BFS path finding
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<(String, Vec<String>)> =
            std::collections::VecDeque::new();

        queue.push_back((from_id.to_string(), vec![from_id.to_string()]));
        visited.insert(from_id.to_string());

        while let Some((current, path)) = queue.pop_front() {
            if current == to_id {
                return Ok(path);
            }

            let mut stmt = conn.prepare(
                "SELECT CASE WHEN from_id = ?1 THEN to_id ELSE from_id END as neighbor
                 FROM relations
                 WHERE (from_id = ?1 OR to_id = ?1)",
            )?;

            let neighbors: Vec<String> = stmt
                .query_map(params![current], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    let mut new_path = path.clone();
                    new_path.push(neighbor.clone());
                    queue.push_back((neighbor, new_path));
                }
            }
        }

        Ok(vec![]) // No path found
    }

    fn search_entities(
        conn: &Connection,
        query: &str,
        entity_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntitySummary>> {
        let mut results = Vec::new();

        if let Some(et) = entity_type {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.entity_type, e.title, e.status, e.created_at
                 FROM entities e
                 JOIN entities_fts fts ON e.rowid = fts.rowid
                 WHERE entities_fts MATCH ?1
                 AND e.entity_type = ?2
                 AND e.deleted_at IS NULL
                 LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![query, et, limit], |row| {
                Ok(EntitySummary {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?;
            for row in rows {
                results.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.entity_type, e.title, e.status, e.created_at
                 FROM entities e
                 JOIN entities_fts fts ON e.rowid = fts.rowid
                 WHERE entities_fts MATCH ?1
                 AND e.deleted_at IS NULL
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![query, limit], |row| {
                Ok(EntitySummary {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?;
            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    }

    fn get_similar(
        _conn: &Connection,
        _entity_id: &str,
        _limit: usize,
    ) -> Result<Vec<EntitySummary>> {
        // TODO: Implement embedding-based similarity when embeddings are available
        Ok(vec![])
    }

    fn get_statistics(conn: &Connection, entity_type: Option<&str>) -> Result<GraphStats> {
        let total_entities: usize = if let Some(et) = entity_type {
            conn.query_row(
                "SELECT COUNT(*) FROM entities WHERE entity_type = ?1 AND deleted_at IS NULL",
                params![et],
                |row| row.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM entities WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )?
        };

        let total_relations: usize =
            conn.query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))?;

        let mut entities_by_type = std::collections::HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT entity_type, COUNT(*) FROM entities WHERE deleted_at IS NULL GROUP BY entity_type"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        })?;
        for row in rows {
            let (t, c) = row?;
            entities_by_type.insert(t, c);
        }

        let mut relations_by_type = std::collections::HashMap::new();
        let mut stmt =
            conn.prepare("SELECT relation_type, COUNT(*) FROM relations GROUP BY relation_type")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        })?;
        for row in rows {
            let (t, c) = row?;
            relations_by_type.insert(t, c);
        }

        Ok(GraphStats {
            total_entities,
            total_relations,
            entities_by_type,
            relations_by_type,
        })
    }

    fn get_entity_context(conn: &Connection, entity_id: &str, max_tokens: usize) -> Result<String> {
        let entity: Option<(String, String, Option<String>, Option<String>)> = conn.query_row(
            "SELECT entity_type, title, body, status FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).ok();

        let Some((entity_type, title, body, status)) = entity else {
            return Ok("Entity not found.".to_string());
        };

        let mut context = format!("## {} ({})\n**Title**: {}\n", entity_id, entity_type, title);

        if let Some(s) = status {
            context.push_str(&format!("**Status**: {}\n", s));
        }

        if let Some(b) = body {
            let truncated = if b.len() > max_tokens * 4 {
                format!("{}...", &b[..max_tokens * 4])
            } else {
                b
            };
            context.push_str(&format!("\n{}\n", truncated));
        }

        // Add related entities
        let neighbors = Self::get_neighbors(conn, entity_id, 1)?;
        if !neighbors.is_empty() {
            context.push_str("\n**Related**:\n");
            for n in neighbors.iter().take(5) {
                context.push_str(&format!("- {} ({}) - {}\n", n.id, n.entity_type, n.title));
            }
        }

        Ok(context)
    }

    fn get_relevant_context(conn: &Connection, query: &str, max_tokens: usize) -> Result<String> {
        let entities = Self::search_entities(conn, query, None, 5)?;

        if entities.is_empty() {
            return Ok("No relevant entities found.".to_string());
        }

        let mut context = format!("## Relevant Entities ({} found)\n\n", entities.len());
        let mut tokens_used = context.len() / 4;

        for (i, entity) in entities.iter().enumerate() {
            let entry = format!(
                "{}. **{}** ({}) - {}\n",
                i + 1,
                entity.title,
                entity.entity_type,
                entity.status.as_deref().unwrap_or("no status")
            );

            if tokens_used + entry.len() / 4 > max_tokens {
                break;
            }

            context.push_str(&entry);
            tokens_used += entry.len() / 4;
        }

        Ok(context)
    }
}

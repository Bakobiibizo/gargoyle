use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::Result;
use crate::models::chat::{ChatMessageRow, ChatSession};

pub struct ChatService;

impl ChatService {
    pub fn create_session(
        conn: &Connection,
        title: &str,
        system_prompt: Option<&str>,
    ) -> Result<ChatSession> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        conn.execute(
            "INSERT INTO chat_sessions (id, title, system_prompt, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, title, system_prompt, now, now],
        )?;

        Ok(ChatSession {
            id,
            title: title.to_string(),
            system_prompt: system_prompt.map(|s| s.to_string()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_sessions(conn: &Connection) -> Result<Vec<ChatSession>> {
        let mut stmt = conn.prepare(
            "SELECT id, title, system_prompt, created_at, updated_at FROM chat_sessions ORDER BY updated_at DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(ChatSession {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    system_prompt: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn get_session(conn: &Connection, id: &str) -> Result<Option<ChatSession>> {
        let result = conn.query_row(
            "SELECT id, title, system_prompt, created_at, updated_at FROM chat_sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok(ChatSession {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    system_prompt: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn update_session_title(conn: &Connection, id: &str, title: &str) -> Result<()> {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        conn.execute(
            "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, now, id],
        )?;

        Ok(())
    }

    pub fn delete_session(conn: &Connection, id: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM chat_messages WHERE session_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])?;

        Ok(())
    }

    pub fn add_message(
        conn: &Connection,
        session_id: &str,
        role: &str,
        content: &str,
        model: Option<&str>,
        tokens: Option<i64>,
    ) -> Result<ChatMessageRow> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content, model, tokens, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, session_id, role, content, model, tokens, now],
        )?;

        // Also update session's updated_at
        conn.execute(
            "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;

        Ok(ChatMessageRow {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            model: model.map(|s| s.to_string()),
            tokens,
            created_at: now,
        })
    }

    pub fn list_messages(conn: &Connection, session_id: &str) -> Result<Vec<ChatMessageRow>> {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, model, tokens, created_at FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;

        let messages = stmt
            .query_map(params![session_id], |row| {
                Ok(ChatMessageRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    model: row.get(4)?,
                    tokens: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(messages)
    }
}

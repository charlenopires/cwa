//! Memory and session related database queries.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Memory entry row from database.
#[derive(Debug, Clone)]
pub struct MemoryRow {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub entry_type: String,
    pub content: String,
    pub importance: String,
    pub tags: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

/// Session row from database.
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub project_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: Option<String>,
    pub goals: Option<String>,
    pub accomplishments: Option<String>,
}

/// Create a memory entry.
pub fn create_memory_entry(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    entry_type: &str,
    content: &str,
    importance: &str,
    session_id: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO memory (id, project_id, entry_type, content, importance, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, project_id, entry_type, content, importance, session_id],
        )?;
        Ok(())
    })
}

/// List memory entries for a project.
pub fn list_memory(pool: &DbPool, project_id: &str, limit: Option<i64>) -> DbResult<Vec<MemoryRow>> {
    pool.with_conn(|conn| {
        let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
        let sql = format!(
            "SELECT id, project_id, session_id, entry_type, content, importance,
                    tags, related_entity_type, related_entity_id, created_at, expires_at
             FROM memory WHERE project_id = ?1
             AND (expires_at IS NULL OR expires_at > datetime('now'))
             ORDER BY
                CASE importance
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'normal' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at DESC{}",
            limit_clause
        );

        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(MemoryRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                session_id: row.get(2)?,
                entry_type: row.get(3)?,
                content: row.get(4)?,
                importance: row.get(5)?,
                tags: row.get(6)?,
                related_entity_type: row.get(7)?,
                related_entity_id: row.get(8)?,
                created_at: row.get(9)?,
                expires_at: row.get(10)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Search memory by content (simple LIKE search).
pub fn search_memory(pool: &DbPool, project_id: &str, query: &str) -> DbResult<Vec<MemoryRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, session_id, entry_type, content, importance,
                    tags, related_entity_type, related_entity_id, created_at, expires_at
             FROM memory WHERE project_id = ?1 AND content LIKE ?2
             ORDER BY created_at DESC LIMIT 50",
        )?;

        let search_pattern = format!("%{}%", query);
        let rows = stmt.query_map(params![project_id, search_pattern], |row| {
            Ok(MemoryRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                session_id: row.get(2)?,
                entry_type: row.get(3)?,
                content: row.get(4)?,
                importance: row.get(5)?,
                tags: row.get(6)?,
                related_entity_type: row.get(7)?,
                related_entity_id: row.get(8)?,
                created_at: row.get(9)?,
                expires_at: row.get(10)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Delete expired memory entries.
pub fn cleanup_expired_memory(pool: &DbPool) -> DbResult<usize> {
    pool.with_conn(|conn| {
        let count = conn.execute(
            "DELETE FROM memory WHERE expires_at IS NOT NULL AND expires_at < datetime('now')",
            [],
        )?;
        Ok(count)
    })
}

/// Create a new session.
pub fn create_session(pool: &DbPool, id: &str, project_id: &str, goals: Option<&str>) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO sessions (id, project_id, goals) VALUES (?1, ?2, ?3)",
            params![id, project_id, goals],
        )?;
        Ok(())
    })
}

/// End a session.
pub fn end_session(pool: &DbPool, id: &str, summary: Option<&str>, accomplishments: Option<&str>) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE sessions SET ended_at = datetime('now'), summary = ?1, accomplishments = ?2 WHERE id = ?3",
            params![summary, accomplishments, id],
        )?;
        Ok(())
    })
}

/// Get the current active session.
pub fn get_active_session(pool: &DbPool, project_id: &str) -> DbResult<Option<SessionRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, summary, goals, accomplishments
             FROM sessions WHERE project_id = ?1 AND ended_at IS NULL
             ORDER BY started_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(SessionRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                summary: row.get(4)?,
                goals: row.get(5)?,
                accomplishments: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    })
}

/// List recent sessions.
pub fn list_sessions(pool: &DbPool, project_id: &str, limit: i64) -> DbResult<Vec<SessionRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, summary, goals, accomplishments
             FROM sessions WHERE project_id = ?1
             ORDER BY started_at DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![project_id, limit], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                summary: row.get(4)?,
                goals: row.get(5)?,
                accomplishments: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

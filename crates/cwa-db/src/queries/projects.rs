//! Project-related database queries.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Project row from database.
#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub constitution_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Create a new project.
pub fn create_project(pool: &DbPool, id: &str, name: &str, description: Option<&str>) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, description) VALUES (?1, ?2, ?3)",
            params![id, name, description],
        )?;
        Ok(())
    })
}

/// Get a project by ID.
pub fn get_project(pool: &DbPool, id: &str) -> DbResult<ProjectRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, name, description, constitution_path, status, created_at, updated_at
             FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    constitution_path: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Project: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// Get the first/default project (for single-project usage).
pub fn get_default_project(pool: &DbPool) -> DbResult<Option<ProjectRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, constitution_path, status, created_at, updated_at
             FROM projects WHERE status = 'active' ORDER BY created_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ProjectRow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                constitution_path: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    })
}

/// List all projects.
pub fn list_projects(pool: &DbPool) -> DbResult<Vec<ProjectRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, constitution_path, status, created_at, updated_at
             FROM projects ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                constitution_path: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Update project constitution path.
pub fn update_constitution_path(pool: &DbPool, project_id: &str, path: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE projects SET constitution_path = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![path, project_id],
        )?;
        Ok(())
    })
}

/// Update project name and description.
pub fn update_project(pool: &DbPool, id: &str, name: &str, description: Option<&str>) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![name, description, id],
        )?;
        Ok(())
    })
}

/// Get project info from memory table (entry_type = 'project_info').
pub fn get_project_info(pool: &DbPool, project_id: &str) -> DbResult<Option<String>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT content FROM memory
             WHERE project_id = ?1 AND entry_type = 'project_info'
             ORDER BY created_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    })
}

/// Set project info in memory table (upsert pattern: delete old, insert new).
pub fn set_project_info(pool: &DbPool, project_id: &str, content: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        // Delete existing project_info entries
        conn.execute(
            "DELETE FROM memory WHERE project_id = ?1 AND entry_type = 'project_info'",
            params![project_id],
        )?;

        // Insert new
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO memory (id, project_id, entry_type, content, importance)
             VALUES (?1, ?2, 'project_info', ?3, 'critical')",
            params![id, project_id, content],
        )?;
        Ok(())
    })
}

//! Specification-related database queries.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Spec row from database.
#[derive(Debug, Clone)]
pub struct SpecRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub acceptance_criteria: Option<String>,
    pub dependencies: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

/// Create a new spec.
pub fn create_spec(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO specs (id, project_id, title, description, priority)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, project_id, title, description, priority],
        )?;
        Ok(())
    })
}

/// Create a new spec with acceptance criteria.
pub fn create_spec_with_criteria(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
    criteria_json: &str,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO specs (id, project_id, title, description, priority, acceptance_criteria)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, project_id, title, description, priority, criteria_json],
        )?;
        Ok(())
    })
}

/// Get a spec by ID.
pub fn get_spec(pool: &DbPool, id: &str) -> DbResult<SpecRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, project_id, title, description, status, priority,
                    acceptance_criteria, dependencies, created_at, updated_at, archived_at
             FROM specs WHERE id = ?1",
            params![id],
            |row| {
                Ok(SpecRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: row.get(4)?,
                    priority: row.get(5)?,
                    acceptance_criteria: row.get(6)?,
                    dependencies: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    archived_at: row.get(10)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Spec: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// Get a spec by ID prefix (for short-ID lookup).
pub fn get_spec_by_id_prefix(pool: &DbPool, prefix: &str) -> DbResult<SpecRow> {
    pool.with_conn(|conn| {
        let pattern = format!("{}%", prefix);
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, status, priority,
                    acceptance_criteria, dependencies, created_at, updated_at, archived_at
             FROM specs WHERE id LIKE ?1",
        )?;
        let mut rows = stmt.query(params![pattern])?;
        let first = rows.next()?.ok_or_else(|| DbError::NotFound(format!("Spec: {}", prefix)))?;
        let spec = SpecRow {
            id: first.get(0)?,
            project_id: first.get(1)?,
            title: first.get(2)?,
            description: first.get(3)?,
            status: first.get(4)?,
            priority: first.get(5)?,
            acceptance_criteria: first.get(6)?,
            dependencies: first.get(7)?,
            created_at: first.get(8)?,
            updated_at: first.get(9)?,
            archived_at: first.get(10)?,
        };
        if rows.next()?.is_some() {
            return Err(DbError::NotFound(format!(
                "Spec prefix '{}' is ambiguous (matches multiple specs)",
                prefix
            )));
        }
        Ok(spec)
    })
}

/// Get spec by title (for name-based lookup).
pub fn get_spec_by_title(pool: &DbPool, project_id: &str, title: &str) -> DbResult<SpecRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, project_id, title, description, status, priority,
                    acceptance_criteria, dependencies, created_at, updated_at, archived_at
             FROM specs WHERE project_id = ?1 AND title = ?2",
            params![project_id, title],
            |row| {
                Ok(SpecRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: row.get(4)?,
                    priority: row.get(5)?,
                    acceptance_criteria: row.get(6)?,
                    dependencies: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    archived_at: row.get(10)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Spec: {}", title)),
            e => DbError::Connection(e),
        })
    })
}

/// List specs for a project.
pub fn list_specs(pool: &DbPool, project_id: &str) -> DbResult<Vec<SpecRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, status, priority,
                    acceptance_criteria, dependencies, created_at, updated_at, archived_at
             FROM specs WHERE project_id = ?1 AND archived_at IS NULL
             ORDER BY
                CASE priority
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(SpecRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                acceptance_criteria: row.get(6)?,
                dependencies: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                archived_at: row.get(10)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Get active spec (status = 'active').
pub fn get_active_spec(pool: &DbPool, project_id: &str) -> DbResult<Option<SpecRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, status, priority,
                    acceptance_criteria, dependencies, created_at, updated_at, archived_at
             FROM specs WHERE project_id = ?1 AND status = 'active' LIMIT 1",
        )?;

        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(SpecRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                acceptance_criteria: row.get(6)?,
                dependencies: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                archived_at: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    })
}

/// Update spec status.
pub fn update_spec_status(pool: &DbPool, id: &str, status: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        let archived_at = if status == "archived" {
            Some("datetime('now')")
        } else {
            None
        };

        if let Some(_) = archived_at {
            conn.execute(
                "UPDATE specs SET status = ?1, archived_at = datetime('now'), updated_at = datetime('now') WHERE id = ?2",
                params![status, id],
            )?;
        } else {
            conn.execute(
                "UPDATE specs SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![status, id],
            )?;
        }
        Ok(())
    })
}

/// Update spec acceptance criteria.
pub fn update_acceptance_criteria(pool: &DbPool, id: &str, criteria_json: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE specs SET acceptance_criteria = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![criteria_json, id],
        )?;
        Ok(())
    })
}

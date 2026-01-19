//! Task-related database queries.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::{params, OptionalExtension};

/// Task row from database.
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    pub project_id: String,
    pub spec_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub labels: Option<String>,
    pub estimated_effort: Option<String>,
    pub actual_effort: Option<String>,
    pub blocked_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Create a new task.
pub fn create_task(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    spec_id: Option<&str>,
    priority: &str,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO tasks (id, project_id, title, description, spec_id, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, project_id, title, description, spec_id, priority],
        )?;
        Ok(())
    })
}

/// Get a task by ID.
pub fn get_task(pool: &DbPool, id: &str) -> DbResult<TaskRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, project_id, spec_id, title, description, status, priority,
                    assignee, labels, estimated_effort, actual_effort, blocked_by,
                    created_at, updated_at, started_at, completed_at
             FROM tasks WHERE id = ?1",
            params![id],
            |row| {
                Ok(TaskRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    spec_id: row.get(2)?,
                    title: row.get(3)?,
                    description: row.get(4)?,
                    status: row.get(5)?,
                    priority: row.get(6)?,
                    assignee: row.get(7)?,
                    labels: row.get(8)?,
                    estimated_effort: row.get(9)?,
                    actual_effort: row.get(10)?,
                    blocked_by: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                    started_at: row.get(14)?,
                    completed_at: row.get(15)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Task: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// Get the current in-progress task.
pub fn get_current_task(pool: &DbPool, project_id: &str) -> DbResult<Option<TaskRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, spec_id, title, description, status, priority,
                    assignee, labels, estimated_effort, actual_effort, blocked_by,
                    created_at, updated_at, started_at, completed_at
             FROM tasks WHERE project_id = ?1 AND status = 'in_progress'
             ORDER BY started_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(TaskRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                spec_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                assignee: row.get(7)?,
                labels: row.get(8)?,
                estimated_effort: row.get(9)?,
                actual_effort: row.get(10)?,
                blocked_by: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
                started_at: row.get(14)?,
                completed_at: row.get(15)?,
            }))
        } else {
            Ok(None)
        }
    })
}

/// List tasks for a project.
pub fn list_tasks(pool: &DbPool, project_id: &str) -> DbResult<Vec<TaskRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, spec_id, title, description, status, priority,
                    assignee, labels, estimated_effort, actual_effort, blocked_by,
                    created_at, updated_at, started_at, completed_at
             FROM tasks WHERE project_id = ?1
             ORDER BY
                CASE status
                    WHEN 'in_progress' THEN 1
                    WHEN 'todo' THEN 2
                    WHEN 'backlog' THEN 3
                    WHEN 'review' THEN 4
                    WHEN 'done' THEN 5
                END,
                CASE priority
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(TaskRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                spec_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                assignee: row.get(7)?,
                labels: row.get(8)?,
                estimated_effort: row.get(9)?,
                actual_effort: row.get(10)?,
                blocked_by: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
                started_at: row.get(14)?,
                completed_at: row.get(15)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// List tasks by status.
pub fn list_tasks_by_status(pool: &DbPool, project_id: &str, status: &str) -> DbResult<Vec<TaskRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, spec_id, title, description, status, priority,
                    assignee, labels, estimated_effort, actual_effort, blocked_by,
                    created_at, updated_at, started_at, completed_at
             FROM tasks WHERE project_id = ?1 AND status = ?2
             ORDER BY
                CASE priority
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_id, status], |row| {
            Ok(TaskRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                spec_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                assignee: row.get(7)?,
                labels: row.get(8)?,
                estimated_effort: row.get(9)?,
                actual_effort: row.get(10)?,
                blocked_by: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
                started_at: row.get(14)?,
                completed_at: row.get(15)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Update task status.
pub fn update_task_status(pool: &DbPool, id: &str, status: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        let _now = "datetime('now')";

        // Handle timestamps based on status
        match status {
            "in_progress" => {
                conn.execute(
                    "UPDATE tasks SET status = ?1, started_at = COALESCE(started_at, datetime('now')), updated_at = datetime('now') WHERE id = ?2",
                    params![status, id],
                )?;
            }
            "done" => {
                conn.execute(
                    "UPDATE tasks SET status = ?1, completed_at = datetime('now'), updated_at = datetime('now') WHERE id = ?2",
                    params![status, id],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE tasks SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
                    params![status, id],
                )?;
            }
        }
        Ok(())
    })
}

/// Count tasks by status (for WIP limits).
pub fn count_tasks_by_status(pool: &DbPool, project_id: &str, status: &str) -> DbResult<i64> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = ?2",
            params![project_id, status],
            |row| row.get(0),
        )
        .map_err(DbError::from)
    })
}

/// Get WIP limit for a column.
pub fn get_wip_limit(pool: &DbPool, project_id: &str, column_name: &str) -> DbResult<Option<i64>> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT wip_limit FROM kanban_config WHERE project_id = ?1 AND column_name = ?2",
            params![project_id, column_name],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map(|opt| opt.flatten())
        .map_err(DbError::from)
    })
}

/// Set WIP limit for a column.
pub fn set_wip_limit(pool: &DbPool, project_id: &str, column_name: &str, limit: Option<i64>, order: i32) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO kanban_config (id, project_id, column_name, column_order, wip_limit)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_id, column_name) DO UPDATE SET wip_limit = ?5",
            params![
                format!("{}-{}", project_id, column_name),
                project_id,
                column_name,
                order,
                limit
            ],
        )?;
        Ok(())
    })
}

//! Decision (ADR) related database queries.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Decision row from database.
#[derive(Debug, Clone)]
pub struct DecisionRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub context: String,
    pub decision: String,
    pub consequences: Option<String>,
    pub alternatives: Option<String>,
    pub related_specs: Option<String>,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Create a new decision.
pub fn create_decision(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    title: &str,
    context: &str,
    decision: &str,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO decisions (id, project_id, title, context, decision)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, project_id, title, context, decision],
        )?;
        Ok(())
    })
}

/// Get a decision by ID.
pub fn get_decision(pool: &DbPool, id: &str) -> DbResult<DecisionRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, project_id, title, status, context, decision,
                    consequences, alternatives, related_specs, superseded_by,
                    created_at, updated_at
             FROM decisions WHERE id = ?1",
            params![id],
            |row| {
                Ok(DecisionRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    context: row.get(4)?,
                    decision: row.get(5)?,
                    consequences: row.get(6)?,
                    alternatives: row.get(7)?,
                    related_specs: row.get(8)?,
                    superseded_by: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Decision: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// List decisions for a project.
pub fn list_decisions(pool: &DbPool, project_id: &str) -> DbResult<Vec<DecisionRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, status, context, decision,
                    consequences, alternatives, related_specs, superseded_by,
                    created_at, updated_at
             FROM decisions WHERE project_id = ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(DecisionRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                context: row.get(4)?,
                decision: row.get(5)?,
                consequences: row.get(6)?,
                alternatives: row.get(7)?,
                related_specs: row.get(8)?,
                superseded_by: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// List accepted decisions.
pub fn list_accepted_decisions(pool: &DbPool, project_id: &str) -> DbResult<Vec<DecisionRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, status, context, decision,
                    consequences, alternatives, related_specs, superseded_by,
                    created_at, updated_at
             FROM decisions WHERE project_id = ?1 AND status = 'accepted'
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(DecisionRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                context: row.get(4)?,
                decision: row.get(5)?,
                consequences: row.get(6)?,
                alternatives: row.get(7)?,
                related_specs: row.get(8)?,
                superseded_by: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Update decision status.
pub fn update_decision_status(pool: &DbPool, id: &str, status: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE decisions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    })
}

/// Supersede a decision.
pub fn supersede_decision(pool: &DbPool, old_id: &str, new_id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE decisions SET status = 'superseded', superseded_by = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![new_id, old_id],
        )?;
        Ok(())
    })
}

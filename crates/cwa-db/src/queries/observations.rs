//! Observation and summary database queries.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Full observation row from database.
#[derive(Debug, Clone)]
pub struct ObservationRow {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub obs_type: String,
    pub title: String,
    pub narrative: Option<String>,
    pub facts: Option<String>,
    pub concepts: Option<String>,
    pub files_modified: Option<String>,
    pub files_read: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub confidence: f64,
    pub embedding_id: Option<String>,
    pub created_at: String,
}

/// Compact observation row for progressive disclosure (index only).
#[derive(Debug, Clone)]
pub struct ObservationIndexRow {
    pub id: String,
    pub obs_type: String,
    pub title: String,
    pub confidence: f64,
    pub created_at: String,
}

/// Summary row from database.
#[derive(Debug, Clone)]
pub struct SummaryRow {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub observations_count: i64,
    pub key_facts: Option<String>,
    pub time_range_start: Option<String>,
    pub time_range_end: Option<String>,
    pub created_at: String,
}

/// Create an observation.
pub fn create_observation(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    session_id: Option<&str>,
    obs_type: &str,
    title: &str,
    narrative: Option<&str>,
    facts: Option<&str>,
    concepts: Option<&str>,
    files_modified: Option<&str>,
    files_read: Option<&str>,
    related_entity_type: Option<&str>,
    related_entity_id: Option<&str>,
    confidence: f64,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO observations (id, project_id, session_id, obs_type, title, narrative,
             facts, concepts, files_modified, files_read, related_entity_type, related_entity_id, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![id, project_id, session_id, obs_type, title, narrative,
                    facts, concepts, files_modified, files_read,
                    related_entity_type, related_entity_id, confidence],
        )?;
        Ok(())
    })
}

/// Get a single observation by ID.
pub fn get_observation(pool: &DbPool, id: &str) -> DbResult<Option<ObservationRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, session_id, obs_type, title, narrative,
                    facts, concepts, files_modified, files_read,
                    related_entity_type, related_entity_id, confidence, embedding_id, created_at
             FROM observations WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_observation(row)?))
        } else {
            Ok(None)
        }
    })
}

/// Get multiple observations by IDs.
pub fn get_observations_batch(pool: &DbPool, ids: &[&str]) -> DbResult<Vec<ObservationRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    pool.with_conn(|conn| {
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT id, project_id, session_id, obs_type, title, narrative,
                    facts, concepts, files_modified, files_read,
                    related_entity_type, related_entity_id, confidence, embedding_id, created_at
             FROM observations WHERE id IN ({})
             ORDER BY created_at DESC",
            placeholders.join(", ")
        );

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), |row| row_to_observation(row))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// List observations in compact form (for progressive disclosure).
pub fn list_observations_compact(
    pool: &DbPool,
    project_id: &str,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<ObservationIndexRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, obs_type, title, confidence, created_at
             FROM observations WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let rows = stmt.query_map(params![project_id, limit, offset], |row| {
            Ok(ObservationIndexRow {
                id: row.get(0)?,
                obs_type: row.get(1)?,
                title: row.get(2)?,
                confidence: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// List observations as a timeline (grouped by day, most recent first).
pub fn list_observations_timeline(
    pool: &DbPool,
    project_id: &str,
    days_back: i64,
    limit: i64,
) -> DbResult<Vec<ObservationIndexRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, obs_type, title, confidence, created_at
             FROM observations
             WHERE project_id = ?1
               AND created_at >= datetime('now', ?2)
             ORDER BY created_at DESC
             LIMIT ?3",
        )?;

        let days_modifier = format!("-{} days", days_back);
        let rows = stmt.query_map(params![project_id, days_modifier, limit], |row| {
            Ok(ObservationIndexRow {
                id: row.get(0)?,
                obs_type: row.get(1)?,
                title: row.get(2)?,
                confidence: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// List high-confidence observations (full details).
pub fn list_high_confidence(
    pool: &DbPool,
    project_id: &str,
    min_confidence: f64,
    limit: i64,
) -> DbResult<Vec<ObservationRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, session_id, obs_type, title, narrative,
                    facts, concepts, files_modified, files_read,
                    related_entity_type, related_entity_id, confidence, embedding_id, created_at
             FROM observations
             WHERE project_id = ?1 AND confidence >= ?2
             ORDER BY confidence DESC, created_at DESC
             LIMIT ?3",
        )?;

        let rows = stmt.query_map(params![project_id, min_confidence, limit], |row| {
            row_to_observation(row)
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Update an observation's confidence.
pub fn update_confidence(pool: &DbPool, id: &str, confidence: f64) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE observations SET confidence = ?1 WHERE id = ?2",
            params![confidence, id],
        )?;
        Ok(())
    })
}

/// Update an observation's embedding ID.
pub fn update_embedding_id(pool: &DbPool, id: &str, embedding_id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE observations SET embedding_id = ?1 WHERE id = ?2",
            params![embedding_id, id],
        )?;
        Ok(())
    })
}

/// Decay confidence for all observations in a project by a factor.
pub fn decay_all_confidence(pool: &DbPool, project_id: &str, factor: f64) -> DbResult<usize> {
    pool.with_conn(|conn| {
        let count = conn.execute(
            "UPDATE observations SET confidence = confidence * ?1 WHERE project_id = ?2",
            params![factor, project_id],
        )?;
        Ok(count)
    })
}

/// Remove observations below a confidence threshold.
pub fn remove_low_confidence(pool: &DbPool, project_id: &str, min_confidence: f64) -> DbResult<Vec<String>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id FROM observations WHERE project_id = ?1 AND confidence < ?2",
        )?;

        let ids: Vec<String> = stmt
            .query_map(params![project_id, min_confidence], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for id in &ids {
            conn.execute("DELETE FROM observations WHERE id = ?1", params![id])?;
        }

        Ok(ids)
    })
}

/// Create a summary.
pub fn create_summary(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    session_id: Option<&str>,
    content: &str,
    observations_count: i64,
    key_facts: Option<&str>,
    time_range_start: Option<&str>,
    time_range_end: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO summaries (id, project_id, session_id, content, observations_count,
             key_facts, time_range_start, time_range_end)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, project_id, session_id, content, observations_count,
                    key_facts, time_range_start, time_range_end],
        )?;
        Ok(())
    })
}

/// Get recent summaries for a project.
pub fn get_recent_summaries(pool: &DbPool, project_id: &str, limit: i64) -> DbResult<Vec<SummaryRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, session_id, content, observations_count,
                    key_facts, time_range_start, time_range_end, created_at
             FROM summaries WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![project_id, limit], |row| {
            Ok(SummaryRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                session_id: row.get(2)?,
                content: row.get(3)?,
                observations_count: row.get(4)?,
                key_facts: row.get(5)?,
                time_range_start: row.get(6)?,
                time_range_end: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Helper to map a rusqlite Row into an ObservationRow.
fn row_to_observation(row: &rusqlite::Row) -> rusqlite::Result<ObservationRow> {
    Ok(ObservationRow {
        id: row.get(0)?,
        project_id: row.get(1)?,
        session_id: row.get(2)?,
        obs_type: row.get(3)?,
        title: row.get(4)?,
        narrative: row.get(5)?,
        facts: row.get(6)?,
        concepts: row.get(7)?,
        files_modified: row.get(8)?,
        files_read: row.get(9)?,
        related_entity_type: row.get(10)?,
        related_entity_id: row.get(11)?,
        confidence: row.get(12)?,
        embedding_id: row.get(13)?,
        created_at: row.get(14)?,
    })
}

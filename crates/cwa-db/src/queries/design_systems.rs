//! Design system database queries.

use crate::pool::{DbPool, DbError};
use rusqlite::params;

/// Row from the design_systems table.
#[derive(Debug, Clone)]
pub struct DesignSystemRow {
    pub id: String,
    pub project_id: String,
    pub source_url: String,
    pub colors_json: Option<String>,
    pub typography_json: Option<String>,
    pub spacing_json: Option<String>,
    pub border_radius_json: Option<String>,
    pub shadows_json: Option<String>,
    pub breakpoints_json: Option<String>,
    pub components_json: Option<String>,
    pub raw_analysis: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Create a new design system entry.
pub fn create_design_system(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    source_url: &str,
    colors_json: Option<&str>,
    typography_json: Option<&str>,
    spacing_json: Option<&str>,
    border_radius_json: Option<&str>,
    shadows_json: Option<&str>,
    breakpoints_json: Option<&str>,
    components_json: Option<&str>,
    raw_analysis: Option<&str>,
) -> Result<(), DbError> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO design_systems (id, project_id, source_url, colors_json, typography_json, spacing_json, border_radius_json, shadows_json, breakpoints_json, components_json, raw_analysis)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id, project_id, source_url,
                colors_json, typography_json, spacing_json,
                border_radius_json, shadows_json, breakpoints_json,
                components_json, raw_analysis
            ],
        )?;
        Ok(())
    })
}

/// Get the most recent design system for a project.
pub fn get_latest_design_system(pool: &DbPool, project_id: &str) -> Result<Option<DesignSystemRow>, DbError> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, source_url, colors_json, typography_json, spacing_json,
                    border_radius_json, shadows_json, breakpoints_json, components_json,
                    raw_analysis, created_at, updated_at
             FROM design_systems
             WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![project_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(DesignSystemRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_url: row.get(2)?,
                colors_json: row.get(3)?,
                typography_json: row.get(4)?,
                spacing_json: row.get(5)?,
                border_radius_json: row.get(6)?,
                shadows_json: row.get(7)?,
                breakpoints_json: row.get(8)?,
                components_json: row.get(9)?,
                raw_analysis: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })),
            None => Ok(None),
        }
    })
}

/// List all design systems for a project.
pub fn list_design_systems(pool: &DbPool, project_id: &str) -> Result<Vec<DesignSystemRow>, DbError> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, source_url, colors_json, typography_json, spacing_json,
                    border_radius_json, shadows_json, breakpoints_json, components_json,
                    raw_analysis, created_at, updated_at
             FROM design_systems
             WHERE project_id = ?1
             ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(DesignSystemRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_url: row.get(2)?,
                colors_json: row.get(3)?,
                typography_json: row.get(4)?,
                spacing_json: row.get(5)?,
                border_radius_json: row.get(6)?,
                shadows_json: row.get(7)?,
                breakpoints_json: row.get(8)?,
                components_json: row.get(9)?,
                raw_analysis: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| DbError::Connection(e))
    })
}

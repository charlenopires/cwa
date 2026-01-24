//! Design system module.
//!
//! Handles extraction, storage, and retrieval of design systems
//! from UI screenshots via the Claude Vision API.

pub mod model;
pub mod vision;

use anyhow::Result;
use cwa_db::DbPool;

use model::DesignSystem;

/// Store a new design system in the database.
pub fn store_design_system(pool: &DbPool, design: &DesignSystem) -> Result<()> {
    let colors_json = serde_json::to_string(&design.colors)?;
    let typography_json = serde_json::to_string(&design.typography)?;
    let spacing_json = serde_json::to_string(&design.spacing)?;
    let border_radius_json = serde_json::to_string(&design.border_radius)?;
    let shadows_json = serde_json::to_string(&design.shadows)?;
    let breakpoints_json = serde_json::to_string(&design.breakpoints)?;
    let components_json = serde_json::to_string(&design.components)?;

    cwa_db::queries::design_systems::create_design_system(
        pool,
        &design.id,
        &design.project_id,
        &design.source_url,
        Some(&colors_json),
        Some(&typography_json),
        Some(&spacing_json),
        Some(&border_radius_json),
        Some(&shadows_json),
        Some(&breakpoints_json),
        Some(&components_json),
        if design.raw_analysis.is_empty() { None } else { Some(&design.raw_analysis) },
    )
    .map_err(|e| anyhow::anyhow!("Failed to store design system: {}", e))
}

/// Get the latest design system for a project.
pub fn get_design_system(pool: &DbPool, project_id: &str) -> Result<Option<DesignSystem>> {
    let row = cwa_db::queries::design_systems::get_latest_design_system(pool, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to get design system: {}", e))?;

    Ok(row.map(DesignSystem::from_row))
}

/// List all design systems for a project.
pub fn list_design_systems(pool: &DbPool, project_id: &str) -> Result<Vec<DesignSystem>> {
    let rows = cwa_db::queries::design_systems::list_design_systems(pool, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list design systems: {}", e))?;

    Ok(rows.into_iter().map(DesignSystem::from_row).collect())
}

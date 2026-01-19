//! Specification management (SDD).

pub mod model;

use crate::error::{CwaError, CwaResult};
use cwa_db::DbPool;
use cwa_db::queries::specs as queries;
use model::{Spec, SpecStatus, Priority};
use uuid::Uuid;

/// Create a new specification.
pub fn create_spec(
    pool: &DbPool,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
) -> CwaResult<Spec> {
    let id = Uuid::new_v4().to_string();
    let _priority_enum = Priority::from_str(priority);

    queries::create_spec(pool, &id, project_id, title, description, priority)?;

    let row = queries::get_spec(pool, &id)?;
    Ok(Spec::from_row(row))
}

/// Get a spec by ID or title.
pub fn get_spec(pool: &DbPool, project_id: &str, identifier: &str) -> CwaResult<Spec> {
    // Try by ID first, then by title
    match queries::get_spec(pool, identifier) {
        Ok(row) => Ok(Spec::from_row(row)),
        Err(cwa_db::DbError::NotFound(_)) => {
            let row = queries::get_spec_by_title(pool, project_id, identifier)
                .map_err(|_| CwaError::SpecNotFound(identifier.to_string()))?;
            Ok(Spec::from_row(row))
        }
        Err(e) => Err(e.into()),
    }
}

/// List all specs for a project.
pub fn list_specs(pool: &DbPool, project_id: &str) -> CwaResult<Vec<Spec>> {
    let rows = queries::list_specs(pool, project_id)?;
    Ok(rows.into_iter().map(Spec::from_row).collect())
}

/// Get the active spec for a project.
pub fn get_active_spec(pool: &DbPool, project_id: &str) -> CwaResult<Option<Spec>> {
    let row = queries::get_active_spec(pool, project_id)?;
    Ok(row.map(Spec::from_row))
}

/// Update spec status.
pub fn update_status(pool: &DbPool, id: &str, status: &str) -> CwaResult<()> {
    let current = queries::get_spec(pool, id)?;
    let current_status = SpecStatus::from_str(&current.status);
    let new_status = SpecStatus::from_str(status);

    // Validate transition
    if !current_status.can_transition_to(&new_status) {
        return Err(CwaError::InvalidStateTransition {
            from: current.status,
            to: status.to_string(),
        });
    }

    queries::update_spec_status(pool, id, status)?;
    Ok(())
}

/// Archive a spec.
pub fn archive_spec(pool: &DbPool, id: &str) -> CwaResult<()> {
    update_status(pool, id, "archived")
}

/// Validate a spec (placeholder - would check implementation).
pub fn validate_spec(pool: &DbPool, id: &str) -> CwaResult<ValidationResult> {
    let spec = queries::get_spec(pool, id)?;

    let mut issues = Vec::new();

    // Check acceptance criteria
    if spec.acceptance_criteria.is_none() {
        issues.push("Missing acceptance criteria".to_string());
    }

    // More validation logic would go here...

    Ok(ValidationResult {
        spec_id: id.to_string(),
        is_valid: issues.is_empty(),
        issues,
    })
}

/// Result of spec validation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub spec_id: String,
    pub is_valid: bool,
    pub issues: Vec<String>,
}

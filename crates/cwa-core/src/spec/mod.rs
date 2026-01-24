//! Specification management (SDD).

pub mod model;
pub mod parser;

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
    create_spec_with_criteria(pool, project_id, title, description, priority, None)
}

/// Create a new specification with optional acceptance criteria.
pub fn create_spec_with_criteria(
    pool: &DbPool,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
    criteria: Option<&[String]>,
) -> CwaResult<Spec> {
    let id = Uuid::new_v4().to_string();
    let _priority_enum = Priority::from_str(priority);

    if let Some(criteria) = criteria {
        let criteria_json = serde_json::to_string(criteria)
            .map_err(|e| CwaError::ValidationError(format!("Failed to serialize criteria: {}", e)))?;
        queries::create_spec_with_criteria(pool, &id, project_id, title, description, priority, &criteria_json)?;
    } else {
        queries::create_spec(pool, &id, project_id, title, description, priority)?;
    }

    let row = queries::get_spec(pool, &id)?;
    Ok(Spec::from_row(row))
}

/// Get a spec by ID, ID prefix, or title.
pub fn get_spec(pool: &DbPool, project_id: &str, identifier: &str) -> CwaResult<Spec> {
    // Try exact ID first
    match queries::get_spec(pool, identifier) {
        Ok(row) => return Ok(Spec::from_row(row)),
        Err(cwa_db::DbError::NotFound(_)) => {}
        Err(e) => return Err(e.into()),
    }
    // Try ID prefix match
    match queries::get_spec_by_id_prefix(pool, identifier) {
        Ok(row) => return Ok(Spec::from_row(row)),
        Err(cwa_db::DbError::NotFound(_)) => {}
        Err(e) => return Err(e.into()),
    }
    // Try by title
    let row = queries::get_spec_by_title(pool, project_id, identifier)
        .map_err(|_| CwaError::SpecNotFound(identifier.to_string()))?;
    Ok(Spec::from_row(row))
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

/// Clear all specs for a project. Returns the number of deleted specs.
pub fn clear_specs(pool: &DbPool, project_id: &str) -> CwaResult<usize> {
    let count = queries::delete_all_specs(pool, project_id)?;
    Ok(count)
}

/// Add acceptance criteria to an existing spec (appends to existing list).
pub fn add_acceptance_criteria(
    pool: &DbPool,
    project_id: &str,
    identifier: &str,
    new_criteria: &[String],
) -> CwaResult<Spec> {
    let spec = get_spec(pool, project_id, identifier)?;

    let mut criteria = spec.acceptance_criteria.clone();
    criteria.extend(new_criteria.iter().cloned());

    let criteria_json = serde_json::to_string(&criteria)
        .map_err(|e| CwaError::ValidationError(format!("Failed to serialize criteria: {}", e)))?;
    queries::update_acceptance_criteria(pool, &spec.id, &criteria_json)?;

    let row = queries::get_spec(pool, &spec.id)?;
    Ok(Spec::from_row(row))
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

/// Create multiple specs from a parsed prompt.
pub fn create_specs_from_prompt(
    pool: &DbPool,
    project_id: &str,
    input: &str,
    priority: &str,
) -> CwaResult<Vec<Spec>> {
    let parsed = parser::parse_prompt(input);
    if parsed.is_empty() {
        return Err(CwaError::ValidationError("No specs could be parsed from the input".to_string()));
    }

    let mut specs = Vec::new();
    for entry in parsed {
        let prio = if entry.priority == "medium" { priority } else { &entry.priority };
        let spec = create_spec(pool, project_id, &entry.title, entry.description.as_deref(), prio)?;
        specs.push(spec);
    }

    Ok(specs)
}

/// Result of spec validation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub spec_id: String,
    pub is_valid: bool,
    pub issues: Vec<String>,
}

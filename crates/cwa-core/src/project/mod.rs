//! Project management and scaffolding.

pub mod model;
pub mod scaffold;

use crate::error::{CwaError, CwaResult};
use cwa_db::DbPool;
use cwa_db::queries::projects as queries;
use model::Project;
use uuid::Uuid;

/// Create a new project in the database.
pub fn create_project(pool: &DbPool, name: &str, description: Option<&str>) -> CwaResult<Project> {
    let id = Uuid::new_v4().to_string();

    queries::create_project(pool, &id, name, description)?;

    let row = queries::get_project(pool, &id)?;
    Ok(Project::from_row(row))
}

/// Get a project by ID.
pub fn get_project(pool: &DbPool, id: &str) -> CwaResult<Project> {
    let row = queries::get_project(pool, id)?;
    Ok(Project::from_row(row))
}

/// Get the default/active project.
pub fn get_default_project(pool: &DbPool) -> CwaResult<Option<Project>> {
    let row = queries::get_default_project(pool)?;
    Ok(row.map(Project::from_row))
}

/// List all projects.
pub fn list_projects(pool: &DbPool) -> CwaResult<Vec<Project>> {
    let rows = queries::list_projects(pool)?;
    Ok(rows.into_iter().map(Project::from_row).collect())
}

/// Get the project constitution content.
pub fn get_constitution(pool: &DbPool, project_id: &str) -> CwaResult<String> {
    let project = queries::get_project(pool, project_id)?;

    if let Some(path) = project.constitution_path {
        std::fs::read_to_string(&path).map_err(|e| {
            CwaError::Io(e)
        })
    } else {
        Ok("No constitution defined for this project.".to_string())
    }
}

/// Update the constitution path for a project.
pub fn set_constitution_path(pool: &DbPool, project_id: &str, path: &str) -> CwaResult<()> {
    queries::update_constitution_path(pool, project_id, path)?;
    Ok(())
}

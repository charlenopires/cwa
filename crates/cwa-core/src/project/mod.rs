//! Project management and scaffolding.

pub mod model;
pub mod scaffold;

use crate::error::{CwaError, CwaResult};
use cwa_db::DbPool;
use cwa_db::queries::projects as queries;
use model::{Project, ProjectInfo};
use uuid::Uuid;

/// Create a new project in the database.
pub async fn create_project(pool: &DbPool, name: &str, description: Option<&str>) -> CwaResult<Project> {
    let id = Uuid::new_v4().to_string();
    queries::create_project(pool, &id, name, description).await?;
    let row = queries::get_project(pool, &id).await?;
    Ok(Project::from_row(row))
}

/// Get a project by ID.
pub async fn get_project(pool: &DbPool, id: &str) -> CwaResult<Project> {
    let row = queries::get_project(pool, id).await?;
    Ok(Project::from_row(row))
}

/// Get the default/active project.
pub async fn get_default_project(pool: &DbPool) -> CwaResult<Option<Project>> {
    let row = queries::get_default_project(pool).await?;
    Ok(row.map(Project::from_row))
}

/// List all projects.
pub async fn list_projects(pool: &DbPool) -> CwaResult<Vec<Project>> {
    let rows = queries::list_projects(pool).await?;
    Ok(rows.into_iter().map(Project::from_row).collect())
}

/// Get the project constitution content.
pub async fn get_constitution(pool: &DbPool, project_id: &str) -> CwaResult<String> {
    let project = queries::get_project(pool, project_id).await?;

    if let Some(path) = project.constitution_path {
        std::fs::read_to_string(&path).map_err(CwaError::Io)
    } else {
        Ok("No constitution defined for this project.".to_string())
    }
}

/// Update the constitution path for a project.
pub async fn set_constitution_path(pool: &DbPool, project_id: &str, path: &str) -> CwaResult<()> {
    queries::update_constitution_path(pool, project_id, path).await?;
    Ok(())
}

/// Update project name and description.
pub async fn update_project(pool: &DbPool, id: &str, name: &str, description: Option<&str>) -> CwaResult<()> {
    queries::update_project(pool, id, name, description).await?;
    Ok(())
}

/// Get project info (extended metadata).
pub async fn get_project_info(pool: &DbPool, project_id: &str) -> CwaResult<Option<ProjectInfo>> {
    let json = queries::get_project_info(pool, project_id).await?;
    match json {
        Some(j) => {
            let info = ProjectInfo::from_json(&j)
                .map_err(|e| CwaError::validation(format!("Invalid project info JSON: {}", e)))?;
            Ok(Some(info))
        }
        None => Ok(None),
    }
}

/// Set project info (extended metadata).
pub async fn set_project_info(pool: &DbPool, project_id: &str, info: &ProjectInfo) -> CwaResult<()> {
    let json = info.to_json()
        .map_err(|e| CwaError::validation(format!("Failed to serialize project info: {}", e)))?;
    queries::set_project_info(pool, project_id, &json).await?;
    Ok(())
}

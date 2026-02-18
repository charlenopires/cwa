//! Spec route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateSpecRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
}

#[derive(Deserialize)]
pub struct GenerateTasksRequest {
    pub status: Option<String>,
}

pub async fn list_specs(
    State(state): State<AppState>,
) -> Result<Json<Vec<cwa_core::spec::model::Spec>>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let specs = cwa_core::spec::list_specs(&state.db, &project.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(specs))
}

pub async fn get_spec(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<cwa_core::spec::model::Spec>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let spec = cwa_core::spec::get_spec(&state.db, &project.id, &id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(spec))
}

pub async fn create_spec(
    State(state): State<AppState>,
    Json(req): Json<CreateSpecRequest>,
) -> Result<(StatusCode, Json<cwa_core::spec::model::Spec>), (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let spec = cwa_core::spec::create_spec(
        &state.db,
        &project.id,
        &req.title,
        req.description.as_deref(),
        req.priority.as_deref().unwrap_or("medium"),
    ).await
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(spec)))
}

pub async fn generate_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<GenerateTasksRequest>>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let status = body.as_ref()
        .and_then(|b| b.status.as_deref())
        .unwrap_or("backlog");

    let result = cwa_core::task::generate_tasks_from_spec(&state.db, &project.id, &id, status).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "created": result.created.len(),
        "skipped": result.skipped,
        "tasks": result.created
    }))))
}

//! Task route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::state::{AppState, WebSocketMessage};

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub spec_id: Option<String>,
    pub priority: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<String>,
    pub priority: Option<String>,
}

pub async fn list_tasks(
    State(state): State<AppState>,
) -> Result<Json<Vec<cwa_core::task::model::Task>>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let tasks = cwa_core::task::list_tasks(&state.db, &project.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tasks))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<cwa_core::task::model::Task>, (StatusCode, String)> {
    let task = cwa_core::task::get_task(&state.db, &id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(task))
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<cwa_core::task::model::Task>), (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let task = cwa_core::task::create_task(
        &state.db,
        &project.id,
        &req.title,
        req.description.as_deref(),
        req.spec_id.as_deref(),
        req.priority.as_deref().unwrap_or("medium"),
    ).await
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    state.broadcast(WebSocketMessage::BoardRefresh);

    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<cwa_core::task::model::Task>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    if let Some(status) = &req.status {
        cwa_core::task::move_task(&state.db, &project.id, &id, status).await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        state.broadcast(WebSocketMessage::TaskUpdated {
            task_id: id.clone(),
            status: status.clone(),
        });
    }

    let task = cwa_core::task::get_task(&state.db, &id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(task))
}

pub async fn get_board(
    State(state): State<AppState>,
) -> Result<Json<cwa_core::task::model::Board>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let board = cwa_core::task::get_board(&state.db, &project.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(board))
}

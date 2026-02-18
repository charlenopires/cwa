//! Domain route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::state::AppState;

pub async fn list_contexts(
    State(state): State<AppState>,
) -> Result<Json<Vec<cwa_core::domain::model::BoundedContext>>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let contexts = cwa_core::domain::list_contexts(&state.db, &project.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(contexts))
}

pub async fn get_context(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<cwa_core::domain::model::BoundedContext>, (StatusCode, String)> {
    let context = cwa_core::domain::get_context(&state.db, &id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(context))
}

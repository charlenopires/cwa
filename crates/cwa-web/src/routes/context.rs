//! Context route handlers.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::state::AppState;

pub async fn get_summary(
    State(state): State<AppState>,
) -> Result<Json<cwa_core::memory::model::ContextSummary>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let summary = cwa_core::memory::get_context_summary(&state.db, &project.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(summary))
}

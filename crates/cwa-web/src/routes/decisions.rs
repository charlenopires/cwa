//! Decision route handlers.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateDecisionRequest {
    pub title: String,
    pub context: String,
    pub decision: String,
}

pub async fn list_decisions(
    State(state): State<AppState>,
) -> Result<Json<Vec<cwa_core::decision::model::Decision>>, (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let decisions = cwa_core::decision::list_decisions(&state.db, &project.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(decisions))
}

pub async fn create_decision(
    State(state): State<AppState>,
    Json(req): Json<CreateDecisionRequest>,
) -> Result<(StatusCode, Json<cwa_core::decision::model::Decision>), (StatusCode, String)> {
    let project = cwa_core::project::get_default_project(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No project found".to_string()))?;

    let decision = cwa_core::decision::create_decision(
        &state.db,
        &project.id,
        &req.title,
        &req.context,
        &req.decision,
    ).await
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(decision)))
}

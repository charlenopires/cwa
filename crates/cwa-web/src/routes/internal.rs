//! Internal notification endpoints.

use axum::{extract::State, http::StatusCode, Json};

use crate::state::{AppState, WebSocketMessage};

/// Receive a notification and broadcast to all WebSocket clients.
pub async fn notify(
    State(state): State<AppState>,
    Json(msg): Json<WebSocketMessage>,
) -> StatusCode {
    state.broadcast(msg);
    StatusCode::OK
}

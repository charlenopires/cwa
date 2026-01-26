//! Internal notification endpoints.

use axum::{extract::State, http::StatusCode, Json};
use tracing::{debug, info};

use crate::state::{AppState, WebSocketMessage};

/// Receive a notification and broadcast to all WebSocket clients.
pub async fn notify(
    State(state): State<AppState>,
    Json(msg): Json<WebSocketMessage>,
) -> StatusCode {
    info!(?msg, "Received internal notification, broadcasting to WebSocket clients");
    let receiver_count = state.tx.receiver_count();
    debug!(receiver_count, "Active WebSocket receivers");
    state.broadcast(msg);
    StatusCode::OK
}

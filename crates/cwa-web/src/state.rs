//! Application state.

use cwa_db::DbPool;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// WebSocket message types.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    TaskUpdated { task_id: String, status: String },
    SpecUpdated { spec_id: String },
    BoardRefresh,
}

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub tx: broadcast::Sender<WebSocketMessage>,
}

impl AppState {
    pub fn new(db: Arc<DbPool>) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { db, tx }
    }

    /// Broadcast a message to all WebSocket clients.
    pub fn broadcast(&self, msg: WebSocketMessage) {
        let _ = self.tx.send(msg);
    }
}

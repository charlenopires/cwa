//! Application state.

use std::sync::Arc;

// Re-export types for use in routes
pub use cwa_db::{BroadcastSender, DbPool, WebSocketMessage};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub tx: BroadcastSender,
}

impl AppState {
    /// Create new app state with a shared broadcast sender.
    pub fn new(db: Arc<DbPool>, tx: BroadcastSender) -> Self {
        Self { db, tx }
    }

    /// Broadcast a message to all WebSocket clients.
    pub fn broadcast(&self, msg: WebSocketMessage) {
        let _ = self.tx.send(msg);
    }
}

//! Shared broadcast channel for real-time updates.
//!
//! This module provides a broadcast channel that can be shared between
//! the MCP server and Web server for real-time WebSocket notifications.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// WebSocket message types for real-time updates.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// A task's status was updated.
    TaskUpdated { task_id: String, status: String },
    /// A spec was updated.
    SpecUpdated { spec_id: String },
    /// Request a full board refresh.
    BoardRefresh,
}

/// Type alias for the broadcast sender.
pub type BroadcastSender = broadcast::Sender<WebSocketMessage>;

/// Type alias for the broadcast receiver.
pub type BroadcastReceiver = broadcast::Receiver<WebSocketMessage>;

/// Create a new broadcast channel with default capacity.
pub fn create_broadcast_channel() -> BroadcastSender {
    let (tx, _rx) = broadcast::channel(100);
    tx
}

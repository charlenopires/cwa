//! Web server notifier for real-time updates.
//!
//! This module provides a shared notifier that CLI and MCP can use
//! to notify the web server of changes for live reload functionality.

use std::time::Duration;
use tracing::{debug, warn};

/// Default web server URL.
const DEFAULT_WEB_URL: &str = "http://127.0.0.1:3030";

/// Notifies the web server of changes via HTTP.
#[derive(Clone)]
pub struct WebNotifier {
    client: reqwest::Client,
    base_url: String,
}

impl WebNotifier {
    /// Create a new notifier with default settings.
    ///
    /// Uses the `CWA_WEB_URL` environment variable if set,
    /// otherwise defaults to `http://127.0.0.1:3030`.
    pub fn new() -> Self {
        let base_url = std::env::var("CWA_WEB_URL")
            .unwrap_or_else(|_| DEFAULT_WEB_URL.to_string());
        debug!(base_url = %base_url, "WebNotifier initialized");
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_default(),
            base_url,
        }
    }

    /// Create a notifier with a custom base URL.
    pub fn with_url(base_url: &str) -> Self {
        debug!(base_url = %base_url, "WebNotifier initialized with custom URL");
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_default(),
            base_url: base_url.to_string(),
        }
    }

    /// Notify the web server that a task status was updated.
    ///
    /// This sends an HTTP POST to `/internal/notify` which broadcasts
    /// the update to all connected WebSocket clients.
    pub async fn notify_task_updated(&self, task_id: &str, status: &str) {
        let url = format!("{}/internal/notify", self.base_url);
        let payload = serde_json::json!({
            "type": "TaskUpdated",
            "data": { "task_id": task_id, "status": status }
        });

        debug!(url = %url, task_id = %task_id, status = %status, "Sending task update notification");

        match self.client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!(task_id = %task_id, "Task update notification sent successfully");
                } else {
                    warn!(
                        task_id = %task_id,
                        status_code = %response.status(),
                        "Task update notification failed with status"
                    );
                }
            }
            Err(e) => {
                // This is expected if cwa serve is not running - just debug log
                debug!(
                    task_id = %task_id,
                    error = %e,
                    url = %url,
                    "Failed to send task update notification (cwa serve may not be running)"
                );
            }
        }
    }

    /// Notify the web server to refresh the entire board.
    pub async fn notify_board_refresh(&self) {
        let url = format!("{}/internal/notify", self.base_url);
        let payload = serde_json::json!({
            "type": "BoardRefresh"
        });

        debug!(url = %url, "Sending board refresh notification");

        match self.client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Board refresh notification sent successfully");
                } else {
                    warn!(
                        status_code = %response.status(),
                        "Board refresh notification failed with status"
                    );
                }
            }
            Err(e) => {
                debug!(
                    error = %e,
                    url = %url,
                    "Failed to send board refresh notification (cwa serve may not be running)"
                );
            }
        }
    }

    /// Notify the web server that a spec was updated.
    pub async fn notify_spec_updated(&self, spec_id: &str) {
        let url = format!("{}/internal/notify", self.base_url);
        let payload = serde_json::json!({
            "type": "SpecUpdated",
            "data": { "spec_id": spec_id }
        });

        debug!(url = %url, spec_id = %spec_id, "Sending spec update notification");

        match self.client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!(spec_id = %spec_id, "Spec update notification sent successfully");
                } else {
                    warn!(
                        spec_id = %spec_id,
                        status_code = %response.status(),
                        "Spec update notification failed with status"
                    );
                }
            }
            Err(e) => {
                debug!(
                    spec_id = %spec_id,
                    error = %e,
                    url = %url,
                    "Failed to send spec update notification (cwa serve may not be running)"
                );
            }
        }
    }
}

impl Default for WebNotifier {
    fn default() -> Self {
        Self::new()
    }
}

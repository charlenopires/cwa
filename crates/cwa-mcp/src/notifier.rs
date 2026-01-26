//! Web server notifier for real-time updates.

use std::time::Duration;
use tracing::{debug, warn};

/// Notifies the web server of changes via HTTP.
pub struct WebNotifier {
    client: reqwest::Client,
    base_url: String,
}

impl WebNotifier {
    /// Create a new notifier with default settings.
    pub fn new() -> Self {
        let base_url = std::env::var("CWA_WEB_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3030".to_string());
        debug!(base_url = %base_url, "WebNotifier initialized");
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_default(),
            base_url,
        }
    }

    /// Notify the web server that a task status was updated.
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
                warn!(
                    task_id = %task_id,
                    error = %e,
                    url = %url,
                    "Failed to send task update notification (is cwa serve running?)"
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

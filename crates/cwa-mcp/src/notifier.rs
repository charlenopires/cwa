//! Web server notifier for real-time updates.

use std::time::Duration;

/// Notifies the web server of changes via HTTP.
pub struct WebNotifier {
    client: reqwest::Client,
    base_url: String,
}

impl WebNotifier {
    /// Create a new notifier with default settings.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_millis(500))
                .build()
                .unwrap_or_default(),
            base_url: std::env::var("CWA_WEB_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3030".to_string()),
        }
    }

    /// Notify the web server that a task status was updated.
    pub async fn notify_task_updated(&self, task_id: &str, status: &str) {
        let _ = self
            .client
            .post(format!("{}/internal/notify", self.base_url))
            .json(&serde_json::json!({
                "type": "TaskUpdated",
                "data": { "task_id": task_id, "status": status }
            }))
            .send()
            .await;
    }
}

impl Default for WebNotifier {
    fn default() -> Self {
        Self::new()
    }
}

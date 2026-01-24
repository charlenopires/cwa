//! Dashboard route handler.
//!
//! Serves the embedded task board dashboard HTML.

use axum::response::{Html, IntoResponse};

const DASHBOARD_HTML: &str = include_str!("../../../../assets/web/index.html");

/// GET / - Serve the task board dashboard.
pub async fn index() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

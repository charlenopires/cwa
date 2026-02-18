//! CWA MCP Server
//!
//! Model Context Protocol server for Claude Code integration.

pub mod planner;
pub mod planner_template;
pub mod server;

use cwa_db::{BroadcastSender, DbPool};
use std::sync::Arc;

/// Run the MCP server over stdio.
///
/// If `broadcast_tx` is provided, task updates will be broadcast directly
/// to WebSocket clients (when running alongside the web server via `cwa serve`).
pub async fn run_stdio_server(
    pool: Arc<DbPool>,
    broadcast_tx: Option<BroadcastSender>,
) -> anyhow::Result<()> {
    server::run_stdio(pool, broadcast_tx).await
}

/// Run the MCP planner server over stdio (for Claude Desktop).
///
/// `project_dir` is the directory where the command was run. If it contains a
/// valid CWA project (`.cwa/cwa.db`), it becomes the default context for
/// `cwa_plan_software` calls that omit `project_path`.
pub async fn run_planner_stdio(project_dir: &std::path::Path) -> anyhow::Result<()> {
    planner::run_planner_stdio(project_dir).await
}

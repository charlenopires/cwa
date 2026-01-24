//! CWA MCP Server
//!
//! Model Context Protocol server for Claude Code integration.

pub mod planner;
pub mod planner_template;
pub mod server;

use cwa_db::DbPool;
use std::sync::Arc;

/// Run the MCP server over stdio.
pub async fn run_stdio_server(pool: Arc<DbPool>) -> anyhow::Result<()> {
    server::run_stdio(pool).await
}

/// Run the MCP planner server over stdio (for Claude Desktop).
pub async fn run_planner_stdio() -> anyhow::Result<()> {
    planner::run_planner_stdio().await
}

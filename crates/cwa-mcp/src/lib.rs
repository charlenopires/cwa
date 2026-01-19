//! CWA MCP Server
//!
//! Model Context Protocol server for Claude Code integration.

pub mod server;

use cwa_db::DbPool;
use std::sync::Arc;

/// Run the MCP server over stdio.
pub async fn run_stdio_server(pool: Arc<DbPool>) -> anyhow::Result<()> {
    server::run_stdio(pool).await
}

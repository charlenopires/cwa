//! MCP Planner Server for Claude Desktop.
//!
//! A full-featured MCP server that exposes ALL CWA tools plus the `cwa_plan_software`
//! tool for generating DDD/SDD-based planning documents.
//! Designed to be configured in Claude Desktop's MCP settings.

use cwa_db::DbPool;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::planner_template;
use crate::server::{self, JsonRpcError};

// ============================================================
// JSON-RPC TYPES
// ============================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

// ============================================================
// SERVER LOOP
// ============================================================

/// Run the MCP planner server over stdio.
/// This is a full-featured server with all CWA tools plus the planner tool.
pub async fn run_planner_stdio() -> anyhow::Result<()> {
    // Find project directory and initialize database
    let project_dir = find_project_dir()?;
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = Arc::new(cwa_db::init_pool(&db_path)?);

    let stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut lines = stdin.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                if write_response(&mut stdout, &response).await.is_err() {
                    break;
                }
                continue;
            }
        };

        // JSON-RPC 2.0: notifications (no id) must not receive responses
        if request.id.is_none() {
            continue;
        }

        let response = handle_request(&pool, request).await;
        if write_response(&mut stdout, &response).await.is_err() {
            break;
        }
    }

    Ok(())
}

/// Find the project directory by looking for .cwa/cwa.db
fn find_project_dir() -> anyhow::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join(".cwa/cwa.db").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!("No CWA project found. Run 'cwa init' first or navigate to a CWA project directory.");
        }
    }
}

/// Write a JSON-RPC response to stdout.
async fn write_response(stdout: &mut tokio::io::Stdout, response: &JsonRpcResponse) -> std::io::Result<()> {
    let json = serde_json::to_string(response).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    stdout.write_all(format!("{}\n", json).as_bytes()).await?;
    stdout.flush().await
}

// ============================================================
// REQUEST ROUTING
// ============================================================

async fn handle_request(pool: &DbPool, request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tool_call(pool, request.params).await,
        "resources/list" => server::get_resources_list(),
        "resources/read" => server::read_resource(pool, request.params).await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found: {}", request.method),
        }),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(error),
        },
    }
}

// ============================================================
// HANDLERS
// ============================================================

fn handle_initialize() -> Result<serde_json::Value, JsonRpcError> {
    Ok(serde_json::json!({
        "protocolVersion": "2025-06-18",
        "serverInfo": {
            "name": "cwa-planner",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": {
            "tools": {
                "listChanged": true
            },
            "resources": {
                "listChanged": true
            }
        }
    }))
}

fn handle_tools_list() -> Result<serde_json::Value, JsonRpcError> {
    // Get all tools from the main server
    let mut tools = server::get_tools_list()?;

    // Add the planner tool
    let planner_tool = serde_json::json!({
        "name": "cwa_plan_software",
        "description": "Generate a software plan using Domain-Driven Design (DDD) and Specification-Driven Development (SDD) methodologies. Returns executable CWA CLI commands covering: Strategic Design (bounded contexts, subdomains), Ubiquitous Language (domain glossary), Architectural Decisions (ADRs), and Specifications (source of truth with acceptance criteria). The AI asks clarifying questions before generating the plan.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Description of the desired software, feature, or system to plan"
                },
                "project_path": {
                    "type": "string",
                    "description": "Optional absolute path to an existing CWA project directory (contains .cwa/cwa.db). When provided, reads current project state and generates a continuation plan that integrates with existing specs, contexts, and tasks."
                }
            },
            "required": ["prompt"]
        }
    });

    // Insert planner tool at the beginning
    if let Some(tools_array) = tools["tools"].as_array_mut() {
        tools_array.insert(0, planner_tool);
    }

    Ok(tools)
}

async fn handle_tool_call(
    pool: &DbPool,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let params = params.ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Missing params".to_string(),
    })?;

    let name = params["name"]
        .as_str()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing tool name".to_string(),
        })?;

    // Handle planner tool specially
    if name == "cwa_plan_software" {
        let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
        return plan_software(&args).await;
    }

    // Delegate all other tools to the main server
    // No broadcast channel for planner (uses HTTP fallback)
    server::call_tool(pool, &None, Some(params)).await
}

// ============================================================
// PLANNER TOOL IMPLEMENTATION
// ============================================================

async fn plan_software(
    args: &serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError> {
    let prompt = args["prompt"]
        .as_str()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing required parameter: prompt".to_string(),
        })?;

    // Optionally read existing project state
    let existing_state = if let Some(path_str) = args.get("project_path").and_then(|v| v.as_str()) {
        let path = Path::new(path_str);
        match planner_template::read_existing_state(path) {
            Ok(state) => Some(state),
            Err(e) => {
                // Include the error as a note but don't fail
                return Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!(
                            "Note: Could not read existing project at '{}': {}\n\n{}",
                            path_str,
                            e,
                            planner_template::render_planning_document(prompt, None)
                        )
                    }]
                }));
            }
        }
    } else {
        None
    };

    let document = planner_template::render_planning_document(prompt, existing_state);

    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": document
        }]
    }))
}

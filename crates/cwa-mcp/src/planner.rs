//! MCP Planner Server for Claude Desktop.
//!
//! A specialized MCP server that exposes a single tool (`cwa_plan_software`)
//! for generating structured planning documents from user prompts.
//! Designed to be configured in Claude Desktop's MCP settings.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::planner_template;

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

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

// ============================================================
// SERVER LOOP
// ============================================================

/// Run the MCP planner server over stdio.
pub async fn run_planner_stdio() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => break,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };
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
                if write_response(&mut stdout, &response).is_err() {
                    break;
                }
                continue;
            }
        };

        let response = handle_request(request).await;
        if write_response(&mut stdout, &response).is_err() {
            break;
        }
    }

    Ok(())
}

/// Write a JSON-RPC response to stdout, returning Err on broken pipe.
fn write_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> io::Result<()> {
    let json = serde_json::to_string(response).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    match writeln!(stdout, "{}", json) {
        Ok(()) => stdout.flush(),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Err(e),
        Err(e) => Err(e),
    }
}

// ============================================================
// REQUEST ROUTING
// ============================================================

async fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(),
        "notifications/initialized" => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({})),
                error: None,
            };
        }
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tool_call(request.params).await,
        "resources/list" => Ok(serde_json::json!({ "resources": [] })),
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
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "cwa-planner",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": {
            "tools": {}
        }
    }))
}

fn handle_tools_list() -> Result<serde_json::Value, JsonRpcError> {
    let tools = vec![
        Tool {
            name: "cwa_plan_software".to_string(),
            description: "Generate a structured software planning document. Returns markdown with DDD bounded contexts, specifications with acceptance criteria, domain model, task breakdown, and CWA CLI bootstrap commands. Instructs the AI to ask clarifying questions before generating any code.".to_string(),
            input_schema: serde_json::json!({
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
            }),
        },
    ];

    Ok(serde_json::json!({ "tools": tools }))
}

async fn handle_tool_call(
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

    let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

    match name {
        "cwa_plan_software" => plan_software(&args).await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Unknown tool: {}", name),
        }),
    }
}

// ============================================================
// TOOL IMPLEMENTATION
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

//! MCP Server implementation.
//!
//! Note: This is a simplified implementation. Full rmcp integration
//! would require the actual rmcp crate which may have different APIs.

use cwa_db::DbPool;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::sync::Arc;

/// JSON-RPC request structure.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

/// JSON-RPC response structure.
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

/// Tool definition.
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

/// Resource definition.
#[derive(Debug, Serialize)]
struct Resource {
    uri: String,
    name: String,
    description: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
}

/// Run the MCP server over stdio.
pub async fn run_stdio(pool: Arc<DbPool>) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
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
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = handle_request(&pool, request).await;
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

async fn handle_request(pool: &DbPool, request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tool_call(pool, request.params).await,
        "resources/list" => handle_resources_list(),
        "resources/read" => handle_resource_read(pool, request.params).await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found: {}", request.method),
        }),
    };

    match result {
        Ok(r) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(r),
            error: None,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(e),
        },
    }
}

fn handle_initialize() -> Result<serde_json::Value, JsonRpcError> {
    Ok(serde_json::json!({
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "cwa",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": {
            "tools": {},
            "resources": {}
        }
    }))
}

fn handle_tools_list() -> Result<serde_json::Value, JsonRpcError> {
    let tools = vec![
        Tool {
            name: "cwa_get_current_task".to_string(),
            description: "Get the current in-progress task".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_get_spec".to_string(),
            description: "Get a specification by ID or name".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "Spec ID or title"
                    }
                },
                "required": ["identifier"]
            }),
        },
        Tool {
            name: "cwa_get_context_summary".to_string(),
            description: "Get a compact summary of the project context".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_get_domain_model".to_string(),
            description: "Get the domain model with bounded contexts".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_update_task_status".to_string(),
            description: "Update a task's status".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID"
                    },
                    "status": {
                        "type": "string",
                        "description": "New status (backlog, todo, in_progress, review, done)"
                    }
                },
                "required": ["task_id", "status"]
            }),
        },
        Tool {
            name: "cwa_add_decision".to_string(),
            description: "Register an architectural decision (ADR)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Decision title"
                    },
                    "context": {
                        "type": "string",
                        "description": "Problem context"
                    },
                    "decision": {
                        "type": "string",
                        "description": "The decision made"
                    }
                },
                "required": ["title", "context", "decision"]
            }),
        },
        Tool {
            name: "cwa_get_next_steps".to_string(),
            description: "Get suggested next steps based on current state".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_search_memory".to_string(),
            description: "Search project memory".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        },
    ];

    Ok(serde_json::json!({ "tools": tools }))
}

async fn handle_tool_call(
    pool: &DbPool,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let params = params.ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Missing params".to_string(),
    })?;

    let name = params["name"].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Missing tool name".to_string(),
    })?;

    let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

    // Get default project
    let project = cwa_core::project::get_default_project(pool)
        .map_err(|e| JsonRpcError {
            code: -32603,
            message: e.to_string(),
        })?
        .ok_or_else(|| JsonRpcError {
            code: -32603,
            message: "No project found".to_string(),
        })?;

    let result = match name {
        "cwa_get_current_task" => {
            let task = cwa_core::task::get_current_task(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            match task {
                Some(t) => serde_json::to_value(&t).unwrap(),
                None => serde_json::json!({ "message": "No task currently in progress" }),
            }
        }

        "cwa_get_spec" => {
            let identifier = args["identifier"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing identifier".to_string(),
            })?;

            let spec = cwa_core::spec::get_spec(pool, &project.id, identifier)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_value(&spec).unwrap()
        }

        "cwa_get_context_summary" => {
            let summary = cwa_core::memory::get_context_summary(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "text": summary.to_compact_string()
            })
        }

        "cwa_get_domain_model" => {
            let model = cwa_core::domain::get_domain_model(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_value(&model).unwrap()
        }

        "cwa_update_task_status" => {
            let task_id = args["task_id"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing task_id".to_string(),
            })?;
            let status = args["status"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing status".to_string(),
            })?;

            cwa_core::task::move_task(pool, &project.id, task_id, status)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "message": format!("Task {} moved to {}", task_id, status)
            })
        }

        "cwa_add_decision" => {
            let title = args["title"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing title".to_string(),
            })?;
            let context = args["context"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing context".to_string(),
            })?;
            let decision = args["decision"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing decision".to_string(),
            })?;

            let adr = cwa_core::decision::create_decision(pool, &project.id, title, context, decision)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "id": adr.id,
                "message": format!("Decision recorded: {}", title)
            })
        }

        "cwa_get_next_steps" => {
            let steps = cwa_core::memory::suggest_next_steps(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "steps": steps
            })
        }

        "cwa_search_memory" => {
            let query = args["query"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing query".to_string(),
            })?;

            let results = cwa_core::memory::search_memory(pool, &project.id, query)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_value(&results).unwrap()
        }

        _ => {
            return Err(JsonRpcError {
                code: -32601,
                message: format!("Unknown tool: {}", name),
            })
        }
    };

    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&result).unwrap()
        }]
    }))
}

fn handle_resources_list() -> Result<serde_json::Value, JsonRpcError> {
    let resources = vec![
        Resource {
            uri: "project://constitution".to_string(),
            name: "Project Constitution".to_string(),
            description: "Core project values and constraints".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        Resource {
            uri: "project://current-spec".to_string(),
            name: "Current Specification".to_string(),
            description: "Active specification being worked on".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://domain-model".to_string(),
            name: "Domain Model".to_string(),
            description: "DDD bounded contexts and objects".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://kanban-board".to_string(),
            name: "Kanban Board".to_string(),
            description: "Current task board state".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://decisions".to_string(),
            name: "Architectural Decisions".to_string(),
            description: "ADR log".to_string(),
            mime_type: "application/json".to_string(),
        },
    ];

    Ok(serde_json::json!({ "resources": resources }))
}

async fn handle_resource_read(
    pool: &DbPool,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let params = params.ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Missing params".to_string(),
    })?;

    let uri = params["uri"].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Missing uri".to_string(),
    })?;

    let project = cwa_core::project::get_default_project(pool)
        .map_err(|e| JsonRpcError {
            code: -32603,
            message: e.to_string(),
        })?
        .ok_or_else(|| JsonRpcError {
            code: -32603,
            message: "No project found".to_string(),
        })?;

    let content = match uri {
        "project://constitution" => {
            cwa_core::project::get_constitution(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?
        }

        "project://current-spec" => {
            let spec = cwa_core::spec::get_active_spec(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            match spec {
                Some(s) => serde_json::to_string_pretty(&s).unwrap(),
                None => "No active specification".to_string(),
            }
        }

        "project://domain-model" => {
            let model = cwa_core::domain::get_domain_model(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_string_pretty(&model).unwrap()
        }

        "project://kanban-board" => {
            let board = cwa_core::task::get_board(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_string_pretty(&board).unwrap()
        }

        "project://decisions" => {
            let decisions = cwa_core::decision::list_decisions(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_string_pretty(&decisions).unwrap()
        }

        _ => {
            return Err(JsonRpcError {
                code: -32602,
                message: format!("Unknown resource: {}", uri),
            })
        }
    };

    Ok(serde_json::json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/plain",
            "text": content
        }]
    }))
}

//! MCP Server implementation.
//!
//! Note: This is a simplified implementation. Full rmcp integration
//! would require the actual rmcp crate which may have different APIs.

use cwa_db::{BroadcastSender, DbPool, WebSocketMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

#[derive(Debug, Serialize, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
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
///
/// If `broadcast_tx` is provided, task updates will be broadcast directly
/// to WebSocket clients (when running alongside the web server).
pub async fn run_stdio(
    pool: Arc<DbPool>,
    broadcast_tx: Option<BroadcastSender>,
) -> anyhow::Result<()> {
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
                let output = format!("{}\n", serde_json::to_string(&response)?);
                stdout.write_all(output.as_bytes()).await?;
                stdout.flush().await?;
                continue;
            }
        };

        // JSON-RPC 2.0: notifications (no id) must not receive responses
        if request.id.is_none() {
            continue;
        }

        let response = handle_request(&pool, &broadcast_tx, request).await;
        let output = format!("{}\n", serde_json::to_string(&response)?);
        stdout.write_all(output.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(
    pool: &DbPool,
    broadcast_tx: &Option<BroadcastSender>,
    request: JsonRpcRequest,
) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(),
        "tools/list" => get_tools_list(),
        "tools/call" => {
            // Per MCP spec, tool execution failures should be returned as
            // successful JSON-RPC responses with isError: true, not as
            // JSON-RPC errors. Only protocol errors (-32602, -32601) stay
            // as JSON-RPC errors.
            match call_tool(pool, broadcast_tx, request.params).await {
                Ok(v) => Ok(v),
                Err(e) if e.code == -32603 => Ok(tool_error(&e.message)),
                Err(e) => Err(e),
            }
        }
        "resources/list" => get_resources_list(),
        "resources/read" => read_resource(pool, request.params).await,
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
        "protocolVersion": "2025-06-18",
        "serverInfo": {
            "name": "cwa",
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

/// Get the list of all available tools (for reuse by planner).
pub fn get_tools_list() -> Result<serde_json::Value, JsonRpcError> {
    let tools = vec![
        Tool {
            name: "cwa_get_project_info".to_string(),
            description: "Get project metadata including name, description, tech stack, features, and constraints. Run 'cwa update' to modify this information.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
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
        // Graph tools
        Tool {
            name: "cwa_graph_query".to_string(),
            description: "Execute a Cypher query against the Knowledge Graph".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cypher": {
                        "type": "string",
                        "description": "Cypher query to execute"
                    }
                },
                "required": ["cypher"]
            }),
        },
        Tool {
            name: "cwa_graph_impact".to_string(),
            description: "Analyze the impact of changes to an entity in the Knowledge Graph".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_type": {
                        "type": "string",
                        "description": "Entity type (spec, task, context, decision)"
                    },
                    "entity_id": {
                        "type": "string",
                        "description": "Entity ID"
                    }
                },
                "required": ["entity_type", "entity_id"]
            }),
        },
        Tool {
            name: "cwa_graph_sync".to_string(),
            description: "Sync SQLite entities to Neo4j Knowledge Graph".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        // Embedding tools
        Tool {
            name: "cwa_memory_semantic_search".to_string(),
            description: "Search memories using vector similarity (semantic search)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of results (default: 5)"
                    }
                },
                "required": ["query"]
            }),
        },
        Tool {
            name: "cwa_generate_tasks".to_string(),
            description: "Generate tasks from a spec's acceptance criteria".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "spec_identifier": {
                        "type": "string",
                        "description": "Spec ID or title"
                    },
                    "status": {
                        "type": "string",
                        "description": "Initial task status (default: backlog)"
                    }
                },
                "required": ["spec_identifier"]
            }),
        },
        Tool {
            name: "cwa_memory_add".to_string(),
            description: "Store a memory with vector embedding for future semantic search".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Memory content to store"
                    },
                    "entry_type": {
                        "type": "string",
                        "description": "Type: preference, decision, fact, pattern"
                    },
                    "context": {
                        "type": "string",
                        "description": "Optional context for the memory"
                    }
                },
                "required": ["content", "entry_type"]
            }),
        },
        // Observation tools (progressive disclosure)
        Tool {
            name: "cwa_observe".to_string(),
            description: "Record a structured observation about development activity. Use this to capture bugfixes, features, discoveries, decisions, changes, and insights.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Brief title of the observation"
                    },
                    "obs_type": {
                        "type": "string",
                        "description": "Type: bugfix, feature, refactor, discovery, decision, change, insight"
                    },
                    "narrative": {
                        "type": "string",
                        "description": "Detailed narrative explanation"
                    },
                    "facts": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Specific facts learned"
                    },
                    "concepts": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Concept categories: how-it-works, why-it-exists, what-changed, problem-solution, gotcha, pattern, trade-off"
                    },
                    "files_modified": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Files that were modified"
                    }
                },
                "required": ["title", "obs_type"]
            }),
        },
        Tool {
            name: "cwa_memory_timeline".to_string(),
            description: "Get a compact timeline of recent observations. Returns index-only data (~50 tokens per entry). Use cwa_memory_get for full details.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "days_back": {
                        "type": "integer",
                        "description": "Number of days back to look (default: 7)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max entries to return (default: 20)"
                    }
                }
            }),
        },
        Tool {
            name: "cwa_memory_get".to_string(),
            description: "Get full details of specific observations by ID. Returns complete data (~500 tokens per entry). Boosts confidence of accessed items.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Observation IDs to retrieve"
                    }
                },
                "required": ["ids"]
            }),
        },
        // Creation tools
        Tool {
            name: "cwa_create_context".to_string(),
            description: "Create a new bounded context (DDD)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Bounded context name"
                    },
                    "description": {
                        "type": "string",
                        "description": "Context description"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "cwa_create_spec".to_string(),
            description: "Create a new specification with optional acceptance criteria".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Specification title"
                    },
                    "description": {
                        "type": "string",
                        "description": "Specification description"
                    },
                    "priority": {
                        "type": "string",
                        "description": "Priority: low, medium, high, critical (default: medium)"
                    },
                    "criteria": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Acceptance criteria"
                    }
                },
                "required": ["title"]
            }),
        },
        Tool {
            name: "cwa_create_task".to_string(),
            description: "Create a new task".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Task title"
                    },
                    "description": {
                        "type": "string",
                        "description": "Task description"
                    },
                    "priority": {
                        "type": "string",
                        "description": "Priority: low, medium, high, critical (default: medium)"
                    },
                    "spec_id": {
                        "type": "string",
                        "description": "Optional spec ID to associate with"
                    }
                },
                "required": ["title"]
            }),
        },
        Tool {
            name: "cwa_memory_search_all".to_string(),
            description: "Search across both memories and observations using semantic similarity. Returns compact index (~50 tokens per result). Use cwa_memory_get for full details.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of results (default: 10)"
                    }
                },
                "required": ["query"]
            }),
        },
        // New listing tools
        Tool {
            name: "cwa_list_specs".to_string(),
            description: "List all specifications for the project".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "description": "Filter by status: draft, active, completed, archived (optional)"
                    }
                }
            }),
        },
        Tool {
            name: "cwa_list_tasks".to_string(),
            description: "List all tasks for the project".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "description": "Filter by status: backlog, todo, in_progress, review, done (optional)"
                    },
                    "spec_id": {
                        "type": "string",
                        "description": "Filter by spec ID (optional)"
                    }
                }
            }),
        },
        Tool {
            name: "cwa_update_spec_status".to_string(),
            description: "Update a specification's status".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "spec_id": {
                        "type": "string",
                        "description": "Spec ID or title"
                    },
                    "status": {
                        "type": "string",
                        "description": "New status: draft, active, completed, archived"
                    }
                },
                "required": ["spec_id", "status"]
            }),
        },
        Tool {
            name: "cwa_get_glossary".to_string(),
            description: "Get the domain glossary with term definitions".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_add_glossary_term".to_string(),
            description: "Add a term to the domain glossary".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "term": {
                        "type": "string",
                        "description": "The term to define"
                    },
                    "definition": {
                        "type": "string",
                        "description": "The definition of the term"
                    },
                    "context_id": {
                        "type": "string",
                        "description": "Optional bounded context ID to associate with"
                    }
                },
                "required": ["term", "definition"]
            }),
        },
        Tool {
            name: "cwa_get_wip_status".to_string(),
            description: "Get WIP (Work In Progress) limits status for all Kanban columns".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_set_wip_limit".to_string(),
            description: "Set WIP limit for a Kanban column".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "column": {
                        "type": "string",
                        "description": "Column name: backlog, todo, in_progress, review, done"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "WIP limit (null to remove limit)"
                    }
                },
                "required": ["column"]
            }),
        },
        Tool {
            name: "cwa_validate_spec".to_string(),
            description: "Validate a specification for completeness".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "spec_id": {
                        "type": "string",
                        "description": "Spec ID or title"
                    }
                },
                "required": ["spec_id"]
            }),
        },
        Tool {
            name: "cwa_add_acceptance_criteria".to_string(),
            description: "Add acceptance criteria to an existing specification".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "spec_id": {
                        "type": "string",
                        "description": "Spec ID or title"
                    },
                    "criteria": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Acceptance criteria to add"
                    }
                },
                "required": ["spec_id", "criteria"]
            }),
        },
        Tool {
            name: "cwa_get_context_map".to_string(),
            description: "Get the DDD context map showing relationships between bounded contexts".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "cwa_create_domain_object".to_string(),
            description: "Create a domain object (entity, value object, aggregate, service, event) within a bounded context".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "context_id": {
                        "type": "string",
                        "description": "Bounded context ID"
                    },
                    "name": {
                        "type": "string",
                        "description": "Object name"
                    },
                    "object_type": {
                        "type": "string",
                        "description": "Type: entity, value_object, aggregate, service, event"
                    },
                    "description": {
                        "type": "string",
                        "description": "Object description (optional)"
                    }
                },
                "required": ["context_id", "name", "object_type"]
            }),
        },
        Tool {
            name: "cwa_list_decisions".to_string(),
            description: "List all architectural decisions (ADRs) for the project".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ];

    Ok(serde_json::json!({ "tools": tools }))
}

/// Build a tool error response with `isError: true` per MCP spec.
/// Used for execution failures (DB, embedding, graph) — not protocol errors.
fn tool_error(message: &str) -> serde_json::Value {
    serde_json::json!({
        "content": [{"type": "text", "text": format!("Error: {}", message)}],
        "isError": true
    })
}

/// Call a tool by name (for reuse by planner).
///
/// Execution errors (DB, embedding, graph failures) are returned as successful
/// JSON-RPC responses with `isError: true` per the MCP spec.
/// Protocol errors (missing params, unknown tool) remain as JSON-RPC errors.
pub async fn call_tool(
    pool: &DbPool,
    broadcast_tx: &Option<BroadcastSender>,
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

    // Get default project — execution failure returns isError
    let project = match cwa_core::project::get_default_project(pool) {
        Ok(Some(p)) => p,
        Ok(None) => return Ok(tool_error("No project found. Run 'cwa init' first.")),
        Err(e) => return Ok(tool_error(&format!("Database error: {}", e))),
    };

    let result = match name {
        "cwa_get_project_info" => {
            let info = cwa_core::project::get_project_info(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            match info {
                Some(i) => serde_json::json!({
                    "name": i.name,
                    "description": i.description,
                    "tech_stack": i.tech_stack,
                    "main_features": i.main_features,
                    "constraints": i.constraints,
                    "updated_at": i.updated_at,
                    "hint": "Run 'cwa update' to modify project information"
                }),
                None => serde_json::json!({
                    "name": project.name,
                    "description": project.description,
                    "message": "No extended project info configured. Run 'cwa update' to add metadata.",
                    "hint": "Run 'cwa update' to add tech stack, features, and constraints"
                }),
            }
        }

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

            // Broadcast to WebSocket clients (direct if available, HTTP fallback)
            if let Some(tx) = broadcast_tx {
                let _ = tx.send(WebSocketMessage::TaskUpdated {
                    task_id: task_id.to_string(),
                    status: status.to_string(),
                });
            } else {
                // Fallback to HTTP notification when running standalone
                // Use cwa_core notifier and await completion (no fire-and-forget)
                let notifier = cwa_core::WebNotifier::new();
                notifier.notify_task_updated(task_id, status).await;
            }

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

        "cwa_generate_tasks" => {
            let spec_identifier = args["spec_identifier"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing spec_identifier".to_string(),
            })?;
            let status = args.get("status").and_then(|v| v.as_str()).unwrap_or("backlog");

            let result = cwa_core::task::generate_tasks_from_spec(pool, &project.id, spec_identifier, status)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "created": result.created.len(),
                "skipped": result.skipped,
                "tasks": result.created.iter().map(|t| serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status.as_str()
                })).collect::<Vec<_>>()
            })
        }

        // Graph tools
        "cwa_graph_query" => {
            let cypher = args["cypher"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing cypher".to_string(),
            })?;

            let client = cwa_graph::GraphClient::connect_default().await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Neo4j connection failed: {}", e),
                })?;

            let results = cwa_graph::queries::search::raw_query(&client, cypher).await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({ "results": results })
        }

        "cwa_graph_impact" => {
            let entity_type = args["entity_type"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing entity_type".to_string(),
            })?;
            let entity_id = args["entity_id"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing entity_id".to_string(),
            })?;

            let client = cwa_graph::GraphClient::connect_default().await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Neo4j connection failed: {}", e),
                })?;

            let nodes = cwa_graph::queries::impact::impact_analysis(&client, entity_type, entity_id, 3).await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({ "impacts": nodes })
        }

        "cwa_graph_sync" => {
            let client = cwa_graph::GraphClient::connect_default().await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Neo4j connection failed: {}", e),
                })?;

            cwa_graph::schema::initialize_schema(&client).await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            let result = cwa_graph::run_full_sync(&client, pool, &project.id).await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "nodes_created": result.nodes_created,
                "nodes_updated": result.nodes_updated,
                "relationships_created": result.relationships_created
            })
        }

        // Embedding tools
        "cwa_memory_semantic_search" => {
            let query = args["query"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing query".to_string(),
            })?;
            let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5);

            let search = cwa_embedding::SemanticSearch::default_search()
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Search initialization failed: {}", e),
                })?;

            let results = search.search_project(query, &project.id, top_k).await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({ "results": results })
        }

        "cwa_observe" => {
            let title = args["title"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing title".to_string(),
            })?;
            let obs_type = args["obs_type"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing obs_type".to_string(),
            })?;
            let narrative = args.get("narrative").and_then(|v| v.as_str());
            let facts: Vec<String> = args.get("facts")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let concepts: Vec<String> = args.get("concepts")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let files_modified: Vec<String> = args.get("files_modified")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            // Try with embedding pipeline, fallback to DB-only
            match cwa_embedding::ObservationPipeline::default_pipeline() {
                Ok(pipeline) => {
                    let result = pipeline.add_observation(
                        pool, &project.id, obs_type, title, narrative,
                        &facts, &concepts, &files_modified, &[],
                        None, 0.8,
                    ).await.map_err(|e| JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                    })?;

                    serde_json::json!({
                        "success": true,
                        "id": result.id,
                        "embedding_dim": result.embedding_dim
                    })
                }
                Err(_) => {
                    let obs = cwa_core::memory::add_observation(
                        pool, &project.id, obs_type, title, narrative,
                        &facts, &concepts, &files_modified, &[],
                        None, 0.8,
                    ).map_err(|e| JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                    })?;

                    serde_json::json!({
                        "success": true,
                        "id": obs.id,
                        "embedding_dim": 0
                    })
                }
            }
        }

        "cwa_memory_timeline" => {
            let days_back = args.get("days_back").and_then(|v| v.as_i64()).unwrap_or(7);
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);

            let timeline = cwa_core::memory::get_timeline(pool, &project.id, days_back, limit)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({ "observations": timeline })
        }

        "cwa_memory_get" => {
            let ids: Vec<String> = args["ids"].as_array()
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing ids array".to_string(),
                })?
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
            let observations = cwa_core::memory::get_observations_batch(pool, &id_refs)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            // Boost confidence for accessed items
            for obs in &observations {
                let _ = cwa_core::memory::boost_confidence(pool, &obs.id, 0.05);
            }

            serde_json::json!({ "observations": observations })
        }

        "cwa_memory_search_all" => {
            let query = args["query"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing query".to_string(),
            })?;
            let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(10);

            // Try semantic search, fallback to timeline
            match cwa_embedding::SemanticSearch::default_search() {
                Ok(search) => {
                    let results = search.search_all(query, &project.id, top_k).await
                        .map_err(|e| JsonRpcError {
                            code: -32603,
                            message: e.to_string(),
                        })?;

                    serde_json::json!({ "results": results })
                }
                Err(_) => {
                    // Fallback to text-based search
                    let memories = cwa_core::memory::search_memory(pool, &project.id, query)
                        .map_err(|e| JsonRpcError {
                            code: -32603,
                            message: e.to_string(),
                        })?;

                    serde_json::json!({ "results": memories })
                }
            }
        }

        "cwa_memory_add" => {
            let content = args["content"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing content".to_string(),
            })?;
            let entry_type_str = args["entry_type"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing entry_type".to_string(),
            })?;
            let context = args.get("context").and_then(|v| v.as_str());

            let entry_type = cwa_embedding::MemoryType::from_str(entry_type_str)
                .map_err(|e| JsonRpcError {
                    code: -32602,
                    message: e.to_string(),
                })?;

            let pipeline = cwa_embedding::MemoryPipeline::default_pipeline()
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Pipeline initialization failed: {}", e),
                })?;

            let result = pipeline.add_memory(pool, &project.id, content, entry_type, context).await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "id": result.id,
                "embedding_dim": result.embedding_dim
            })
        }

        // Listing tools
        "cwa_list_specs" => {
            let specs = cwa_core::spec::list_specs(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            let status_filter = args.get("status").and_then(|v| v.as_str());
            let filtered: Vec<_> = specs
                .into_iter()
                .filter(|s| status_filter.map_or(true, |f| s.status.as_str() == f))
                .collect();

            serde_json::json!({
                "count": filtered.len(),
                "specs": filtered.iter().map(|s| serde_json::json!({
                    "id": s.id,
                    "title": s.title,
                    "status": s.status.as_str(),
                    "priority": s.priority.as_str(),
                    "criteria_count": s.acceptance_criteria.len()
                })).collect::<Vec<_>>()
            })
        }

        "cwa_list_tasks" => {
            let status_filter = args.get("status").and_then(|v| v.as_str());
            let spec_filter = args.get("spec_id").and_then(|v| v.as_str());

            let tasks = if let Some(spec_id) = spec_filter {
                let spec = cwa_core::spec::get_spec(pool, &project.id, spec_id)
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                    })?;
                cwa_core::task::list_tasks_by_spec(pool, &spec.id)
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                    })?
            } else {
                cwa_core::task::list_tasks(pool, &project.id)
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                    })?
            };

            let filtered: Vec<_> = tasks
                .into_iter()
                .filter(|t| status_filter.map_or(true, |f| t.status.as_str() == f))
                .collect();

            serde_json::json!({
                "count": filtered.len(),
                "tasks": filtered.iter().map(|t| serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status.as_str(),
                    "priority": t.priority.as_str(),
                    "spec_id": t.spec_id
                })).collect::<Vec<_>>()
            })
        }

        "cwa_update_spec_status" => {
            let spec_id = args["spec_id"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing spec_id".to_string(),
            })?;
            let status = args["status"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing status".to_string(),
            })?;

            let spec = cwa_core::spec::get_spec(pool, &project.id, spec_id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            cwa_core::spec::update_status(pool, &spec.id, status)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "message": format!("Spec '{}' status updated to {}", spec.title, status)
            })
        }

        "cwa_get_glossary" => {
            let terms = cwa_core::domain::list_glossary(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "count": terms.len(),
                "terms": terms
            })
        }

        "cwa_add_glossary_term" => {
            let term = args["term"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing term".to_string(),
            })?;
            let definition = args["definition"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing definition".to_string(),
            })?;
            let context_id = args.get("context_id").and_then(|v| v.as_str());

            cwa_core::domain::add_glossary_term(pool, &project.id, term, definition, context_id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "message": format!("Glossary term '{}' added", term)
            })
        }

        "cwa_get_wip_status" => {
            let wip_status = cwa_core::task::get_wip_status(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_value(&wip_status).unwrap()
        }

        "cwa_set_wip_limit" => {
            let column = args["column"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing column".to_string(),
            })?;
            let limit = args.get("limit").and_then(|v| v.as_i64());

            cwa_core::task::set_wip_limit(pool, &project.id, column, limit)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            let msg = match limit {
                Some(l) => format!("WIP limit for '{}' set to {}", column, l),
                None => format!("WIP limit for '{}' removed", column),
            };

            serde_json::json!({
                "success": true,
                "message": msg
            })
        }

        "cwa_validate_spec" => {
            let spec_id = args["spec_id"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing spec_id".to_string(),
            })?;

            let spec = cwa_core::spec::get_spec(pool, &project.id, spec_id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            let result = cwa_core::spec::validate_spec(pool, &spec.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_value(&result).unwrap()
        }

        "cwa_add_acceptance_criteria" => {
            let spec_id = args["spec_id"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing spec_id".to_string(),
            })?;
            let criteria: Vec<String> = args["criteria"].as_array()
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing criteria array".to_string(),
                })?
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            let spec = cwa_core::spec::add_acceptance_criteria(pool, &project.id, spec_id, &criteria)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "spec_id": spec.id,
                "title": spec.title,
                "total_criteria": spec.acceptance_criteria.len()
            })
        }

        "cwa_get_context_map" => {
            let context_map = cwa_core::domain::get_context_map(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_value(&context_map).unwrap()
        }

        "cwa_create_domain_object" => {
            let context_id = args["context_id"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing context_id".to_string(),
            })?;
            let name = args["name"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing name".to_string(),
            })?;
            let object_type = args["object_type"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing object_type".to_string(),
            })?;
            let description = args.get("description").and_then(|v| v.as_str());

            cwa_core::domain::create_domain_object(pool, context_id, name, object_type, description)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "message": format!("Domain object '{}' ({}) created", name, object_type)
            })
        }

        "cwa_list_decisions" => {
            let decisions = cwa_core::decision::list_decisions(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "count": decisions.len(),
                "decisions": decisions
            })
        }

        // Creation tools
        "cwa_create_context" => {
            let name = args["name"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing name".to_string(),
            })?;
            let description = args.get("description").and_then(|v| v.as_str());

            let ctx = cwa_core::domain::create_context(pool, &project.id, name, description)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "id": ctx.id,
                "name": ctx.name
            })
        }

        "cwa_create_spec" => {
            let title = args["title"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing title".to_string(),
            })?;
            let description = args.get("description").and_then(|v| v.as_str());
            let priority = args.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");
            let criteria: Option<Vec<String>> = args.get("criteria").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter().filter_map(|item| item.as_str().map(String::from)).collect()
                })
            });

            let spec = cwa_core::spec::create_spec_with_criteria(
                pool,
                &project.id,
                title,
                description,
                priority,
                criteria.as_deref(),
            )
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "id": spec.id,
                "title": spec.title,
                "criteria_count": spec.acceptance_criteria.len()
            })
        }

        "cwa_create_task" => {
            let title = args["title"].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing title".to_string(),
            })?;
            let description = args.get("description").and_then(|v| v.as_str());
            let priority = args.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");
            let spec_id = args.get("spec_id").and_then(|v| v.as_str());

            let task = cwa_core::task::create_task(pool, &project.id, title, description, spec_id, priority)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::json!({
                "success": true,
                "id": task.id,
                "title": task.title
            })
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

/// Get the list of all available resources (for reuse by planner).
pub fn get_resources_list() -> Result<serde_json::Value, JsonRpcError> {
    let resources = vec![
        Resource {
            uri: "project://info".to_string(),
            name: "Project Information".to_string(),
            description: "Project metadata including tech stack, features, and constraints. Run 'cwa update' to modify.".to_string(),
            mime_type: "text/markdown".to_string(),
        },
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
        // New resources
        Resource {
            uri: "project://specs".to_string(),
            name: "All Specifications".to_string(),
            description: "List of all project specifications with status and criteria count".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://tasks".to_string(),
            name: "All Tasks".to_string(),
            description: "List of all tasks with current status".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://glossary".to_string(),
            name: "Domain Glossary".to_string(),
            description: "Ubiquitous language terms and definitions".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://wip-status".to_string(),
            name: "WIP Status".to_string(),
            description: "Work-in-progress limits and current counts per column".to_string(),
            mime_type: "application/json".to_string(),
        },
        Resource {
            uri: "project://context-map".to_string(),
            name: "Context Map".to_string(),
            description: "DDD context map showing relationships between bounded contexts".to_string(),
            mime_type: "application/json".to_string(),
        },
    ];

    Ok(serde_json::json!({ "resources": resources }))
}

/// Read a resource by URI (for reuse by planner).
pub async fn read_resource(
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
        "project://info" => {
            let info = cwa_core::project::get_project_info(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            match info {
                Some(i) => i.to_markdown(),
                None => format!(
                    "# {}\n\n{}\n\n> Run `cwa update` to add project metadata (tech stack, features, constraints).",
                    project.name,
                    project.description.as_deref().unwrap_or("")
                ),
            }
        }

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

        "project://specs" => {
            let specs = cwa_core::spec::list_specs(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            let summary: Vec<_> = specs.iter().map(|s| serde_json::json!({
                "id": s.id,
                "title": s.title,
                "status": s.status.as_str(),
                "priority": s.priority.as_str(),
                "criteria_count": s.acceptance_criteria.len()
            })).collect();

            serde_json::to_string_pretty(&serde_json::json!({
                "count": specs.len(),
                "specs": summary
            })).unwrap()
        }

        "project://tasks" => {
            let tasks = cwa_core::task::list_tasks(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            let summary: Vec<_> = tasks.iter().map(|t| serde_json::json!({
                "id": t.id,
                "title": t.title,
                "status": t.status.as_str(),
                "priority": t.priority.as_str(),
                "spec_id": t.spec_id
            })).collect();

            serde_json::to_string_pretty(&serde_json::json!({
                "count": tasks.len(),
                "tasks": summary
            })).unwrap()
        }

        "project://glossary" => {
            let terms = cwa_core::domain::list_glossary(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_string_pretty(&serde_json::json!({
                "count": terms.len(),
                "terms": terms
            })).unwrap()
        }

        "project://wip-status" => {
            let wip_status = cwa_core::task::get_wip_status(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_string_pretty(&wip_status).unwrap()
        }

        "project://context-map" => {
            let context_map = cwa_core::domain::get_context_map(pool, &project.id)
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                })?;

            serde_json::to_string_pretty(&context_map).unwrap()
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

//! MCP Planner Server for Claude Desktop.
//!
//! A focused MCP server that exposes ONLY the `cwa_plan_software` tool
//! for generating DDD/SDD-based planning documents.
//! Designed to be configured in Claude Desktop's MCP settings.

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::planner_template;
use crate::server::JsonRpcError;

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

fn print_startup_banner(project: Option<&Path>) {
    if std::io::stderr().is_terminal() {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!();
        eprintln!(
            "\x1b[1;36m  ◆  cwa-planner\x1b[0m  \x1b[1;33mv{version}\x1b[0m"
        );
        eprintln!("\x1b[90m     MCP Server · JSON-RPC 2.0 · stdio\x1b[0m");
        eprintln!("\x1b[90m     Tools (1):\x1b[0m");
        eprintln!("\x1b[90m       ◦ \x1b[0mcwa_plan_software");
        eprintln!("\x1b[90m     Resources:  none\x1b[0m");
        if let Some(p) = project {
            eprintln!(
                "\x1b[90m     Project  \x1b[0m\x1b[32m{}\x1b[0m",
                p.display()
            );
        }
        eprintln!(
            "\x1b[90m     Listening on stdin · press \x1b[0m\x1b[1mCtrl+C\x1b[0m\x1b[90m to stop\x1b[0m"
        );
        eprintln!();
    } else {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!("[cwa-planner v{version}] MCP server started. Listening on stdin (JSON-RPC 2.0). Press Ctrl+C to stop.");
        eprintln!("[cwa-planner] Tools: cwa_plan_software | Resources: none");
        if let Some(p) = project {
            eprintln!("[cwa-planner] Project: {}", p.display());
        }
    }
}

/// Run the MCP planner server over stdio.
/// Exposes only the `cwa_plan_software` tool for Claude Desktop.
///
/// If `project_dir` contains a valid CWA project (`.cwa/` directory), it is used
/// as the default project context for `cwa_plan_software` calls when the caller
/// does not supply an explicit `project_path` argument.
pub async fn run_planner_stdio(project_dir: &Path) -> anyhow::Result<()> {
    // Only treat as a valid project if the .cwa/ directory exists.
    let effective_project: Option<std::path::PathBuf> =
        if project_dir.join(".cwa").is_dir() {
            Some(project_dir.to_path_buf())
        } else {
            None
        };

    print_startup_banner(effective_project.as_deref());

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

        let response = handle_request(request, effective_project.as_deref()).await;
        if write_response(&mut stdout, &response).await.is_err() {
            break;
        }
    }

    Ok(())
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

async fn handle_request(request: JsonRpcRequest, default_project: Option<&Path>) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tool_call(request.params, default_project).await,
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
        "protocolVersion": "2025-06-18",
        "serverInfo": {
            "name": "cwa-planner",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": {
            "tools": {
                "listChanged": true
            }
        }
    }))
}

fn handle_tools_list() -> Result<serde_json::Value, JsonRpcError> {
    Ok(serde_json::json!({
        "tools": [{
            "name": "cwa_plan_software",
            "description": "Generate a software plan using Domain-Driven Design (DDD) and Specification-Driven Development (SDD) methodologies. Returns executable CWA CLI commands covering: Strategic Design (bounded contexts, subdomains), Architectural Decisions (ADRs), Tech Stack decisions, and Specifications (source of truth with acceptance criteria). The AI asks clarifying questions before generating the plan.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "Description of the desired software, feature, or system to plan"
                    },
                    "project_path": {
                        "type": "string",
                        "description": "Optional absolute path to an existing CWA project directory (contains a .cwa/ directory). When provided, reads current project state and generates a continuation plan that integrates with existing specs and contexts."
                    }
                },
                "required": ["prompt"]
            }
        }]
    }))
}

async fn handle_tool_call(
    params: Option<serde_json::Value>,
    default_project: Option<&Path>,
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

    if name == "cwa_plan_software" {
        let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
        return plan_software(&args, default_project).await;
    }

    // Only cwa_plan_software is available in the planner server
    Err(JsonRpcError {
        code: -32601,
        message: format!("Unknown tool: {}. The planner server only exposes 'cwa_plan_software'. Use 'cwa mcp stdio' for all tools.", name),
    })
}

// ============================================================
// PLANNER TOOL IMPLEMENTATION
// ============================================================

async fn plan_software(
    args: &serde_json::Value,
    default_project: Option<&Path>,
) -> Result<serde_json::Value, JsonRpcError> {
    let prompt = args["prompt"]
        .as_str()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing required parameter: prompt".to_string(),
        })?;

    // Resolve the project path: explicit arg takes priority, then server default.
    let explicit_path = args.get("project_path").and_then(|v| v.as_str()).map(Path::new);
    let project_path: Option<&Path> = explicit_path.or(default_project);

    // Optionally read existing project state
    let existing_state = if let Some(path) = project_path {
        match planner_template::read_existing_state(path).await {
            Ok(state) => Some(state),
            Err(e) => {
                if explicit_path.is_some() {
                    // Explicit path was given but failed — surface the error.
                    return Ok(serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": format!(
                                "Note: Could not read existing project at '{}': {}\n\n{}",
                                path.display(),
                                e,
                                planner_template::render_planning_document(prompt, None)
                            )
                        }]
                    }));
                }
                // Default project detection failed silently — just plan without context.
                None
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

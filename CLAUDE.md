# CWA - Claude Workflow Architect

## Project Overview

CWA is a Rust CLI tool for development workflow orchestration integrated with Claude Code. It combines:
- **Spec Driven Development (SDD)** - Specification management
- **Domain Driven Design (DDD)** - Domain modeling with bounded contexts
- **Kanban** - Task management with WIP limits

## Architecture

```
crates/
├── cwa-cli/     # Binary - CLI entry point (clap)
├── cwa-core/    # Library - Domain logic
├── cwa-db/      # Library - SQLite persistence (rusqlite)
├── cwa-mcp/     # Library - MCP server (JSON-RPC over stdio)
└── cwa-web/     # Library - Web server (axum)
```

### Key Dependencies
- `clap` 4.5 - CLI argument parsing
- `axum` 0.8 - Web framework
- `rusqlite` 0.32 - SQLite database
- `tokio` 1.43 - Async runtime
- `serde` 1.0 - Serialization

## Database Schema

SQLite database at `.cwa/cwa.db` with tables:
- `projects` - Project metadata
- `specs` - Specifications with status and priority
- `bounded_contexts` - DDD contexts
- `domain_objects` - Entities, value objects, aggregates
- `tasks` - Kanban tasks with workflow states
- `kanban_config` - WIP limits per column
- `decisions` - Architectural Decision Records (ADRs)
- `memory` - Session memory entries
- `sessions` - Development sessions
- `glossary` - Domain terminology
- `analyses` - Market/competitor analyses

## CLI Commands

```bash
cwa init <name>              # Initialize project
cwa spec new <title>         # Create specification
cwa spec list                # List all specs
cwa task new <title>         # Create task
cwa task move <id> <status>  # Move task (backlog->todo->in_progress->review->done)
cwa task board               # Display Kanban board
cwa task wip                 # Show WIP limits status
cwa domain context new <n>   # Create bounded context
cwa context status           # Current focus summary
cwa serve                    # Start web server on :3000
cwa mcp stdio                # Run MCP server
```

## MCP Integration

### Tools
| Tool | Description |
|------|-------------|
| `cwa_get_current_task` | Get in-progress task |
| `cwa_get_spec` | Get spec by ID or title |
| `cwa_get_context_summary` | Compact project summary |
| `cwa_get_domain_model` | Bounded contexts and objects |
| `cwa_update_task_status` | Move task to new status |
| `cwa_add_decision` | Record architectural decision |
| `cwa_get_next_steps` | Suggested next actions |
| `cwa_search_memory` | Search project memory |

### Resources
- `project://constitution` - Project values/constraints
- `project://current-spec` - Active specification
- `project://domain-model` - DDD model
- `project://kanban-board` - Task board state
- `project://decisions` - ADR log

## Web API

Base URL: `http://localhost:3000/api`

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/tasks` | GET/POST | List/create tasks |
| `/tasks/{id}` | GET/PUT | Get/update task |
| `/board` | GET | Kanban board with columns |
| `/specs` | GET/POST | List/create specs |
| `/specs/{id}` | GET | Get spec details |
| `/domains` | GET | List bounded contexts |
| `/decisions` | GET/POST | List/create ADRs |
| `/context/summary` | GET | Project context summary |

WebSocket at `/ws` for real-time updates.

## Development Workflow

1. **Spec Creation**: Define feature with acceptance criteria
2. **Task Breakdown**: Create tasks linked to specs
3. **Kanban Flow**: Move tasks through workflow stages
4. **Domain Discovery**: Model bounded contexts as understanding grows
5. **Decision Recording**: Document architectural choices

## Task Status Transitions

```
backlog -> todo -> in_progress -> review -> done
```

WIP limits enforced:
- `todo`: 5
- `in_progress`: 1
- `review`: 2

## Building

```bash
cargo build --release
```

Binary output: `target/release/cwa`

## Testing

```bash
# Initialize test project
cwa init my-project
cd my-project

# Create and manage tasks
cwa task new "Implement feature X" --priority high
cwa task board
cwa task move <task-id> todo
cwa task move <task-id> in_progress

# Start web dashboard
cwa serve
```

## File Locations

When initialized in a project:
```
project/
├── .cwa/
│   └── cwa.db              # SQLite database
├── .claude/
│   ├── agents/             # Agent templates
│   ├── commands/           # Slash commands
│   └── rules/              # Project rules
├── .mcp.json               # MCP server config
├── CLAUDE.md               # Generated context file
└── docs/
    └── constitution.md     # Project constitution
```

# CWA - Claude Workflow Architect

A Rust CLI tool for development workflow orchestration integrated with Claude Code.

## Features

- **Spec Driven Development (SDD)** - Manage specifications with status tracking and validation
- **Domain Driven Design (DDD)** - Model bounded contexts, entities, and domain glossary
- **Kanban Board** - Task management with WIP limits and workflow enforcement
- **MCP Server** - Integration with Claude Code via Model Context Protocol
- **Web Dashboard** - Real-time Kanban board with REST API and WebSocket

## Installation

### From Source

```bash
git clone https://github.com/yourusername/cwa.git
cd cwa
cargo build --release
```

The binary will be at `target/release/cwa`.

### Add to PATH

```bash
# Add to your shell profile
export PATH="$PATH:/path/to/cwa/target/release"
```

## Quick Start

```bash
# Initialize a new project
cwa init my-project
cd my-project

# Create your first specification
cwa spec new "User Authentication" --description "JWT-based auth system" --priority high

# Create tasks from the spec
cwa task new "Set up JWT library" --priority high
cwa task new "Implement login endpoint" --priority high
cwa task new "Add refresh token support" --priority medium

# View the Kanban board
cwa task board

# Move a task through the workflow
cwa task move <task-id> todo
cwa task move <task-id> in_progress

# Check WIP limits
cwa task wip

# Start the web dashboard
cwa serve
# Open http://localhost:3000 in your browser
```

## CLI Reference

### Project Management

```bash
cwa init <name> [--from-prompt <prompt>]   # Initialize new project
cwa context status                          # View current focus
```

### Specifications (SDD)

```bash
cwa spec new <title> [--description <desc>] [--priority <p>]
cwa spec list                               # List all specifications
cwa spec status [<spec>]                    # Show spec details
cwa spec validate <spec>                    # Validate spec completeness
cwa spec archive <spec-id>                  # Archive a specification
```

### Tasks (Kanban)

```bash
cwa task new <title> [--description <d>] [--spec <id>] [--priority <p>]
cwa task move <task-id> <status>            # backlog|todo|in_progress|review|done
cwa task board                              # Display Kanban board
cwa task wip                                # Show WIP limits status
```

### Domain Modeling (DDD)

```bash
cwa domain discover                         # Interactive domain discovery
cwa domain context new <name>               # Create bounded context
cwa domain context list                     # List contexts
cwa domain context map                      # Show context relationships
cwa domain glossary                         # Display domain glossary
```

### Memory & Context

```bash
cwa memory sync                             # Sync CLAUDE.md with current state
cwa memory compact                          # Remove expired memory entries
cwa memory export [--output <file>]         # Export memory as JSON
cwa memory search <query>                   # Search project memory
```

### Servers

```bash
cwa serve [--port <port>]                   # Start web server (default: 3000)
cwa mcp stdio                               # Run MCP server over stdio
```

## MCP Integration

CWA provides a Model Context Protocol server for Claude Code integration.

### Configuration

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "cwa": {
      "command": "cwa",
      "args": ["mcp", "stdio"]
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `cwa_get_current_task` | Get the current in-progress task |
| `cwa_get_spec` | Get a specification by ID or title |
| `cwa_get_context_summary` | Get compact project summary |
| `cwa_get_domain_model` | Get bounded contexts and objects |
| `cwa_update_task_status` | Move a task to a new status |
| `cwa_add_decision` | Record an architectural decision |
| `cwa_get_next_steps` | Get suggested next actions |
| `cwa_search_memory` | Search project memory |

### Available Resources

| URI | Description |
|-----|-------------|
| `project://constitution` | Project values and constraints |
| `project://current-spec` | Currently active specification |
| `project://domain-model` | DDD model with contexts |
| `project://kanban-board` | Current board state |
| `project://decisions` | Architectural decision log |

## Web API

The web server provides a REST API at `http://localhost:3000/api`.

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/tasks` | List all tasks |
| POST | `/api/tasks` | Create a task |
| GET | `/api/tasks/{id}` | Get task by ID |
| PUT | `/api/tasks/{id}` | Update task |
| GET | `/api/board` | Get Kanban board |
| GET | `/api/specs` | List specifications |
| POST | `/api/specs` | Create specification |
| GET | `/api/specs/{id}` | Get spec by ID |
| GET | `/api/domains` | List bounded contexts |
| GET | `/api/decisions` | List decisions |
| POST | `/api/decisions` | Create decision |
| GET | `/api/context/summary` | Get context summary |

### WebSocket

Connect to `/ws` for real-time updates on task changes.

## Project Structure

When you run `cwa init`, the following structure is created:

```
my-project/
├── .cwa/
│   └── cwa.db                 # SQLite database
├── .claude/
│   ├── agents/
│   │   ├── analyst.md         # Requirements analyst agent
│   │   ├── architect.md       # System architect agent
│   │   ├── specifier.md       # Spec writer agent
│   │   ├── implementer.md     # Implementation agent
│   │   └── reviewer.md        # Code review agent
│   ├── commands/
│   │   ├── analyze-market.md
│   │   ├── create-spec.md
│   │   ├── plan-feature.md
│   │   └── implement-task.md
│   └── rules/
│       ├── coding-standards.md
│       ├── git-workflow.md
│       └── documentation.md
├── .mcp.json                   # MCP server configuration
├── CLAUDE.md                   # Context for Claude Code
└── docs/
    └── constitution.md         # Project constitution
```

## Task Workflow

Tasks follow a strict workflow with WIP limits:

```
┌─────────┐    ┌──────┐    ┌─────────────┐    ┌────────┐    ┌──────┐
│ BACKLOG │ -> │ TODO │ -> │ IN_PROGRESS │ -> │ REVIEW │ -> │ DONE │
│   (∞)   │    │ (5)  │    │     (1)     │    │  (2)   │    │ (∞)  │
└─────────┘    └──────┘    └─────────────┘    └────────┘    └──────┘
```

- **backlog**: Unlimited - Ideas and future work
- **todo**: Max 5 - Ready to be picked up
- **in_progress**: Max 1 - Currently being worked on
- **review**: Max 2 - Waiting for review
- **done**: Unlimited - Completed work

## Development

### Prerequisites

- Rust 1.75+ (2024 edition)
- SQLite 3.x

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

### Project Layout

```
cwa/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── cwa-cli/            # CLI binary
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/   # Command handlers
│   │       └── output.rs   # Terminal formatting
│   ├── cwa-core/           # Domain logic
│   │   └── src/
│   │       ├── error.rs
│   │       ├── project/
│   │       ├── spec/
│   │       ├── domain/
│   │       ├── task/
│   │       ├── decision/
│   │       └── memory/
│   ├── cwa-db/             # Database layer
│   │   └── src/
│   │       ├── pool.rs
│   │       ├── migrations/
│   │       └── queries/
│   ├── cwa-mcp/            # MCP server
│   │   └── src/
│   │       └── server.rs
│   └── cwa-web/            # Web server
│       └── src/
│           ├── routes/
│           ├── state.rs
│           └── websocket.rs
└── assets/
    └── web/                # Dashboard static files
```

## License

MIT

## Contributing

Contributions are welcome! Please read the contributing guidelines before submitting a PR.

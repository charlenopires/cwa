# CWA - Claude Workflow Architect

## Project Overview

CWA is a Rust CLI tool for development workflow orchestration integrated with Claude Code. It combines:
- **Spec Driven Development (SDD)** - Specification management with acceptance criteria
- **Domain Driven Design (DDD)** - Domain modeling with bounded contexts and ubiquitous language
- **Kanban** - Task management with WIP limits and workflow enforcement
- **Knowledge Graph** - Neo4j-backed entity relationships and impact analysis
- **Semantic Memory** - Vector embeddings via Ollama + Qdrant for intelligent recall
- **Code Generation** - Generates Claude Code agents, skills, hooks, and CLAUDE.md
- **Token Analysis** - Context budget management and optimization

## Architecture

```
crates/
├── cwa-cli/        # Binary - CLI entry point (clap)
├── cwa-core/       # Library - Domain logic
├── cwa-db/         # Library - SQLite persistence (rusqlite)
├── cwa-graph/      # Library - Neo4j knowledge graph (neo4rs)
├── cwa-embedding/  # Library - Vector embeddings (Ollama + Qdrant)
├── cwa-codegen/    # Library - .claude/ artifact generation
├── cwa-token/      # Library - Token counting (tiktoken-rs)
├── cwa-mcp/        # Library - MCP server (JSON-RPC over stdio)
└── cwa-web/        # Library - Web server (axum + HTMX + Askama)
```

### Key Dependencies
- `clap` 4.5 - CLI argument parsing
- `axum` 0.8 - Web framework
- `rusqlite` 0.32 - SQLite database
- `neo4rs` 0.8 - Neo4j graph database driver
- `qdrant-client` 1.12 - Qdrant vector store
- `reqwest` 0.12 - HTTP client (Ollama API)
- `tiktoken-rs` 0.6 - Token counting (cl100k_base)
- `askama` 0.12 - Compile-time HTML templates
- `tokio` 1.43 - Async runtime
- `serde` 1.0 - Serialization

## Database Schema

SQLite database at `.cwa/cwa.db` with tables:
- `projects` - Project metadata
- `specs` - Specifications with status, priority, acceptance criteria
- `bounded_contexts` - DDD contexts
- `domain_objects` - Entities, value objects, aggregates with invariants
- `tasks` - Kanban tasks with workflow states
- `kanban_config` - WIP limits per column
- `decisions` - Architectural Decision Records (ADRs)
- `memory` - Legacy session memory entries
- `memories` - Enhanced memory entries with embeddings
- `sessions` - Development sessions
- `glossary` - Domain terminology
- `analyses` - Market/competitor analyses
- `boards` - Kanban boards (web UI)
- `columns` - Board columns with WIP limits
- `cards` - Board cards with positions
- `labels` / `card_labels` - Card labeling
- `card_history` - Card movement audit trail
- `sync_state` - Neo4j sync tracking

## CLI Commands

```bash
# Project
cwa init <name>                          # Initialize project
cwa context status                       # Current focus summary

# Specifications (SDD)
cwa spec new <title>                     # Create specification
cwa spec from-prompt [<text>] [--file]   # Parse long prompt into multiple specs
cwa spec list                            # List all specs
cwa spec status [<spec>]                 # Show spec details
cwa spec validate <spec>                 # Validate completeness
cwa spec archive <spec-id>               # Archive a spec

# Tasks (Kanban)
cwa task new <title>                     # Create task
cwa task move <id> <status>              # Move task through workflow
cwa task board                           # Display Kanban board
cwa task wip                             # Show WIP limits status

# Domain Modeling (DDD)
cwa domain discover                      # Interactive domain discovery
cwa domain context new <name>            # Create bounded context
cwa domain context list                  # List contexts
cwa domain context map                   # Show context relationships
cwa domain glossary                      # Display domain glossary

# Memory (Semantic)
cwa memory add "<content>" -t <type>     # Add memory with embedding
cwa memory search "<query>" [--top-k N]  # Semantic search
cwa memory import                        # Import legacy entries
cwa memory compact [--min-confidence N]  # Remove low-confidence entries
cwa memory sync                          # Sync CLAUDE.md
cwa memory export [--output <file>]      # Export as JSON

# Knowledge Graph
cwa graph sync                           # Sync SQLite -> Neo4j
cwa graph query "<cypher>"               # Execute Cypher query
cwa graph impact <type> <id>             # Impact analysis
cwa graph explore <type> <id> [--depth]  # Explore neighborhood
cwa graph status                         # Node/relationship counts

# Code Generation
cwa codegen agent [context-id]           # Generate agent from context
cwa codegen agent --all                  # Generate all agents
cwa codegen skill <spec-id>              # Generate skill from spec
cwa codegen hooks                        # Generate validation hooks
cwa codegen claude-md                    # Regenerate CLAUDE.md
cwa codegen all                          # Generate all artifacts
# All codegen commands support --dry-run

# Token Analysis
cwa tokens analyze [path]                # Count tokens for file
cwa tokens analyze --all                 # Count all context files
cwa tokens optimize [--budget N]         # Suggest optimizations
cwa tokens report                        # Full breakdown report

# Infrastructure (Docker)
cwa infra up                             # Start Neo4j + Qdrant + Ollama
cwa infra down                           # Stop all services
cwa infra status                         # Check service health
cwa infra logs [service] [--follow]      # View service logs
cwa infra reset --confirm                # Destroy all data

# Servers
cwa serve [--port <port>]                # Start web server (default: 3000)
cwa mcp stdio                            # Run MCP server
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
| `cwa_graph_query` | Execute Cypher query on graph |
| `cwa_graph_impact` | Analyze entity impact |
| `cwa_graph_sync` | Trigger SQLite -> Neo4j sync |
| `cwa_memory_semantic_search` | Vector similarity search |
| `cwa_memory_add` | Store memory with embedding |

### Resources
- `project://constitution` - Project values/constraints
- `project://current-spec` - Active specification
- `project://domain-model` - DDD model
- `project://kanban-board` - Task board state
- `project://decisions` - ADR log

## Docker Services

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| Neo4j | neo4j:5.26-community | 7474, 7687 | Knowledge Graph |
| Qdrant | qdrant/qdrant:v1.13.2 | 6333, 6334 | Vector Store |
| Ollama | ollama/ollama:0.5.4 | 11434 | Local Embeddings (nomic-embed-text) |

## Web API

Base URL: `http://localhost:3000`

### REST Endpoints (`/api/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/api/tasks` | List/create tasks |
| GET/PUT | `/api/tasks/{id}` | Get/update task |
| GET | `/api/board` | Kanban board with columns |
| GET/POST | `/api/specs` | List/create specs |
| GET | `/api/specs/{id}` | Get spec details |
| GET | `/api/domains` | List bounded contexts |
| GET/POST | `/api/decisions` | List/create ADRs |
| GET | `/api/context/summary` | Project context summary |

### HTMX Board (root `/`)
- Drag-and-drop Kanban board with Sortable.js
- Real-time card movement
- WIP limit enforcement

WebSocket at `/ws` for real-time updates.

## Task Status Transitions

```
backlog -> todo -> in_progress -> review -> done
```

WIP limits enforced:
- `todo`: 5
- `in_progress`: 1
- `review`: 2

## Code Generation Output

```
.claude/
├── agents/           # One .md per bounded context (+ 8 built-in)
├── skills/           # One dir per approved spec (+ 2 built-in)
│   └── <slug>/
│       └── SKILL.md
├── commands/         # Slash commands (8 built-in)
├── rules/            # Code rules (5 built-in)
└── hooks.json        # Validation hooks from invariants
CLAUDE.md             # Regenerated project context
```

## Building

```bash
cargo build --release    # Binary: target/release/cwa
cargo test --workspace   # Run all tests
```

## File Locations

```
project/
├── .cwa/
│   ├── cwa.db               # SQLite database
│   └── constitution.md      # Project values & constraints
├── .claude/
│   ├── agents/              # 8 built-in + generated from contexts
│   ├── skills/              # 2 built-in + generated from specs
│   ├── commands/            # 8 slash commands
│   ├── rules/               # 5 rule files
│   └── hooks.json           # Validation hooks
├── docker/
│   ├── docker-compose.yml   # Infrastructure services
│   ├── neo4j/conf/          # Neo4j configuration
│   └── scripts/             # Init scripts
├── .mcp.json                # MCP server config
├── CLAUDE.md                # Generated context file
└── docs/
    └── constitution.md      # Project constitution
```

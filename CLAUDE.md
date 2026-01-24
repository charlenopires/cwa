# CWA - Claude Workflow Architect

## Project Overview

CWA is a Rust CLI tool that provides **persistent project intelligence for Claude Code**. It bridges the gap between AI-assisted development sessions by maintaining structured context (specs, domain models, decisions, tasks) that Claude Code accesses through MCP, generated artifacts, and auto-regenerated CLAUDE.md files.

### Core Capabilities
- **Spec Driven Development (SDD)** - Specification management with acceptance criteria
- **Domain Driven Design (DDD)** - Domain modeling with bounded contexts and ubiquitous language
- **Kanban** - Task management with WIP limits and workflow enforcement
- **Knowledge Graph** - Neo4j-backed entity relationships and impact analysis
- **Semantic Memory** - Vector embeddings via Ollama + Qdrant for intelligent recall
- **Code Generation** - Generates Claude Code agents, skills, hooks, and CLAUDE.md
- **Token Analysis** - Context budget management and optimization

## Claude Code Integration Model

CWA integrates with Claude Code through three channels, each serving a different purpose:

### Channel 1: MCP Server (Real-time)
The MCP server (`cwa mcp stdio`) gives Claude Code live access to project state during sessions.
- **Tools**: 18 callable functions for reading/writing project data
- **Resources**: 5 URIs for quick context loading (constitution, specs, domain, board, decisions)
- **Progressive disclosure**: Timeline gives ~50 tokens/observation; full details are ~500 tokens each

### Channel 2: Generated Artifacts (Session Start)
The `.claude/` directory is read by Claude Code at session initialization:
- **Agents** (8 built-in + 1 per bounded context) - Personas with domain expertise and MCP tool access
- **Skills** (2 built-in + 1 per approved spec) - Repeatable multi-step workflows
- **Commands** (8 built-in) - Slash commands for common workflows
- **Rules** (5 built-in) - Constraints enforced during code generation
- **Hooks** (3 built-in + generated from invariants) - Event-driven validation

### Channel 3: CLAUDE.md (Session Context)
Auto-regenerated file (`cwa codegen claude-md`) containing:
- Domain model with entities and invariants
- Active specs with acceptance criteria
- Key decisions (top 10 accepted ADRs)
- Current work state (in-progress tasks)
- Recent high-confidence observations
- Last session summary

### Integration Flow by Development Phase

| Phase | Claude Code Actions | CWA Provides |
|-------|--------------------|--------------|
| **Planning** | Reads context, creates specs, generates tasks | `cwa_get_context_summary`, `cwa_search_memory`, `project://current-spec` |
| **Design** | Models domain, discovers contexts, defines invariants | `cwa_get_domain_model`, `cwa_graph_impact`, `cwa_graph_sync` |
| **Implementation** | Uses agents/skills, follows rules, writes code | `cwa_get_current_task`, `cwa_get_spec`, agents, skills, rules |
| **Review** | Validates against criteria, checks invariants | `cwa_get_spec`, hooks.json, reviewer agent |
| **Memory** | Records observations, decisions, patterns | `cwa_observe`, `cwa_memory_add`, `cwa_add_decision` |
| **Sync** | Regenerates all artifacts | `cwa codegen all`, `cwa tokens optimize` |

### Key Architectural Decisions for Contributors

1. **MCP tools are the primary interface** - Agents, skills, and commands all call MCP tools internally
2. **SQLite is source of truth** - Neo4j and Qdrant are derived stores, rebuildable from SQLite
3. **Progressive disclosure for memory** - Timeline first (cheap), then full details (expensive)
4. **WIP limits enforced by MCP** - `cwa_update_task_status` rejects moves that exceed limits
5. **Token budget awareness** - CLAUDE.md and artifacts are optimized to fit within model context windows
6. **Confidence lifecycle** - Observations start at 0.8, decay over time, get removed below threshold

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
- `reqwest` 0.12 - HTTP client (Ollama API, Claude Vision API)
- `base64` 0.22 - Base64 encoding (image payloads for Claude API)
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
- `design_systems` - Design tokens extracted from UI screenshots
- `observations` - Structured development observations with confidence lifecycle
- `summaries` - Compressed session/time-range observation summaries

## CLI Commands

```bash
# Project
cwa init <name>                          # Initialize project
cwa context status                       # Current focus summary

# Specifications (SDD)
cwa spec new <title> [-c <criterion>]... # Create specification (with optional criteria)
cwa spec from-prompt [<text>] [--file]   # Parse long prompt into multiple specs
cwa spec add-criteria <spec> <crit>...   # Add acceptance criteria to existing spec
cwa spec list                            # List all specs
cwa spec status [<spec>]                 # Show spec details
cwa spec validate <spec>                 # Validate completeness
cwa spec archive <spec-id>               # Archive a spec
cwa spec clear [--confirm]               # Delete all specs

# Tasks (Kanban)
cwa task new <title>                     # Create task
cwa task list                            # List all tasks
cwa task generate <spec> [--dry-run]     # Auto-create tasks from spec criteria
cwa task move <id> <status>              # Move task through workflow
cwa task board                           # Display Kanban board
cwa task wip                             # Show WIP limits status
cwa task clear <spec> [--confirm]        # Delete all tasks for a spec

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
cwa memory compact --decay 0.98          # Decay all observation confidences
cwa memory sync                          # Sync CLAUDE.md
cwa memory export [--output <file>]      # Export as JSON

# Observations (Structured Memory)
cwa memory observe "<title>" -t <type>   # Record observation (bugfix/feature/refactor/discovery/decision/change/insight)
cwa memory observe "<title>" -t discovery -f "fact1" -f "fact2"  # With facts
cwa memory observe "<title>" -n "narrative" --files-modified src/foo.rs  # With context
cwa memory timeline [--days N] [--limit N]  # View recent observations timeline
cwa memory summarize [--count N]         # Generate summary from recent observations

# Design System
cwa design from-image <url>              # Extract design system from screenshot (requires ANTHROPIC_API_KEY)
cwa design from-image <url> --dry-run    # Preview without saving
cwa design from-image <url> --model <m>  # Use specific Claude model

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

The MCP server is the **primary runtime interface** between Claude Code and CWA. All agents, skills, and commands ultimately call these tools.

### Tools (by Phase)

**Planning & Context:**
| Tool | Description | Used By |
|------|-------------|---------|
| `cwa_get_context_summary` | Compact project summary | orchestrator, /status |
| `cwa_get_spec` | Spec with acceptance criteria | specifier, implementer, reviewer, tester |
| `cwa_get_next_steps` | Suggested next actions | orchestrator |
| `cwa_generate_tasks` | Create tasks from spec criteria | specifier, /create-spec |
| `cwa_create_context` | Create a bounded context | architect, specifier |
| `cwa_create_spec` | Create a spec with criteria | specifier, orchestrator |
| `cwa_create_task` | Create a task | specifier, orchestrator |

**Implementation & Workflow:**
| Tool | Description | Used By |
|------|-------------|---------|
| `cwa_get_current_task` | In-progress task details | implementer, reviewer, /implement-task |
| `cwa_update_task_status` | Move task (enforces WIP limits) | implementer, reviewer, orchestrator |
| `cwa_get_domain_model` | Bounded contexts and objects | architect, [context]-expert agents |

**Knowledge Graph:**
| Tool | Description | Used By |
|------|-------------|---------|
| `cwa_graph_query` | Execute Cypher query | architect |
| `cwa_graph_impact` | Entity impact analysis | architect, /domain-discover |
| `cwa_graph_sync` | Sync SQLite → Neo4j | orchestrator, /sync-context |

**Memory & Learning:**
| Tool | Description | Used By |
|------|-------------|---------|
| `cwa_search_memory` | Text search project memory | all agents |
| `cwa_memory_semantic_search` | Vector similarity search | analyst, architect |
| `cwa_memory_add` | Store memory with embedding | all agents |
| `cwa_add_decision` | Record ADR with rationale | architect, orchestrator, documenter |
| `cwa_observe` | Structured observation (bugfix/feature/discovery/decision/change/insight) | implementer, tester |
| `cwa_memory_timeline` | Compact timeline (~50 tokens/entry) | orchestrator, /session-summary |
| `cwa_memory_get` | Full details by IDs (~500 tokens/entry) | on-demand deep dive |
| `cwa_memory_search_all` | Search memories + observations | analyst, architect |

### Resources (Loaded at Session Start)
| URI | Content | Token Cost |
|-----|---------|------------|
| `project://constitution` | Project values/constraints | ~200 tokens |
| `project://current-spec` | Active spec with criteria | ~300 tokens |
| `project://domain-model` | DDD contexts + entities | ~500 tokens |
| `project://kanban-board` | Task board state | ~200 tokens |
| `project://decisions` | Recent ADR log | ~400 tokens |

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
| POST | `/api/specs/{id}/generate-tasks` | Generate tasks from criteria |
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

All generated files map to Claude Code features:

```
.claude/
├── agents/           # Claude Code Agents: one per bounded context (+ 8 built-in)
│                     # → Each agent has role, allowed tools, and MCP tool references
├── skills/           # Claude Code Skills: one per approved spec (+ 2 built-in)
│   └── <slug>/       # → Repeatable workflows with acceptance criteria
│       └── SKILL.md
├── commands/         # Claude Code Commands: 8 slash commands
│                     # → Quick-access workflows (/project:next-task, /project:status, etc.)
├── rules/            # Claude Code Rules: 5 constraint files
│                     # → Enforced during code generation (workflow, domain, tests, api, memory)
├── design-system.md  # Design tokens extracted via Claude Vision API
│                     # → Referenced by implementer agent for UI consistency
└── hooks.json        # Claude Code Hooks: event-driven validation
                      # → pre-commit: WIP check, invariant validation
                      # → post-test: task advancement reminder
CLAUDE.md             # Session context: domain, specs, decisions, current work, observations
```

### How Artifacts Are Generated

| Artifact | Source Data | Generator | Trigger |
|----------|------------|-----------|---------|
| `agents/[context]-expert.md` | Bounded context + entities | `cwa codegen agent` | New context created |
| `skills/[spec-slug]/SKILL.md` | Spec + acceptance criteria | `cwa codegen skill` | Spec approved/active |
| `hooks.json` | Domain object invariants | `cwa codegen hooks` | Invariants updated |
| `CLAUDE.md` | All project state | `cwa codegen claude-md` | Any significant change |
| `design-system.md` | UI screenshot analysis | `cwa design from-image` | Manual trigger |

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
│   ├── design-system.md     # Design tokens (from cwa design from-image)
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

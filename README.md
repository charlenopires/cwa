# CWA - Claude Workflow Architect

A Rust CLI tool for development workflow orchestration integrated with Claude Code. CWA bridges the gap between project management, domain modeling, and AI-assisted development by providing structured context that Claude Code can leverage through MCP.

## Features

- **Spec Driven Development (SDD)** - Manage specifications with status tracking, acceptance criteria, and validation
- **Domain Driven Design (DDD)** - Model bounded contexts, entities, value objects, aggregates, and domain glossary
- **Kanban Board** - Task management with WIP limits and workflow enforcement (CLI + web)
- **Knowledge Graph** - Neo4j-backed entity relationships, impact analysis, and exploration
- **Semantic Memory** - Vector embeddings via Ollama + Qdrant for intelligent context recall
- **Code Generation** - Generate Claude Code agents, skills, hooks, and CLAUDE.md from your domain model
- **Token Analysis** - Count tokens, estimate costs, and optimize context budget
- **MCP Server** - Full Model Context Protocol integration with Claude Code
- **Web Dashboard** - HTMX + Askama Kanban board with drag-and-drop

## Installation

### macOS (Script)

```bash
git clone https://github.com/charlenopires/cwa.git
cd cwa
./install.sh
```

The script installs Rust (if needed), builds the release binary, and copies it to `/usr/local/bin`.

### From Source

```bash
git clone https://github.com/charlenopires/cwa.git
cd cwa
cargo build --release
cp target/release/cwa /usr/local/bin/
```

### Prerequisites

- **Rust 1.83+** (2021 edition)
- **Docker** (optional - required for Knowledge Graph, Embeddings, and Semantic Memory features)

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

### With Knowledge Graph & Semantic Memory

```bash
# Start Docker infrastructure
cwa infra up
# This starts Neo4j, Qdrant, and Ollama with nomic-embed-text

# Check services are healthy
cwa infra status

# Add domain knowledge
cwa domain context new "Auth"
cwa domain context new "Payments"

# Sync to Knowledge Graph
cwa graph sync

# Explore relationships
cwa graph impact context <context-id>
cwa graph explore context <context-id> --depth 3

# Add semantic memories
cwa memory add "User prefers RS256 JWT tokens" --type decision
cwa memory add "All payments use Stripe API" --type fact

# Semantic search
cwa memory search "authentication tokens"
```

## CLI Reference

### Project Management

```bash
cwa init <name> [--from-prompt <prompt>]   # Initialize new project
cwa context status                          # View current project focus
```

**Example:**
```bash
$ cwa init my-saas
✓ Project 'my-saas' initialized
  Database: .cwa/cwa.db
  Config: .mcp.json

$ cwa context status
Project: my-saas
  Specs: 3 (1 active, 2 draft)
  Tasks: 5 (1 in_progress, 2 todo, 2 backlog)
  Contexts: 2
```

### Specifications (SDD)

```bash
cwa spec new <title> [--description <desc>] [--priority <p>]
cwa spec list
cwa spec status [<spec>]
cwa spec validate <spec>
cwa spec archive <spec-id>
```

**Example:**
```bash
$ cwa spec new "User Authentication" --description "JWT-based auth" --priority high
✓ Spec created: abc123

$ cwa spec list
ID       Title                  Status   Priority
abc123   User Authentication    draft    high
def456   Payment Processing     active   medium
```

### Tasks (Kanban)

```bash
cwa task new <title> [--description <d>] [--spec <id>] [--priority <p>]
cwa task move <task-id> <status>   # backlog|todo|in_progress|review|done
cwa task board                     # Display Kanban board
cwa task wip                       # Show WIP limits status
```

**Example:**
```bash
$ cwa task board
┌─────────┬──────────┬─────────────┬────────┬──────┐
│ BACKLOG │   TODO   │ IN_PROGRESS │ REVIEW │ DONE │
│   (∞)   │  (5/5)   │    (1/1)    │ (0/2)  │ (∞)  │
├─────────┼──────────┼─────────────┼────────┼──────┤
│ Task 5  │ Task 3   │ Task 1      │        │      │
│ Task 6  │ Task 4   │             │        │      │
└─────────┴──────────┴─────────────┴────────┴──────┘

$ cwa task move abc123 in_progress
✓ Task moved to in_progress

$ cwa task wip
  todo:        3/5
  in_progress: 1/1 (at limit)
  review:      0/2
```

### Domain Modeling (DDD)

```bash
cwa domain discover                    # Interactive domain discovery
cwa domain context new <name>          # Create bounded context
cwa domain context list                # List contexts
cwa domain context map                 # Show context relationships
cwa domain glossary                    # Display domain glossary
```

**Example:**
```bash
$ cwa domain context new "Authentication" --description "User identity and access"
✓ Context created: auth-ctx-001

$ cwa domain context list
ID            Name             Description
auth-ctx-001  Authentication   User identity and access
pay-ctx-002   Payments         Payment processing and billing

$ cwa domain glossary
Term          Definition
JWT           JSON Web Token for stateless auth
Aggregate     Cluster of domain objects with consistency boundary
```

### Memory (Semantic)

```bash
cwa memory add "<content>" -t <type>        # preference|decision|fact|pattern
cwa memory search "<query>" [--top-k N]     # Semantic search (default: 5 results)
cwa memory search "<query>" --legacy        # Text-based search (no embeddings)
cwa memory import                           # Import legacy entries with embeddings
cwa memory compact [--min-confidence 0.3]   # Remove low-confidence entries
cwa memory sync                             # Sync CLAUDE.md with current state
cwa memory export [--output <file>]         # Export memory as JSON
```

**Example:**
```bash
$ cwa memory add "Team prefers functional patterns over OOP" --type preference
✓ Memory added (id: a1b2c3d4, embedding: 768 dims)

$ cwa memory search "coding style"
✓ Found 2 results:

  1. [preference] Team prefers functional patterns over OOP (92%)
  2. [fact] Codebase uses Rust with trait-based composition (78%)

$ cwa memory compact --min-confidence 0.3
✓ Removed 5 low-confidence memories
```

### Knowledge Graph

Requires Docker infrastructure (`cwa infra up`).

```bash
cwa graph sync                                    # Full sync SQLite → Neo4j
cwa graph query "<cypher>"                        # Execute raw Cypher
cwa graph impact <entity-type> <entity-id>        # Impact analysis
cwa graph explore <entity-type> <entity-id> [--depth N]  # Neighborhood
cwa graph status                                  # Graph statistics
```

**Entity types:** `spec`, `task`, `context`, `decision`

**Example:**
```bash
$ cwa graph sync
Syncing to Knowledge Graph...
✓ Sync complete:
  Nodes created/updated: 15
  Relationships created: 23

$ cwa graph status
Knowledge Graph Status
────────────────────────────────────
  Nodes:         15
  Relationships: 23
  Last sync:     2026-01-23T10:30:00
────────────────────────────────────

$ cwa graph impact spec abc123
Impact analysis for spec abc123
──────────────────────────────────────────────
  → [Task] Implement login endpoint (IMPLEMENTS)
  → [Task] Add JWT validation (IMPLEMENTS)
  → [BoundedContext] Authentication (BELONGS_TO)
  → [Decision] Use RS256 algorithm (RELATES_TO)

4 related entities found.

$ cwa graph explore context auth-001 --depth 2
Exploring context auth-001 (depth=2)
──────────────────────────────────────────────
Center: [BoundedContext] Authentication

Connected nodes (6):
  • [DomainEntity] User
  • [DomainEntity] Session
  • [Term] JWT
  • [Spec] User Authentication
  • [Task] Implement login
  • [Decision] Use RS256
```

### Code Generation

Generates Claude Code artifacts from your domain model.

```bash
cwa codegen agent [context-id]     # Agent from bounded context
cwa codegen agent --all            # All agents
cwa codegen skill <spec-id>        # Skill from spec
cwa codegen hooks                  # Validation hooks from invariants
cwa codegen claude-md              # Regenerate CLAUDE.md
cwa codegen all                    # Generate everything
# All commands support --dry-run
```

**Example:**
```bash
$ cwa codegen all --dry-run
Generating all artifacts...
  2 agents: authentication-expert.md, payments-expert.md
  1 skills: user-authentication
  3 hooks
  CLAUDE.md

(dry run - no files written)

$ cwa codegen all
Generating all artifacts...
  ✓ 2 agents
  ✓ 1 skills
  ✓ 3 hooks
  ✓ CLAUDE.md

All artifacts generated.
```

**Generated agent example** (`.claude/agents/authentication-expert.md`):
```markdown
# Authentication Expert

## Role
You are an expert in the Authentication bounded context.

## Responsibilities
- User identity and access management

## Domain Entities
- `User` (aggregate) - Core user identity
- `Session` (entity) - Active user session

## Key Terms
- JWT: JSON Web Token for stateless auth
```

### Token Analysis

```bash
cwa tokens analyze [path]          # Analyze single file
cwa tokens analyze --all           # Analyze all context files
cwa tokens optimize [--budget N]   # Suggest optimizations (default: 8000)
cwa tokens report                  # Full report with chart
```

**Example:**
```bash
$ cwa tokens analyze --all
Token Analysis (All Context Files)
────────────────────────────────────────────────────────────
   2340 (45%) CLAUDE.md
    890 (17%) .claude/agents/authentication-expert.md
    650 (12%) .claude/agents/payments-expert.md
    420 ( 8%) .claude/skills/user-authentication/SKILL.md
    340 ( 6%) .claude/hooks.json
────────────────────────────────────────────────────────────
  5180 total tokens across 5 files

$ cwa tokens optimize --budget 4000
Token Optimization Budget: 4000 tokens, Current: 5180 tokens
──────────────────────────────────────────────────────
  Excess: 1180 tokens need to be reduced

Suggestions:
  1. [HIGH] ~400 tokens: Trim verbose descriptions in CLAUDE.md
  2. [MED]  ~250 tokens: Consolidate similar agent sections
  3. [LOW]  ~180 tokens: Remove redundant comments

  Potential savings: ~830 tokens
```

### Infrastructure (Docker)

Manages Neo4j, Qdrant, and Ollama containers.

```bash
cwa infra up                       # Start all services + pull model
cwa infra down                     # Stop services
cwa infra status                   # Health check
cwa infra logs [service] [--follow]  # View logs
cwa infra reset --confirm          # Destroy all data + volumes
```

**Example:**
```bash
$ cwa infra up
Starting CWA infrastructure...

Waiting for services to be healthy...
  neo4j ... healthy
  qdrant ... healthy
  ollama ... healthy

Pulling embedding model (nomic-embed-text)...
  Model ready

Infrastructure ready.
  Neo4j Browser: http://localhost:7474
  Qdrant API:    http://localhost:6333
  Ollama API:    http://localhost:11434

$ cwa infra status
CWA Infrastructure Status
────────────────────────────────────
  ● neo4j      running
  ● qdrant     running
  ● ollama     running
    model      ready
────────────────────────────────────
```

### Servers

```bash
cwa serve [--port <port>]          # Start web server (default: 3000)
cwa mcp stdio                     # Run MCP server over stdio
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
| `cwa_search_memory` | Search project memory (text) |
| `cwa_graph_query` | Execute Cypher query on knowledge graph |
| `cwa_graph_impact` | Analyze impact of entity changes |
| `cwa_graph_sync` | Trigger SQLite to Neo4j sync |
| `cwa_memory_semantic_search` | Vector similarity search |
| `cwa_memory_add` | Store memory with embedding |

### Available Resources

| URI | Description |
|-----|-------------|
| `project://constitution` | Project values and constraints |
| `project://current-spec` | Currently active specification |
| `project://domain-model` | DDD model with contexts |
| `project://kanban-board` | Current board state |
| `project://decisions` | Architectural decision log |

## Web Dashboard

Start with `cwa serve` and open `http://localhost:3000`.

### REST API (`/api/*`)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/tasks` | List all tasks |
| POST | `/api/tasks` | Create a task |
| GET | `/api/tasks/{id}` | Get task by ID |
| PUT | `/api/tasks/{id}` | Update task |
| GET | `/api/board` | Get Kanban board with columns |
| GET | `/api/specs` | List specifications |
| POST | `/api/specs` | Create specification |
| GET | `/api/specs/{id}` | Get spec by ID |
| GET | `/api/domains` | List bounded contexts |
| GET | `/api/decisions` | List decisions |
| POST | `/api/decisions` | Create decision |
| GET | `/api/context/summary` | Get context summary |

### HTMX Kanban Board (root `/`)

The web UI uses HTMX + Sortable.js for a drag-and-drop Kanban board:
- Drag cards between columns
- WIP limits enforced visually
- Real-time updates via WebSocket at `/ws`

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

## Docker Services

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| Neo4j | `neo4j:5.26-community` | 7474 (HTTP), 7687 (Bolt) | Knowledge Graph |
| Qdrant | `qdrant/qdrant:v1.13.2` | 6333 (HTTP), 6334 (gRPC) | Vector Store |
| Ollama | `ollama/ollama:0.5.4` | 11434 | Embeddings (nomic-embed-text, 768 dims) |

Default credentials (configurable via `docker/.env`):
- Neo4j: `neo4j` / `cwa_dev_2026`

## Project Structure

When you run `cwa init`, the following structure is created:

```
my-project/
├── .cwa/
│   └── cwa.db                 # SQLite database
├── .claude/
│   ├── agents/                # Agent definitions (generated + custom)
│   │   ├── analyst.md
│   │   ├── architect.md
│   │   ├── specifier.md
│   │   ├── implementer.md
│   │   └── reviewer.md
│   ├── skills/                # Skill definitions (generated from specs)
│   │   └── <slug>/
│   │       └── SKILL.md
│   ├── commands/              # Slash commands
│   │   ├── analyze-market.md
│   │   ├── create-spec.md
│   │   ├── plan-feature.md
│   │   └── implement-task.md
│   ├── rules/                 # Project rules
│   │   ├── coding-standards.md
│   │   ├── git-workflow.md
│   │   └── documentation.md
│   └── hooks.json             # Generated validation hooks
├── docker/
│   ├── docker-compose.yml     # Neo4j + Qdrant + Ollama
│   ├── .env.example           # Default configuration
│   ├── neo4j/conf/            # Neo4j configuration
│   └── scripts/               # Init scripts (Cypher, Qdrant)
├── .mcp.json                  # MCP server configuration
├── CLAUDE.md                  # Context file for Claude Code
└── docs/
    └── constitution.md        # Project constitution
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# Run tests
cargo test --workspace
```

### Crate Layout

```
cwa/
├── Cargo.toml                # Workspace manifest
├── crates/
│   ├── cwa-cli/              # CLI binary (clap)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/     # Command handlers
│   │       └── output.rs     # Terminal formatting
│   ├── cwa-core/             # Domain logic
│   │   └── src/
│   │       ├── project/
│   │       ├── spec/
│   │       ├── domain/
│   │       ├── task/
│   │       ├── board/        # Kanban board model
│   │       ├── decision/
│   │       └── memory/
│   ├── cwa-db/               # SQLite persistence
│   │   └── src/
│   │       ├── pool.rs
│   │       ├── migrations/   # Numbered SQL migrations
│   │       └── queries/      # CRUD query modules
│   ├── cwa-graph/            # Neo4j integration
│   │   └── src/
│   │       ├── client.rs     # Connection pool
│   │       ├── schema.rs     # Constraints/indexes
│   │       ├── sync/         # SQLite → Neo4j sync pipeline
│   │       └── queries/      # Impact, explore, search
│   ├── cwa-embedding/        # Vector embeddings
│   │   └── src/
│   │       ├── ollama.rs     # Ollama HTTP client
│   │       ├── qdrant.rs     # Qdrant gRPC client
│   │       ├── memory.rs     # Memory indexing pipeline
│   │       └── search.rs     # Semantic search
│   ├── cwa-codegen/          # Artifact generation
│   │   └── src/
│   │       ├── agents.rs     # Agent .md from contexts
│   │       ├── skills.rs     # Skill .md from specs
│   │       ├── hooks.rs      # hooks.json from invariants
│   │       └── claude_md.rs  # CLAUDE.md regeneration
│   ├── cwa-token/            # Token analysis
│   │   └── src/
│   │       ├── analyzer.rs   # Token counting (cl100k_base)
│   │       ├── optimizer.rs  # Budget optimization
│   │       └── reporter.rs   # Usage reports
│   ├── cwa-mcp/              # MCP server
│   │   └── src/
│   │       └── server.rs     # JSON-RPC over stdio
│   └── cwa-web/              # Web server
│       ├── src/
│       │   ├── routes/       # REST + HTMX handlers
│       │   ├── state.rs
│       │   └── websocket.rs
│       └── templates/        # Askama HTML templates
├── docker/                   # Docker Compose infrastructure
└── install.sh                # macOS installation script
```

## End-to-End Workflow Example

```bash
# 1. Create and initialize project
cwa init my-saas
cd my-saas

# 2. Start infrastructure (optional, for graph/embeddings)
cwa infra up

# 3. Define your domain
cwa domain context new "UserManagement" --description "User registration and profiles"
cwa domain context new "Billing" --description "Subscriptions and payments"

# 4. Create specifications
cwa spec new "User Registration" --priority high \
  --description "Allow users to sign up with email/password"

# 5. Break down into tasks
cwa task new "Create User model" --priority high --spec <spec-id>
cwa task new "Implement signup endpoint" --priority high
cwa task new "Add email verification" --priority medium

# 6. Work through the Kanban
cwa task move <id> todo
cwa task move <id> in_progress
# ... do the work ...
cwa task move <id> review
cwa task move <id> done

# 7. Record decisions as you go
cwa memory add "Using bcrypt for password hashing" --type decision
cwa memory add "Email verification required before login" --type fact

# 8. Sync to Knowledge Graph
cwa graph sync
cwa graph impact spec <spec-id>

# 9. Generate Claude Code artifacts
cwa codegen all

# 10. Analyze token budget
cwa tokens report

# 11. Open web dashboard
cwa serve
```

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a PR at [github.com/charlenopires/cwa](https://github.com/charlenopires/cwa).

# CWA v0.9.0 — Claude Workflow Architect

A Rust CLI tool for development workflow orchestration integrated with Claude Code. CWA bridges the gap between project management, domain modeling, and AI-assisted development by providing structured context that Claude Code can leverage through MCP.

## Features

- **Spec Driven Development (SDD)** - Manage specifications with status tracking, acceptance criteria, and validation
- **Domain Driven Design (DDD)** - Model bounded contexts, entities, value objects, aggregates, and domain glossary
- **Kanban Board** - Task management with WIP limits and workflow enforcement (CLI + web)
- **Knowledge Graph** - Neo4j-backed entity relationships, impact analysis, and exploration
- **Semantic Memory** - Vector embeddings via Ollama + Qdrant for intelligent context recall
- **Git Integration with Local LLM** - Generate commit messages using Ollama (qwen2.5-coder), saving Claude tokens
- **Design System Extraction** - Analyze UI screenshots via Claude Vision API to generate design tokens
- **Tech Stack Agents** - 28 expert agent templates selected automatically from your `.cwa/stack.json`
- **Code Generation** - Generate Claude Code agents, skills, hooks, commands, and CLAUDE.md from your domain model
- **Token Analysis** - Count tokens, estimate costs, and optimize context budget
- **MCP Server** - Full Model Context Protocol integration with 39 tools and 12 resources
- **Web Dashboard** - HTMX + Askama Kanban board with drag-and-drop and real-time WebSocket auto-refresh

## Why CWA?

AI-assisted development with tools like Claude Code faces a fundamental problem: **context fragmentation**. Every new session starts from zero. Decisions made yesterday are forgotten. Domain knowledge lives in developers' heads. Specifications exist in separate documents that grow stale.

CWA solves this by creating a **living project knowledge base** that automatically stays synchronized with your development workflow:

- **Persistent Context** - Your domain model, decisions, and project state are always available to Claude through MCP, regardless of session
- **Structured Workflow** - Specs before code, tests before implementation, decisions recorded as they happen
- **Semantic Recall** - "Why did we choose Redis over Memcached?" is answered instantly via vector search, even months later
- **Impact Awareness** - The knowledge graph shows how changing a spec affects tasks, contexts, and related decisions
- **Token Efficiency** - CLAUDE.md is auto-generated with only relevant context, staying within token budgets

Without CWA, you repeat context in every prompt. With CWA, you describe your idea once and Claude orchestrates everything — domain modeling, specifications, task breakdown, and artifact generation — through MCP tools, automatically.

## How It Works

```
                    ┌───────────────────────────────┐
                    │    User Prompt to Claude       │
                    │    "I want to build a..."      │
                    └───────────────┬───────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │      Claude Code + MCP         │
                    │      (cwa mcp stdio)           │
                    └───────────────┬───────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
    Specifications            Domain Model              Tasks
    (with criteria)        (bounded contexts)       (Kanban board)
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    │
                         ┌──────────┼──────────┐
                         ▼          ▼          ▼
                       Redis      Neo4j     Qdrant
                     (primary    (graph    (vector
                     store +     queries)  search)
                     pub/sub)
                                    │
                                    ▼
                         CLAUDE.md + .claude/
                        (persistent context)
```

**You describe, Claude orchestrates**: your project idea enters as a natural language prompt. Claude Code uses MCP tools to create specs, model the domain, generate tasks, and produce artifacts. CWA stores everything in Redis, syncs relationships to Neo4j, and indexes semantics in Qdrant. All context persists across sessions.

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
- **Redis** (required — included in Docker Compose via `cwa infra up`)
- **Docker** (optional — required for Redis, Knowledge Graph, Embeddings, and Semantic Memory features)
- **ANTHROPIC_API_KEY** (optional — required for `cwa design from-image` command)

## Technology Choices

### Why Redis as Primary Store?

CWA v0.8.0 migrated from SQLite to Redis as the primary data store. The rationale:

- **Async-native** - Redis operations are non-blocking, matching Tokio's async runtime perfectly
- **Pub/Sub for WebSocket** - `PUBLISH`/`SUBSCRIBE` enables real-time board refresh when Claude Code updates tasks via MCP
- **Sorted sets for ordering** - Tasks, specs, and observations maintain insertion order using `ZADD`
- **Zero separate DB process** - Redis runs in Docker Compose alongside Neo4j and Qdrant; no SQLite file to manage
- **Key schema** - All data lives under `cwa:<project_id>:` prefix, making projects self-contained and portable

**Key schema:**
```
cwa:<project_id>:project      — Project metadata (HASH)
cwa:<project_id>:specs:all    — Spec ID set (ZSET ordered by time)
cwa:<project_id>:spec:<id>    — Spec data (HASH)
cwa:<project_id>:tasks:all    — Task ID set (ZSET ordered by time)
cwa:<project_id>:task:<id>    — Task data (HASH)
cwa:<project_id>:contexts:all — Context set (SET)
cwa:<project_id>:context:<id> — Context data (HASH)
```

Neo4j and Qdrant are **derived stores** — they're populated by syncing from Redis and can be rebuilt at any time.

### Why Ollama for Embeddings?

CWA uses [Ollama](https://ollama.ai) with the `nomic-embed-text` model (768 dimensions) for semantic embeddings:

- **Local-first** - Runs entirely on your machine, no API keys, no network latency for embeddings
- **Privacy** - Your project knowledge never leaves your infrastructure
- **Offline capable** - Works without internet after initial model pull
- **Cost** - Zero marginal cost per embedding, unlike cloud APIs

### Why Local LLM for Git Commits?

CWA uses `qwen2.5-coder:3b` via Ollama for generating commit messages:

- **Token savings** - Routine commits don't consume your Claude token budget
- **Speed** - Local inference is faster than API round-trips for simple tasks
- **Offline** - Works without internet after initial model pull

### Why Qdrant for Vector Storage?

- **Native vector operations** - Built from the ground up for similarity search
- **gRPC interface** - Fast binary protocol for high-throughput embedding operations
- **Filtering** - Supports payload filtering during search (e.g., search only "decision" type memories)
- **Self-hosted** - Runs locally via Docker, no cloud dependency

### Why Neo4j for Knowledge Graph?

- **Impact analysis** - "What tasks, specs, and decisions are affected if I change this bounded context?" is a single graph traversal
- **Cypher** - Expressive query language makes complex relationship queries readable
- **Path finding** - Discover indirect dependencies between entities
- **Visualization** - Neo4j Browser provides visual graph exploration at `http://localhost:7474`

### Why Askama for Web Templates?

- **Compile-time** - Templates are checked at build time, no runtime template parsing errors
- **Type-safe** - Template variables are Rust structs, impossible to pass wrong types
- **Zero overhead** - Compiled directly into the binary, no file I/O at runtime

## Getting Started

### 1. Install and Initialize

```bash
cwa init my-project
cd my-project
```

This creates:
- `.mcp.json` — MCP server configuration (auto-detected by Claude Code)
- `.claude/` — Agents, skills, commands, rules, and hooks
- `CLAUDE.md` — Project context file
- `.cwa/` — Project directory (config, scripts, docker)

### 2. Start Infrastructure

Redis is required. Start all services with:

```bash
cwa infra up
```

This starts Redis, Neo4j, Qdrant, and Ollama via Docker Compose.

### 3. Set Tech Stack

Configure your tech stack so `cwa codegen all` selects the right expert agents:

```bash
cwa stack set rust axum redis neo4j qdrant
```

This writes `.cwa/stack.json` and enables automatic tech-stack-aware agent selection.

### 4. Open in Claude Code

Open the project directory in Claude Code. The `.mcp.json` file is detected automatically, connecting the CWA MCP server with **39 tools** and **12 resources**.

### 5. Describe Your Project

Tell Claude Code what you want to build:

> "I want to build a recipe sharing app where users can create accounts,
> save recipes with ingredients and steps, search by ingredient, and
> leave ratings. Use Rust with Axum for the backend and Redis for storage."

### 6. Claude Code Does the Rest

Through MCP tools, Claude Code automatically:

1. **Discovers the domain** — Creates bounded contexts (Recipes, Users, Search, Ratings)
2. **Writes specifications** — Each feature gets acceptance criteria
3. **Generates tasks** — Specs are broken into implementable work items
4. **Populates the board** — Tasks appear on the Kanban board with WIP limits
5. **Records decisions** — Architectural choices are stored for future sessions
6. **Generates artifacts** — Expert agents, skills, and CLAUDE.md are created

You can verify the result:

```bash
cwa task board          # See populated Kanban board
cwa spec list           # See specifications with criteria
cwa domain context list # See bounded contexts
cwa stack show          # See tech agent templates
```

## CLI Reference

### Project Management

```bash
cwa init <name> [--from-prompt <prompt>]   # Initialize new project
cwa update                                  # Update project info interactively
cwa update --regenerate-only                # Only regenerate context files
cwa update --no-regen                       # Only save info, skip file regeneration
cwa context status                          # View current project focus
cwa context summary                         # View context summary
cwa clean [--confirm] [--infra]             # Clean project (start fresh)
```

### Tech Stack Configuration

```bash
cwa stack set <tech> [<tech2>...]          # Set tech stack (writes .cwa/stack.json)
cwa stack show                              # Show current stack + available agent templates
```

**Example:**
```bash
$ cwa stack set rust axum redis neo4j qdrant
✓ Tech stack saved to .cwa/stack.json
  Stack: rust, axum, redis, neo4j, qdrant

Run 'cwa codegen all' to regenerate agents for this stack.

$ cwa stack show
Tech Stack
────────────────────────────────────────
  • rust
  • axum
  • redis
  • neo4j
  • qdrant

→ 5 agent templates would be generated:
  .claude/agents/rust-expert.md
  .claude/agents/axum-expert.md
  .claude/agents/tokio-expert.md
  .claude/agents/redis-expert.md
  .claude/agents/neo4j-expert.md
```

### Specifications (SDD)

```bash
cwa spec new <title> [--description <desc>] [--priority <p>] [-c <criterion>]...
cwa spec from-prompt [<text>] [--file <path>] [--priority <p>] [--dry-run]
cwa spec add-criteria <spec> <criterion>...
cwa spec list
cwa spec get <id>                  # Get spec by ID (supports prefix)
cwa spec status [<spec>]
cwa spec validate <spec>
cwa spec archive <spec-id>
cwa spec clear [--confirm]
```

**Example:**
```bash
$ cwa spec new "User Authentication" --description "JWT-based auth" --priority high \
  -c "User can register with email and password" \
  -c "User can login with valid credentials" \
  -c "Session expires after 24 hours"
✓ Created spec: User Authentication (abc123)
  3 acceptance criteria added
```

### Tasks (Kanban)

```bash
cwa task new <title> [--description <d>] [--spec <id>] [--priority <p>]
cwa task list                      # List all tasks
cwa task generate <spec> [--status <s>] [--prefix <p>] [--dry-run]
cwa task move <task-id> <status>   # backlog|todo|in_progress|review|done
cwa task board                     # Display Kanban board
cwa task wip                       # Show WIP limits status
cwa task clear [<spec>] [--confirm] # Delete tasks (all or by spec)
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
cwa domain context new <name> [--description <d>]  # Create bounded context
cwa domain context list                # List contexts
cwa domain context map                 # Show context relationships
cwa domain glossary                    # Display domain glossary
```

### Memory (Semantic)

```bash
cwa memory add "<content>" -t <type>        # preference|decision|fact|pattern
cwa memory search "<query>" [--top-k N]     # Semantic search (default: 5 results)
cwa memory observe "<title>" -t <type>      # Record structured observation
cwa memory timeline [--days 7] [--limit 20] # Recent observations grouped by day
cwa memory compact [--min-confidence 0.3]   # Remove low-confidence entries
cwa memory sync                             # Sync CLAUDE.md with current state
cwa memory export [--output <file>]         # Export memory as JSON
```

### Knowledge Graph

```bash
cwa graph sync                                    # Full sync Redis -> Neo4j
cwa graph query "<cypher>"                        # Execute raw Cypher
cwa graph impact <entity-type> <entity-id>        # Impact analysis
cwa graph explore <entity-type> <entity-id>       # Neighborhood exploration
cwa graph status                                  # Graph statistics
```

### Code Generation

Generates Claude Code artifacts from your domain model and tech stack.

```bash
cwa codegen agent [context-id]     # Agent from bounded context
cwa codegen agent --all            # All agents
cwa codegen skill <spec-id>        # Skill from spec
cwa codegen hooks                  # Validation hooks (all 4 event types)
cwa codegen commands               # Claude Code slash commands (11)
cwa codegen claude-md              # Regenerate CLAUDE.md
cwa codegen all                    # Generate everything
# All commands support --dry-run
```

**Example:**
```bash
$ cwa stack set rust axum redis
$ cwa codegen all --dry-run
Generating all artifacts...
  3 tech agents (stack: rust, axum, redis): rust-expert.md, axum-expert.md, tokio-expert.md
  2 domain agents: recipes-expert.md, users-expert.md
  3 default skills: workflow-kickoff, refactor-safe, tdd-cycle
  11 commands: generate-tasks, run-backlog, project-status, next-task, spec-review, ...
  CLAUDE.md
  .mcp.json

(dry run - no files written)
```

### Token Analysis

```bash
cwa tokens analyze [path]          # Analyze single file
cwa tokens analyze --all           # Analyze all context files
cwa tokens optimize [--budget N]   # Suggest optimizations (default: 8000)
cwa tokens report                  # Full report with chart
```

### Infrastructure (Docker)

Manages Redis, Neo4j, Qdrant, and Ollama containers.

```bash
cwa infra up                       # Start all services + pull models
cwa infra down                     # Stop services (keep data)
cwa infra down --clean             # Stop + remove volumes + remove images
cwa infra status                   # Health check
cwa infra logs [service] [--follow]  # View logs
cwa infra reset --confirm          # Destroy all data + volumes

# Model management
cwa infra models                   # List installed Ollama models
cwa infra models pull <model>      # Pull a model
```

### Git Commands (Local LLM)

```bash
cwa git msg                        # Generate commit message (preview only)
cwa git commit                     # Generate message and commit
cwa git commit -a                  # Stage all changes + commit
cwa git commitpush                 # Commit and push
```

### Servers

```bash
cwa serve [--port <port>] [--host <host>]  # Start web server
cwa mcp stdio                              # Run standalone MCP server
cwa mcp planner                            # Run MCP planner server (Claude Desktop)
cwa mcp status                             # Show MCP configuration
cwa mcp install [target]                   # Install MCP server to target(s)
cwa mcp uninstall [target]                 # Remove MCP server from target(s)
```

**MCP Installation targets:**

| Target | Config Location |
|--------|-----------------|
| `claude-desktop` | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| `claude-code` | `~/.claude.json` |
| `gemini-cli` | `~/.gemini/settings.json` |
| `vscode` | `~/Library/Application Support/Code/User/mcp.json` |

## Claude Code Integration

CWA is designed as a **companion system for Claude Code**, providing persistent project intelligence across sessions. The integration works through three channels:

1. **MCP Server** - Real-time tools and resources Claude Code calls during sessions
2. **Generated Artifacts** - `.claude/` directory with agents, skills, commands, rules, and hooks
3. **CLAUDE.md** - Auto-generated context file loaded at session start

### MCP Configuration

Add to your `.mcp.json` (auto-generated by `cwa init`):

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

### Claude Desktop Integration (Planning)

For project planning before implementation, configure the planner in Claude Desktop:

```json
{
  "mcpServers": {
    "cwa-planner": {
      "command": "cwa",
      "args": ["mcp", "planner"]
    }
  }
}
```

The planner server exposes **40 tools** (39 CWA tools + `cwa_plan_software`) and **12 resources**, making it a full-featured MCP server with planning capabilities.

The `cwa_plan_software` tool uses DDD/SDD principles to generate a structured project plan with clarifying questions, bounded contexts, ubiquitous language, ADRs, specifications, and a single executable CLI bootstrap script.

### MCP Tools Reference (39 Tools + 1 Planner Tool)

#### Project & Context (6 tools)

| Tool | Description |
|------|-------------|
| `cwa_get_project_info` | Get project metadata (tech stack, features, constraints) |
| `cwa_get_context_summary` | Compact project state overview |
| `cwa_get_domain_model` | Bounded contexts, entities, invariants |
| `cwa_get_context_map` | Get DDD context map showing relationships |
| `cwa_get_tech_stack` | Get project tech stack for agent selection |
| `cwa_cache_status` | Redis connection and cache health status |

#### Specifications (6 tools)

| Tool | Description |
|------|-------------|
| `cwa_get_spec` | Get specification by ID or title |
| `cwa_list_specs` | List all specs (filterable by status) |
| `cwa_create_spec` | Create spec with acceptance criteria |
| `cwa_update_spec_status` | Update spec status (draft, active, completed, archived) |
| `cwa_add_acceptance_criteria` | Add criteria to existing spec |
| `cwa_validate_spec` | Validate spec for completeness |

#### Tasks & Kanban (7 tools)

| Tool | Description |
|------|-------------|
| `cwa_get_current_task` | Get current in-progress task |
| `cwa_list_tasks` | List all tasks (filterable by status/spec) |
| `cwa_create_task` | Create a new task |
| `cwa_update_task_status` | Move task through workflow (enforces WIP) |
| `cwa_generate_tasks` | Auto-create tasks from spec criteria |
| `cwa_get_wip_status` | Get WIP limits status for all columns |
| `cwa_set_wip_limit` | Set WIP limit for a Kanban column |

#### Memory & Observations (9 tools)

| Tool | Description |
|------|-------------|
| `cwa_search_memory` | Text-based memory search |
| `cwa_memory_semantic_search` | Vector similarity search (Qdrant) |
| `cwa_memory_search_all` | Unified search across all memory types |
| `cwa_memory_add` | Store memory with embedding |
| `cwa_observe` | Record structured observation |
| `cwa_memory_timeline` | Compact timeline (~50 tokens/entry) |
| `cwa_memory_get` | Full observation details (~500 tokens/entry) |
| `cwa_get_next_steps` | Suggested next actions based on state |
| `cwa_hybrid_search` | Combined vector + keyword search across all data |

#### Domain Modeling — DDD (4 tools)

| Tool | Description |
|------|-------------|
| `cwa_create_context` | Create a new bounded context |
| `cwa_create_domain_object` | Create domain object (entity, value object, aggregate, service, event) |
| `cwa_get_glossary` | Get domain glossary terms |
| `cwa_add_glossary_term` | Add term to domain glossary |

#### Decisions — ADRs (2 tools)

| Tool | Description |
|------|-------------|
| `cwa_add_decision` | Register architectural decision with rationale |
| `cwa_list_decisions` | List all architectural decisions |

#### Knowledge Graph — Neo4j (4 tools)

| Tool | Description |
|------|-------------|
| `cwa_graph_query` | Execute Cypher query on knowledge graph |
| `cwa_graph_impact` | Analyze impact of entity changes |
| `cwa_graph_sync` | Trigger Redis to Neo4j sync |
| `cwa_graph_hyperedges` | Find multi-entity relationship clusters |

#### Code Generation (1 tool)

| Tool | Description |
|------|-------------|
| `cwa_codegen_agents` | Trigger tech-stack-aware agent generation |

### MCP Resources (12 Resources)

| URI | Description |
|-----|-------------|
| `project://info` | Project metadata (name, tech stack, features, constraints) |
| `project://constitution` | Project values and constraints |
| `project://current-spec` | Currently active specification |
| `project://domain-model` | DDD model with bounded contexts |
| `project://kanban-board` | Current board state |
| `project://decisions` | Architectural decision log |
| `project://specs` | All specifications with status and criteria count |
| `project://tasks` | All tasks with current status |
| `project://glossary` | Domain glossary terms and definitions |
| `project://wip-status` | WIP limits and current counts per column |
| `project://context-map` | Context relationships (upstream/downstream) |
| `project://tech-stack` | Current tech stack and available agent templates |

### Generated Artifacts (`.claude/` Directory)

CWA generates a complete Claude Code configuration directory:

| Artifact | Source | Claude Code Feature |
|----------|--------|---------------------|
| `agents/*.md` | Bounded contexts + tech stack | [Agents](https://docs.anthropic.com/claude-code/agents) — domain expert personas |
| `skills/*/SKILL.md` | Approved specs + built-in (3) | [Skills](https://docs.anthropic.com/claude-code/skills) — repeatable workflows |
| `commands/*.md` | Built-in (11) | [Commands](https://docs.anthropic.com/claude-code/commands) — slash commands |
| `rules/*.md` | Built-in (5) | [Rules](https://docs.anthropic.com/claude-code/rules) — code constraints |
| `hooks.json` | Domain invariants + all event types | [Hooks](https://docs.anthropic.com/claude-code/hooks) — event-driven validation |
| `design-system.md` | UI screenshots | Design tokens for consistent UI |

#### Built-in Commands (11)

| Command | Purpose |
|---------|---------|
| `/generate-tasks` | Create tasks from spec acceptance criteria |
| `/run-backlog` | Plan and execute all tasks in the backlog |
| `/project-status` | Show specs, tasks, and domain model overview |
| `/next-task` | Pick and start next task with full CWA Kanban flow |
| `/spec-review` | Review specification for SDD completeness |
| `/domain-model` | Display complete domain model |
| `/observe` | Record a development observation into CWA memory |
| `/tech-stack` | View tech stack and available agent templates |
| `/kanban` | Display Kanban board and manage task flow |
| `/wip-check` | Verify WIP limits and flag violations |
| `/sync` | Sync to knowledge graph and regenerate CLAUDE.md |

#### Built-in Skills (3)

| Skill | Purpose |
|-------|---------|
| `workflow-kickoff` | Feature idea → spec → tasks → artifacts |
| `refactor-safe` | Safe refactoring with test coverage |
| `tdd-cycle` | Red-Green-Refactor TDD workflow |

#### Generated Hooks (`.claude/hooks.json`)

CWA generates hooks in the correct Claude Code object format with all 4 event types:

```json
{
  "hooks": {
    "PreToolUse": [
      { "matcher": "Bash", "hooks": [{"type": "command", "command": "...danger check..."}] }
    ],
    "PostToolUse": [
      { "matcher": "Write", "hooks": [{"type": "command", "command": "cwa memory observe..."}] },
      { "matcher": "Edit|MultiEdit", "hooks": [{"type": "command", "command": "cwa memory observe..."}] }
    ],
    "UserPromptSubmit": [
      { "matcher": "", "hooks": [{"type": "command", "command": "cwa context status 2>/dev/null || true"}] }
    ],
    "Stop": [
      { "matcher": "", "hooks": [{"type": "command", "command": "cwa task list --status in_progress 2>/dev/null || true"}] }
    ]
  }
}
```

Tech-stack-specific hooks are added automatically (e.g., `cargo fmt` for Rust, `prettier` for TypeScript, `black` for Python).

## Tech Stack Agent Templates

CWA includes **28 pre-built expert agent templates** organized by technology. Set your stack with `cwa stack set` and they're automatically selected during `cwa codegen all`.

### Template Categories

**Rust (7 templates):**
| Template | Expert In |
|----------|-----------|
| `rust-expert.md` | Ownership, lifetimes, idiomatic Rust |
| `axum-expert.md` | Axum routing, extractors, middleware |
| `tokio-expert.md` | Async runtime, tasks, channels |
| `ddd-expert.md` | DDD patterns in Rust |
| `redis-expert.md` | Redis data structures, pub/sub |
| `serde-expert.md` | Serialization, JSON, custom derive |
| `testing-expert.md` | Rust testing, mocks, property-based |

**Elixir/Phoenix (5 templates):**
| Template | Expert In |
|----------|-----------|
| `elixir-expert.md` | Functional Elixir, pattern matching |
| `phoenix-expert.md` | Phoenix controllers, contexts |
| `liveview-expert.md` | Phoenix LiveView, real-time UI |
| `ecto-expert.md` | Ecto schemas, queries, migrations |
| `otp-expert.md` | GenServer, Supervisor, OTP patterns |

**TypeScript/React (6 templates):**
| Template | Expert In |
|----------|-----------|
| `typescript-expert.md` | TypeScript, type safety, generics |
| `react-expert.md` | React 19, hooks, Server Components |
| `nextjs-expert.md` | Next.js App Router, RSC |
| `tailwind-expert.md` | Tailwind CSS v4, utility-first |
| `shadcn-expert.md` | shadcn/ui components |
| `prisma-expert.md` | Prisma ORM, migrations |

**Python (4 templates):**
| Template | Expert In |
|----------|-----------|
| `python-expert.md` | Python 3.12+, type hints, async |
| `fastapi-expert.md` | FastAPI, Pydantic, OpenAPI |
| `sqlalchemy-expert.md` | SQLAlchemy 2.0, async sessions |
| `langchain-expert.md` | LangChain, LLM integration |

**Common Infrastructure (6 templates):**
| Template | Expert In |
|----------|-----------|
| `neo4j-expert.md` | Cypher queries, graph modeling |
| `qdrant-expert.md` | Vector search, collections |
| `docker-expert.md` | Containers, Compose, optimization |
| `kubernetes-expert.md` | K8s deployments, services |
| `graphql-expert.md` | GraphQL schema, resolvers |
| `grpc-expert.md` | gRPC, protobuf, streaming |

### How Stack Selection Works

```bash
# 1. Set your stack
cwa stack set rust axum redis neo4j qdrant

# .cwa/stack.json is written:
# {"tech_stack": ["rust", "axum", "redis", "neo4j", "qdrant"]}

# 2. Preview which agents would be generated
cwa stack show
# → 5 agent templates: rust-expert, axum-expert, tokio-expert, redis-expert, neo4j-expert

# 3. Generate all artifacts
cwa codegen all
# .cwa/stack.json is read FIRST (before Redis), ensuring correct agents
# even without connectivity
```

## Web Dashboard

Start with `cwa serve` and open `http://localhost:3030`.

```bash
cwa serve
cwa serve --log         # With logging
```

**Real-time WebSocket Auto-refresh:** When Claude Code updates tasks via MCP tools (`cwa_update_task_status`), the web board updates automatically. The board connects to the `/ws` WebSocket endpoint and listens for `BoardRefresh` or `TaskUpdated` messages from the MCP server.

### REST API (`/api/*`)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/tasks` | List all tasks |
| POST | `/api/tasks` | Create a task |
| GET | `/api/board` | Get Kanban board with columns |
| GET | `/api/specs` | List specifications |
| GET | `/api/domains` | List bounded contexts |
| GET | `/api/context/summary` | Get context summary |

## Task Workflow

Tasks follow a strict workflow with WIP limits:

```
┌─────────┐    ┌──────┐    ┌─────────────┐    ┌────────┐    ┌──────┐
│ BACKLOG │ -> │ TODO │ -> │ IN_PROGRESS │ -> │ REVIEW │ -> │ DONE │
│   (∞)   │    │ (5)  │    │     (1)     │    │  (2)   │    │ (∞)  │
└─────────┘    └──────┘    └─────────────┘    └────────┘    └──────┘
```

## Docker Services

`cwa init` creates Docker infrastructure in `.cwa/docker/`:

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| Redis | `redis:7-alpine` | 6379 | Primary data store + pub/sub |
| Neo4j | `neo4j:5.26-community` | 7474, 7687 | Knowledge Graph |
| Qdrant | `qdrant/qdrant:v1.13.2` | 6333, 6334 | Vector Store |
| Ollama | `ollama/ollama:0.5.4` | 11434 | Embeddings + Text Generation |

**Ollama Models:**

| Model | Purpose | Size |
|-------|---------|------|
| `nomic-embed-text` | Embeddings (768 dims) | 274MB |
| `qwen2.5-coder:3b` | Commit message generation | 1.9GB |

## Project Structure

When you run `cwa init`, the following structure is created:

```
my-project/
├── .cwa/
│   ├── stack.json                # Tech stack config (cwa stack set)
│   ├── constitution.md           # Project values & constraints
│   ├── scripts/                  # Git helper scripts
│   │   ├── cwa-git-msg.sh        # Generate commit message via Ollama
│   │   ├── cwa-git-commit.sh     # Commit with generated message
│   │   └── cwa-git-commitpush.sh # Commit + push
│   └── docker/                   # Docker infrastructure
│       ├── docker-compose.yml    # Redis, Neo4j, Qdrant, Ollama
│       ├── .env.example          # Environment template
│       └── scripts/
│           ├── init-qdrant.sh    # Qdrant collection setup
│           └── init-neo4j.cypher # Neo4j constraints/indexes
├── .claude/
│   ├── agents/                   # Agent definitions
│   │   ├── analyst.md            # Requirements research
│   │   ├── architect.md          # DDD architecture
│   │   ├── specifier.md          # Spec-driven development
│   │   ├── implementer.md        # TDD implementation
│   │   ├── reviewer.md           # Code review
│   │   ├── orchestrator.md       # Workflow coordination
│   │   ├── tester.md             # Test generation (BDD)
│   │   ├── documenter.md         # Docs & ADR maintenance
│   │   └── [tech]-expert.md...   # Tech-stack agents (up to 28)
│   ├── commands/                  # Slash commands (11 commands)
│   │   ├── generate-tasks.md
│   │   ├── run-backlog.md
│   │   ├── project-status.md
│   │   ├── next-task.md
│   │   ├── spec-review.md
│   │   ├── domain-model.md
│   │   ├── observe.md
│   │   ├── tech-stack.md
│   │   ├── kanban.md
│   │   ├── wip-check.md
│   │   └── sync.md
│   ├── skills/                    # Skill definitions (3 built-in)
│   │   ├── workflow-kickoff/
│   │   ├── refactor-safe/
│   │   └── tdd-cycle/
│   ├── rules/                     # Code rules (5 rule files)
│   │   ├── api.md
│   │   ├── domain.md
│   │   ├── tests.md
│   │   ├── workflow.md
│   │   └── memory.md
│   └── hooks.json                 # All 4 event types + tech-stack hooks
├── .mcp.json                      # MCP server configuration
├── CLAUDE.md                      # Auto-generated project context
└── docs/
```

> **Note:** Redis connection defaults to `redis://127.0.0.1:6379`. Override with `REDIS_URL` environment variable.

## Crate Layout

```
cwa/
├── Cargo.toml                # Workspace manifest (v0.9.0)
├── crates/
│   ├── cwa-cli/              # CLI binary (clap)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/     # Command handlers (incl. stack.rs)
│   │       └── output.rs     # Terminal formatting
│   ├── cwa-core/             # Domain logic
│   │   └── src/
│   │       ├── project/      # Init, scaffold, config
│   │       ├── spec/         # SDD specs + parser
│   │       ├── domain/       # DDD contexts & objects
│   │       ├── task/         # Kanban task management
│   │       ├── board/        # Kanban board model
│   │       ├── decision/     # ADR management
│   │       └── memory/       # Memory entries
│   ├── cwa-db/               # Database abstraction (Redis)
│   │   └── src/
│   │       ├── lib.rs        # Re-exports + broadcast channel
│   │       ├── broadcast.rs  # WebSocketMessage types
│   │       └── queries/      # Delegates to cwa-redis
│   ├── cwa-redis/            # Redis persistence layer
│   │   └── src/
│   │       ├── client.rs     # Connection pool + error types
│   │       └── queries/      # CRUD for all domain types
│   ├── cwa-graph/            # Neo4j integration
│   │   └── src/
│   │       ├── client.rs
│   │       ├── schema.rs
│   │       ├── sync/         # Redis -> Neo4j sync pipeline
│   │       └── queries/      # Impact, explore, search
│   ├── cwa-embedding/        # Vector embeddings
│   │   └── src/
│   │       ├── ollama.rs
│   │       ├── qdrant.rs
│   │       ├── memory.rs
│   │       └── search.rs
│   ├── cwa-codegen/          # Artifact generation
│   │   └── src/
│   │       ├── agents.rs     # Agent .md from contexts
│   │       ├── skills.rs     # Skill .md from specs
│   │       ├── hooks.rs      # hooks.json (4 event types)
│   │       ├── commands.rs   # 11 slash command .md files
│   │       ├── tech_agents.rs # 28 tech-stack agent templates
│   │       └── claude_md.rs  # CLAUDE.md regeneration
│   │   └── templates/
│   │       └── agents/       # 28 .md agent template files
│   ├── cwa-token/            # Token analysis
│   ├── cwa-mcp/              # MCP server (39 tools, 12 resources)
│   │   └── src/
│   │       ├── server.rs     # JSON-RPC over stdio
│   │       └── planner_template.rs # DDD/SDD planning template
│   └── cwa-web/              # Web server
│       ├── src/
│       │   ├── routes/
│       │   ├── state.rs
│       │   └── websocket.rs  # BoardRefresh / TaskUpdated broadcast
│       └── templates/        # Askama HTML templates
└── docker/                   # Docker Compose infrastructure
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

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection string |
| `NEO4J_URI` | `bolt://127.0.0.1:7687` | Neo4j connection |
| `QDRANT_URL` | `http://127.0.0.1:6333` | Qdrant endpoint |
| `OLLAMA_URL` | `http://127.0.0.1:11434` | Ollama endpoint |
| `CWA_WEB_URL` | `http://127.0.0.1:3030` | Web server URL (for MCP notify) |
| `ANTHROPIC_API_KEY` | — | Required for `cwa design from-image` |

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a PR at [github.com/charlenopires/cwa](https://github.com/charlenopires/cwa).

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

## Why CWA?

AI-assisted development with tools like Claude Code faces a fundamental problem: **context fragmentation**. Every new session starts from zero. Decisions made yesterday are forgotten. Domain knowledge lives in developers' heads. Specifications exist in separate documents that grow stale.

CWA solves this by creating a **living project knowledge base** that automatically stays synchronized with your development workflow:

- **Persistent Context** - Your domain model, decisions, and project state are always available to Claude through MCP, regardless of session
- **Structured Workflow** - Specs before code, tests before implementation, decisions recorded as they happen
- **Semantic Recall** - "Why did we choose Redis over Memcached?" is answered instantly via vector search, even months later
- **Impact Awareness** - The knowledge graph shows how changing a spec affects tasks, contexts, and related decisions
- **Token Efficiency** - CLAUDE.md is auto-generated with only relevant context, staying within token budgets

Without CWA, you repeat context in every prompt. With CWA, Claude already knows your domain, your decisions, your current task, and what matters right now.

## How It Works

```
User Intent ─→ Specification ─→ Tasks ─→ Implementation ─→ Review ─→ Done
                     │              │            │               │
                     ▼              ▼            ▼               ▼
                  Domain         Kanban      Codegen         Decisions
                  Model          Board       Artifacts        (ADRs)
                     │              │            │               │
                     └──────────────┴────────────┴───────────────┘
                                         │
                              ┌──────────┼──────────┐
                              ▼          ▼          ▼
                           SQLite     Neo4j     Qdrant
                          (source    (graph    (vector
                          of truth)  queries)  search)
                                         │
                                         ▼
                                  CLAUDE.md + MCP
                                  (context for Claude)
```

**Data flows in one direction**: your domain knowledge enters through specs, tasks, and decisions. CWA stores it in SQLite, syncs relationships to Neo4j, and indexes semantics in Qdrant. Claude Code accesses everything through MCP tools or the auto-generated CLAUDE.md.

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

## Technology Choices

### Why Ollama for Embeddings?

CWA uses [Ollama](https://ollama.ai) with the `nomic-embed-text` model (768 dimensions) for semantic embeddings. The rationale:

- **Local-first** - Runs entirely on your machine, no API keys, no network latency for embeddings
- **Privacy** - Your project knowledge never leaves your infrastructure
- **Offline capable** - Works without internet after initial model pull
- **Cost** - Zero marginal cost per embedding, unlike cloud APIs
- **nomic-embed-text** - Small (274MB), fast, and scores well on retrieval benchmarks for its size

For teams needing cloud embeddings, the architecture allows swapping Ollama for OpenAI/Anthropic embedding APIs through the `cwa-embedding` crate.

### Why Qdrant for Vector Storage?

[Qdrant](https://qdrant.tech) is a purpose-built vector database chosen over alternatives (Pinecone, Weaviate, pgvector):

- **Native vector operations** - Built from the ground up for similarity search, not bolted onto a relational DB
- **gRPC interface** - Fast binary protocol for high-throughput embedding operations
- **Filtering** - Supports payload filtering during search (e.g., search only "decision" type memories)
- **Self-hosted** - Runs locally via Docker, no cloud dependency
- **Low resource** - Minimal memory footprint for the scale of project knowledge (hundreds to low thousands of vectors)

### Why Neo4j for Knowledge Graph?

[Neo4j](https://neo4j.com) enables relationship queries that are impractical in relational databases:

- **Impact analysis** - "What tasks, specs, and decisions are affected if I change this bounded context?" is a single graph traversal
- **Cypher** - Expressive query language makes complex relationship queries readable
- **Path finding** - Discover indirect dependencies between entities
- **Visualization** - Neo4j Browser provides visual graph exploration at `http://localhost:7474`
- **APOC plugins** - Extended algorithms for community detection, centrality, and pattern matching

### Why SQLite as Source of Truth?

- **Zero configuration** - No server to install or manage
- **Single file** - `.cwa/cwa.db` is portable and easy to back up
- **Fast** - Local file I/O is faster than network calls for CLI interactions
- **Reliable** - ACID transactions, well-tested, used by billions of devices
- **Schema migrations** - Numbered SQL migrations managed by `cwa-db`

Neo4j and Qdrant are **derived stores** - they're populated by syncing from SQLite and can be rebuilt at any time.

### Why Askama for Web Templates?

- **Compile-time** - Templates are checked at build time, no runtime template parsing errors
- **Type-safe** - Template variables are Rust structs, impossible to pass wrong types
- **Zero overhead** - Compiled directly into the binary, no file I/O at runtime
- **Familiar syntax** - Jinja2-like syntax for developers coming from Python/JS ecosystems

### Why tiktoken-rs for Token Counting?

- **Accurate** - Uses the same tokenizer (cl100k_base) as Claude and GPT models
- **Fast** - Native Rust implementation, counts thousands of tokens in microseconds
- **Budget management** - Critical for keeping CLAUDE.md and context files within model limits

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
cwa spec from-prompt [<text>] [--file <path>] [--priority <p>] [--dry-run]
cwa spec list
cwa spec status [<spec>]
cwa spec validate <spec>
cwa spec archive <spec-id>
```

**Example: Creating a single spec:**
```bash
$ cwa spec new "User Authentication" --description "JWT-based auth" --priority high
✓ Spec created: abc123
```

**Example: Creating multiple specs from a long prompt:**
```bash
# From inline text (numbered list)
$ cwa spec from-prompt "1. User registration with email validation
2. OAuth2 integration with Google and GitHub
3. Password reset via email
4. Session management with refresh tokens"
✓ Created 4 spec(s):

  1. User registration with email validation (a1b2c3)
  2. OAuth2 integration with Google and GitHub (d4e5f6)
  3. Password reset via email (g7h8i9)
  4. Session management with refresh tokens (j0k1l2)

# From a file
$ cwa spec from-prompt --file features.md --priority high

# Preview without creating
$ cwa spec from-prompt --dry-run "- Feature A\n- Feature B\n- Feature C"

# From stdin (pipe)
$ cat requirements.txt | cwa spec from-prompt
```

**Supported formats for `from-prompt`:**
- Numbered lists: `1. Item`, `2. Item`
- Bullet points: `- Item` or `* Item`
- Markdown headings: `# Title` with body text
- Paragraphs separated by blank lines

```bash
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
cwa graph sync                                    # Full sync SQLite -> Neo4j
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

$ cwa graph impact spec abc123
Impact analysis for spec abc123
──────────────────────────────────────────────
  -> [Task] Implement login endpoint (IMPLEMENTS)
  -> [Task] Add JWT validation (IMPLEMENTS)
  -> [BoundedContext] Authentication (BELONGS_TO)
  -> [Decision] Use RS256 algorithm (RELATES_TO)

4 related entities found.
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
✓ All artifacts generated.
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
Suggestions:
  1. [HIGH] ~400 tokens: Trim verbose descriptions in CLAUDE.md
  2. [MED]  ~250 tokens: Consolidate similar agent sections
  3. [LOW]  ~180 tokens: Remove redundant comments
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
  neo4j ... healthy
  qdrant ... healthy
  ollama ... healthy
  Pulling nomic-embed-text... done

Infrastructure ready.
  Neo4j Browser: http://localhost:7474
  Qdrant API:    http://localhost:6333
  Ollama API:    http://localhost:11434
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
│   ├── cwa.db                    # SQLite database
│   └── constitution.md           # Project values & constraints
├── .claude/
│   ├── agents/                   # Agent definitions (8 agents)
│   │   ├── analyst.md            # Requirements research
│   │   ├── architect.md          # DDD architecture
│   │   ├── specifier.md          # Spec-driven development
│   │   ├── implementer.md        # TDD implementation
│   │   ├── reviewer.md           # Code review
│   │   ├── orchestrator.md       # Workflow coordination
│   │   ├── tester.md             # Test generation (BDD)
│   │   └── documenter.md         # Docs & ADR maintenance
│   ├── commands/                  # Slash commands (8 commands)
│   │   ├── create-spec.md        # Create specification workflow
│   │   ├── implement-task.md     # Task implementation workflow
│   │   ├── session-summary.md    # Session summary generation
│   │   ├── next-task.md          # Pick and start next task
│   │   ├── review-code.md        # Review against spec criteria
│   │   ├── domain-discover.md    # Domain discovery workflow
│   │   ├── sync-context.md       # Regenerate all artifacts
│   │   └── status.md             # Full project status
│   ├── skills/                    # Skill definitions (2 built-in)
│   │   ├── workflow-kickoff/     # Feature idea -> full workflow
│   │   │   └── SKILL.md
│   │   └── refactor-safe/        # Safe refactoring with tests
│   │       └── SKILL.md
│   ├── rules/                     # Code rules (5 rule files)
│   │   ├── api.md                # API patterns & security
│   │   ├── domain.md             # DDD principles
│   │   ├── tests.md              # Test structure & coverage
│   │   ├── workflow.md           # CWA workflow enforcement
│   │   └── memory.md            # When to record memories
│   └── hooks.json                 # Validation hooks
├── .mcp.json                      # MCP server configuration
├── CLAUDE.md                      # Auto-generated project context
└── docs/                          # Documentation directory
```

## Use Cases

### Use Case 1: SaaS Startup MVP

**Scenario:** Two developers building a subscription billing platform. They work in different timezones and need Claude Code to maintain context across sessions.

```bash
# Day 1: Project setup
cwa init billing-platform
cd billing-platform
cwa infra up

# Domain discovery
cwa domain context new "Subscriptions" --description "Plan management and billing cycles"
cwa domain context new "Payments" --description "Payment processing via Stripe"
cwa domain context new "Accounts" --description "User accounts and team management"

# Create initial specs
cwa spec from-prompt "1. User can sign up and create a team
2. Team owner can select a subscription plan (free, pro, enterprise)
3. System processes monthly payments via Stripe
4. Users receive email receipts after successful payment
5. Failed payments retry 3 times with exponential backoff" --priority high

# Day 2: Developer A starts working
cwa task board
cwa task move <signup-task-id> in_progress
# Claude reads the spec via MCP, implements with full context

# Day 3: Developer B picks up where A left off
cwa context status
# Claude instantly knows: current spec, completed tasks, decisions made
cwa memory search "payment retry"
# Finds: "Using exponential backoff: 1s, 4s, 16s delays"

# Week 2: Impact analysis before a change
cwa graph impact context subscriptions-ctx
# Shows: 3 specs, 8 tasks, 2 decisions affected
# Developer makes informed decision about the change scope
```

**What CWA provides:** Shared context between developers and sessions. Claude Code knows the domain model, pending work, and past decisions without re-explanation.

### Use Case 2: Open Source Library Migration (v1 to v2)

**Scenario:** A library maintainer migrating from v1 to v2 with breaking changes. The migration spans 3 months with many design decisions.

```bash
cwa init my-lib-v2
cd my-lib-v2

# Define breaking changes as specs
cwa spec from-prompt --file migration-plan.md --priority critical
# Parses: "## New async API\n## Remove deprecated methods\n## New error types"

# Record design decisions as they happen
cwa memory add "Chose thiserror over anyhow for public error types - users need to match on variants" --type decision
cwa memory add "All public async fns return impl Future, not boxed - zero-cost for callers" --type decision
cwa memory add "Migration guide: provide From impls for old types -> new types" --type pattern

# Months later: "Why did we do X?"
cwa memory search "error types"
# Instantly finds the decision with rationale

# Track what's done vs. remaining
cwa task board
# See at a glance: 12/20 migration tasks done

# Before releasing: verify all specs are validated
cwa spec list
# Check: all specs must be "validated" status before v2 release

# Generate migration guide context
cwa codegen claude-md
# CLAUDE.md now includes all decisions, making it trivial for Claude
# to help write migration documentation
```

**What CWA provides:** Decision history across months. When you return after a break, `cwa memory search` and the knowledge graph restore your mental model instantly.

### Use Case 3: Solo Developer Side Project

**Scenario:** A developer working on a weekend project. They can only spend a few hours per week and often forget context between sessions.

```bash
# Weekend 1: Bootstrap
cwa init recipe-app
cd recipe-app

# Lightweight usage - no Docker needed for basic features
cwa spec new "Recipe CRUD" --description "Create, view, edit, delete recipes" --priority high
cwa spec new "Ingredient search" --description "Search recipes by ingredients" --priority medium

cwa task new "Set up SQLite schema" --priority high
cwa task new "Recipe list endpoint" --priority high
cwa task new "Recipe detail page" --priority medium
cwa task board

# Weekend 2 (2 weeks later): "Where was I?"
cwa context status
# Output:
#   Active Spec: Recipe CRUD
#   Current Task: Recipe list endpoint (in_progress)
#   Done: 1/3 tasks
#
# Claude immediately knows the full context

# Weekend 3: Decisions pile up
cwa memory add "Using askama templates, not Tera - compile-time checking" --type decision
cwa memory add "Recipe model has: title, ingredients (JSON), steps (JSON), cook_time" --type fact

# Weekend 5: "What did I decide about the data model?"
cwa memory search "recipe model"
# Instant recall without re-reading code

# Token-efficient: check context size
cwa tokens analyze --all
# Only 1200 tokens - well within budget for a small project
```

**What CWA provides:** Lightweight context persistence for sporadic work. No Docker required for basic features. SQLite handles everything locally.

### Use Case 4: Feature Development with Full Workflow

**Scenario:** Adding a notification system to an existing project, demonstrating the complete CWA workflow.

```bash
# Step 1: Specification
cwa spec new "User Notifications" \
  --description "Real-time and email notifications for account events" \
  --priority high

# Step 2: Domain Discovery
cwa domain context new "Notifications" \
  --description "Event-driven user notifications (in-app, email, push)"
cwa graph sync

# Step 3: Task Breakdown
cwa task new "Define notification events enum" --spec <spec-id> --priority high
cwa task new "Create notification store (SQLite)" --spec <spec-id> --priority high
cwa task new "Implement WebSocket delivery" --spec <spec-id> --priority medium
cwa task new "Add email delivery via SMTP" --spec <spec-id> --priority medium
cwa task new "Build notification preferences UI" --spec <spec-id> --priority low

# Step 4: Start Implementation
cwa task move <first-task-id> todo
cwa task move <first-task-id> in_progress
# Claude's orchestrator agent takes over:
#   - Reads the spec via MCP
#   - Delegates to tester agent (writes tests first)
#   - Delegates to implementer agent (implements to pass tests)
#   - Runs review-code command

# Step 5: Record Decisions
cwa memory add "Using SSE instead of WebSocket for notifications - simpler, sufficient for one-way" --type decision

# Step 6: Impact Analysis
cwa graph impact context notifications-ctx
# Shows this context has no upstream dependencies yet - safe to build independently

# Step 7: Generate Artifacts
cwa codegen all
# Creates notifications-expert agent, notification skill, updated CLAUDE.md

# Step 8: Verify Token Budget
cwa tokens optimize --budget 8000
# Ensures generated context stays within Claude's effective window
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
│   │       ├── project/      # Init, scaffold, config
│   │       ├── spec/         # SDD specs + parser
│   │       ├── domain/       # DDD contexts & objects
│   │       ├── task/         # Kanban task management
│   │       ├── board/        # Kanban board model
│   │       ├── decision/     # ADR management
│   │       └── memory/       # Memory entries
│   ├── cwa-db/               # SQLite persistence
│   │   └── src/
│   │       ├── pool.rs
│   │       ├── migrations/   # Numbered SQL migrations
│   │       └── queries/      # CRUD query modules
│   ├── cwa-graph/            # Neo4j integration
│   │   └── src/
│   │       ├── client.rs     # Connection pool
│   │       ├── schema.rs     # Constraints/indexes
│   │       ├── sync/         # SQLite -> Neo4j sync pipeline
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

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a PR at [github.com/charlenopires/cwa](https://github.com/charlenopires/cwa).

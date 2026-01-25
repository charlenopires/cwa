# CWA - Claude Workflow Architect

A Rust CLI tool for development workflow orchestration integrated with Claude Code. CWA bridges the gap between project management, domain modeling, and AI-assisted development by providing structured context that Claude Code can leverage through MCP.

## Features

- **Spec Driven Development (SDD)** - Manage specifications with status tracking, acceptance criteria, and validation
- **Domain Driven Design (DDD)** - Model bounded contexts, entities, value objects, aggregates, and domain glossary
- **Kanban Board** - Task management with WIP limits and workflow enforcement (CLI + web)
- **Knowledge Graph** - Neo4j-backed entity relationships, impact analysis, and exploration
- **Semantic Memory** - Vector embeddings via Ollama + Qdrant for intelligent context recall
- **Design System Extraction** - Analyze UI screenshots via Claude Vision API to generate design tokens (colors, typography, spacing, components)
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
                      SQLite     Neo4j     Qdrant
                     (source    (graph    (vector
                     of truth)  queries)  search)
                                    │
                                    ▼
                         CLAUDE.md + .claude/
                        (persistent context)
```

**You describe, Claude orchestrates**: your project idea enters as a natural language prompt. Claude Code uses MCP tools to create specs, model the domain, generate tasks, and produce artifacts. CWA stores everything in SQLite, syncs relationships to Neo4j, and indexes semantics in Qdrant. All context persists across sessions.

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
- **ANTHROPIC_API_KEY** (optional - required for `cwa design from-image` command)

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
- `.cwa/cwa.db` — SQLite database
- `.cwa/docker/` — Docker Compose infrastructure (Neo4j, Qdrant, Ollama)

### 2. Open in Claude Code

Open the project directory in Claude Code. The `.mcp.json` file is detected automatically, connecting the CWA MCP server with 18 tools and 5 resources.

### 3. Describe Your Project

Tell Claude Code what you want to build:

> "I want to build a recipe sharing app where users can create accounts,
> save recipes with ingredients and steps, search by ingredient, and
> leave ratings. Use Rust with Axum for the backend and SQLite for storage."

### 4. Claude Code Does the Rest

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
```

### 5. Start Building

In the same or a future Claude Code session, say:

> "What should I work on next?"

Claude Code reads the project state via MCP, picks a task respecting WIP limits, loads the relevant spec, and begins implementation with TDD.

### With Knowledge Graph & Semantic Memory (Optional)

```bash
# Start Docker infrastructure for graph and embedding features
cwa infra up

# Check services are healthy
cwa infra status

# Sync to Knowledge Graph
cwa graph sync

# Semantic search
cwa memory search "authentication tokens"
```

## Two Approaches

CWA supports two complementary workflows:

### Prompt-First (Recommended)

Describe what you want in natural language. Claude Code uses MCP tools to run the correct CWA commands in the right order. Best for:
- Starting new projects
- Adding features
- Day-to-day development

### CLI-Direct

Run `cwa` commands manually for full control. Best for:
- Fine-tuning specs or tasks
- CI/CD scripts
- Debugging project state
- Learning what CWA does under the hood

All CLI commands are documented in the [CLI Reference](#cli-reference) section below.

## CLI Reference

### Project Management

```bash
cwa init <name> [--from-prompt <prompt>]   # Initialize new project
cwa context status                          # View current project focus
cwa context summary                         # View context summary
cwa clean [--confirm] [--infra]             # Clean project (start fresh)
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
cwa spec new <title> [--description <desc>] [--priority <p>] [-c <criterion>]...
cwa spec from-prompt [<text>] [--file <path>] [--priority <p>] [--dry-run]
cwa spec add-criteria <spec> <criterion>...
cwa spec list
cwa spec status [<spec>]
cwa spec validate <spec>
cwa spec archive <spec-id>
cwa spec clear [--confirm]
```

**Example: Creating a spec with acceptance criteria:**
```bash
$ cwa spec new "User Authentication" --description "JWT-based auth" --priority high \
  -c "User can register with email and password" \
  -c "User can login with valid credentials" \
  -c "Session expires after 24 hours"
✓ Created spec: User Authentication (abc123)
  3 acceptance criteria added
```

**Example: Adding criteria to an existing spec:**
```bash
$ cwa spec add-criteria abc123 "User can reset password" "User can enable 2FA"
✓ Added 2 criteria to spec 'User Authentication' (total: 5)
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

**Example: Clearing all specs:**
```bash
# Preview (requires confirmation)
$ cwa spec clear
! This will permanently delete 2 spec(s). Run with --confirm to confirm.

# Execute
$ cwa spec clear --confirm
✓ Cleared 2 spec(s).
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

**Example: Generating tasks from spec criteria:**
```bash
# Preview what would be generated
$ cwa task generate abc123 --dry-run
⊙ Would create 3 task(s) for spec 'User Authentication':

  1. User can register with email and password [high]
  2. User can login with valid credentials [high]
  3. Session expires after 24 hours [high]

# Generate the tasks
$ cwa task generate abc123
✓ Generated 3 task(s) for spec 'User Authentication':

  1. User can register with email and password (task-001)
  2. User can login with valid credentials (task-002)
  3. Session expires after 24 hours (task-003)

# Running again skips existing tasks
$ cwa task generate abc123
⊙ All 3 criteria already have tasks. Nothing to generate.

# With a title prefix
$ cwa task generate abc123 --prefix "Auth"
✓ Generated 3 task(s) for spec 'User Authentication':

  1. Auth: User can register with email and password (task-004)
  ...
```

**Example: Clearing tasks:**
```bash
# Clear all tasks
$ cwa task clear
! This will permanently delete 5 task(s). Run with --confirm to confirm.

$ cwa task clear --confirm
✓ Cleared 5 task(s).

# Clear tasks for a specific spec
$ cwa task clear abc123
! This will permanently delete 3 task(s) for spec 'User Authentication'. Run with --confirm to confirm.

$ cwa task clear abc123 --confirm
✓ Cleared 3 task(s) for spec 'User Authentication'.
```

### Domain Modeling (DDD)

```bash
cwa domain discover                    # Interactive domain discovery
cwa domain context new <name> [--description <d>]  # Create bounded context
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
cwa memory compact --decay 0.98            # Decay all observation confidences
cwa memory sync                             # Sync CLAUDE.md with current state
cwa memory export [--output <file>]         # Export memory as JSON
```

### Observations (Structured Memory)

Observations capture structured development activity with confidence lifecycle, progressive disclosure via MCP, and automatic CLAUDE.md injection.

```bash
# Record observations
cwa memory observe "<title>" -t <type>      # bugfix|feature|refactor|discovery|decision|change|insight
cwa memory observe "Fixed auth token refresh" -t bugfix -f "Token expiring before refresh window"
cwa memory observe "Use Redis for sessions" -t decision -n "Lower latency than DB queries" --files-modified src/session.rs

# View timeline
cwa memory timeline [--days 7] [--limit 20] # Recent observations grouped by day

# Generate summaries
cwa memory summarize [--count 10]           # Compress recent observations into summary
```

**Example:**
```bash
$ cwa memory add "Team prefers functional patterns over OOP" --type preference
✓ Memory added (id: a1b2c3d4, embedding: 768 dims)

$ cwa memory observe "Fixed auth token refresh" -t bugfix -f "Token was expiring before refresh window"
✓ Observation recorded (id: b2c3d4e5, embedding: 768 dims)
  • [bugfix] Fixed auth token refresh
    → Token was expiring before refresh window

$ cwa memory timeline --days 3
→ Observations (last 3 days):

  2024-01-15
    [BUGFIX] Fixed auth token refresh 80% (b2c3d4e5)
    [DECISION] Use Redis for session cache 80% (c3d4e5f6)

$ cwa memory search "coding style"
✓ Found 2 results:

  1. [preference] Team prefers functional patterns over OOP (92%)
  2. [fact] Codebase uses Rust with trait-based composition (78%)

$ cwa memory compact --decay 0.98 --min-confidence 0.3
✓ Decayed 15 observation confidences by factor 0.98
✓ Removed 2 low-confidence observations
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

### Design System

Extract a complete design system from a UI screenshot using Claude Vision API. The extracted tokens are stored in memory, synced to the knowledge graph, and written to `.claude/design-system.md` for reference by Claude Code agents.

```bash
cwa design from-image <url>              # Extract design system from screenshot
    --model <model>                       # Claude model (default: claude-sonnet-4-20250514)
    --dry-run                             # Preview without saving
```

**Requires:** `ANTHROPIC_API_KEY` environment variable.

**Example:**
```bash
$ cwa design from-image https://example.com/app-screenshot.png
→ Analyzing image: https://example.com/app-screenshot.png
✓ Stored design system (id: a1b2c3d4)
✓ Generated: .claude/design-system.md
✓ Stored embedding (768 dims)
✓ Graph synced (1 nodes, 1 relationships)

Design system ready.
  Reference: .claude/design-system.md
```

The generated `.claude/design-system.md` includes:
- CSS custom properties with all design tokens
- Color palette (primary, secondary, neutral, semantic)
- Typography scale and font families
- Spacing, border-radius, and shadow tokens
- Breakpoints
- Identified UI components with variants and states

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
cwa infra down                     # Stop services (keep data)
cwa infra down --clean             # Stop + remove volumes + remove images
cwa infra status                   # Health check
cwa infra logs [service] [--follow]  # View logs
cwa infra reset --confirm          # Destroy all data + volumes
```

### Project Cleanup

```bash
cwa clean                          # Preview what will be removed
cwa clean --confirm                # Remove .cwa/, .claude/, CLAUDE.md, .mcp.json
cwa clean --confirm --infra        # Also remove Docker infrastructure
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

### Analysis

```bash
cwa analyze competitors <domain>   # Analyze competitors in a domain
cwa analyze features <competitor>  # Analyze features of a competitor
cwa analyze market <niche>         # Analyze a market segment
```

### Servers

```bash
cwa serve [--port <port>] [--host <host>]  # Start web server (default: 127.0.0.1:3030)
cwa mcp stdio                              # Run MCP server over stdio
cwa mcp planner                            # Run MCP planner server (Claude Desktop)
cwa mcp status                             # Show MCP configuration examples
```

## Claude Code Integration

CWA is designed as a **companion system for Claude Code**, providing persistent project intelligence across sessions. The integration works through three channels:

1. **MCP Server** - Real-time tools and resources Claude Code calls during sessions
2. **Generated Artifacts** - `.claude/` directory with agents, skills, commands, rules, and hooks
3. **CLAUDE.md** - Auto-generated context file loaded at session start

### How Claude Code Uses CWA in Each Phase

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Development Lifecycle                             │
├──────────┬──────────┬──────────────┬──────────┬────────────────────────┤
│ Planning │  Design  │Implementation│  Review  │   Memory & Learning    │
│          │          │              │          │                        │
│ Specs    │ Domain   │ Agents       │ Hooks    │ Observations           │
│ from-    │ Contexts │ Skills       │ Spec     │ Semantic Search        │
│ prompt   │ Glossary │ Commands     │ Criteria │ Decision Records       │
│          │ Graph    │ Rules        │ WIP      │ Timeline               │
│          │          │              │ Limits   │ Summaries              │
├──────────┴──────────┴──────────────┴──────────┴────────────────────────┤
│                              ↕ MCP ↕                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                    Claude Code Session                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Phase 1: Planning & Specification

Claude Code reads the project state and helps define what to build.

```
Claude Code                              CWA
    │                                     │
    ├── reads project://current-spec ────→│ "What are we building?"
    ├── reads project://domain-model ────→│ "What's the domain?"
    ├── calls cwa_get_context_summary ───→│ "What's the current state?"
    ├── calls cwa_search_memory ─────────→│ "Any past decisions on this?"
    │                                     │
    ├── User describes feature ──────────→│
    ├── calls cwa spec new / from-prompt →│ Creates spec with criteria
    ├── calls cwa_generate_tasks ────────→│ Auto-creates tasks from criteria
    └── calls cwa_add_decision ──────────→│ Records "why" for future sessions
```

**What Claude Code gains**: Full project context without re-explanation. Past decisions, domain model, and current work state are immediately available.

#### Phase 2: Domain Modeling

Claude Code helps discover and refine the domain model, which drives code generation.

```
Claude Code                              CWA
    │                                     │
    ├── calls cwa_get_domain_model ──────→│ Load current contexts
    ├── creates bounded contexts ────────→│ domain context new
    ├── defines entities/invariants ─────→│ Stored in SQLite
    ├── calls cwa_graph_sync ────────────→│ Syncs to Neo4j
    └── calls cwa_graph_impact ──────────→│ "What does changing this affect?"
```

**What Claude Code gains**: Bounded contexts become generated expert agents. Invariants become validation hooks. The glossary enforces ubiquitous language.

#### Phase 3: Implementation

Claude Code uses the generated agents, skills, and task context to write code.

```
Claude Code                              CWA
    │                                     │
    ├── /project:next-task ──────────────→│ Picks task, checks WIP limits
    ├── calls cwa_get_current_task ──────→│ Loads task details
    ├── calls cwa_get_spec ──────────────→│ Loads acceptance criteria
    │                                     │
    │  ┌─ Agents active: ─────────────────┤
    │  │  implementer.md (TDD flow)       │ Reads spec before coding
    │  │  tester.md (BDD from criteria)   │ Generates tests first
    │  │  [context]-expert.md             │ Domain-specific guidance
    │  └──────────────────────────────────┤
    │                                     │
    │  ┌─ Rules enforced: ────────────────┤
    │  │  workflow.md (spec before code)  │ Prevents skipping phases
    │  │  domain.md (DDD principles)     │ Ubiquitous language
    │  │  tests.md (coverage gates)      │ Test requirements
    │  └──────────────────────────────────┤
    │                                     │
    ├── calls cwa_observe ───────────────→│ Records bugfix/feature/discovery
    ├── calls cwa_update_task_status ────→│ Moves task to review
    └── calls cwa_memory_add ────────────→│ Stores patterns/decisions
```

**What Claude Code gains**: Structured workflow enforcement. The implementer agent reads the spec, the tester agent writes tests from acceptance criteria, and the orchestrator ensures WIP limits are respected.

#### Phase 4: Review & Validation

Claude Code validates work against spec criteria and domain invariants.

```
Claude Code                              CWA
    │                                     │
    ├── /project:review-code ────────────→│ Triggers review workflow
    ├── calls cwa_get_spec ──────────────→│ Loads acceptance criteria
    │                                     │
    │  ┌─ Hooks fire: ───────────────────┤
    │  │  pre-commit: wip check          │ Enforces kanban limits
    │  │  pre-commit: invariant check    │ Domain rule validation
    │  │  post-test: task advancement    │ Reminds to move task
    │  └──────────────────────────────────┤
    │                                     │
    ├── reviewer.md validates criteria ──→│ Each criterion checked
    └── calls cwa_update_task_status ────→│ done (or back to in_progress)
```

**What Claude Code gains**: Automated validation against acceptance criteria. Hooks prevent bypassing workflow rules. The reviewer agent knows exactly what to check.

#### Phase 5: Memory & Continuous Learning

Claude Code builds persistent project memory that survives across sessions.

```
Claude Code                              CWA
    │                                     │
    ├── calls cwa_observe ───────────────→│ Structured: bugfix/feature/decision/...
    ├── calls cwa_memory_add ────────────→│ Facts, preferences, patterns
    ├── calls cwa_add_decision ──────────→│ ADRs with rationale
    │                                     │
    │  Next session starts:               │
    ├── reads CLAUDE.md ─────────────────→│ Recent observations (0.7+ confidence)
    ├── calls cwa_memory_timeline ───────→│ Compact timeline (~50 tok/entry)
    ├── calls cwa_memory_get ────────────→│ Full details (~500 tok/entry)
    └── calls cwa_memory_semantic_search →│ "Why did we choose X?"
```

**What Claude Code gains**: Progressive disclosure memory. Timeline gives a quick overview; full details are loaded on demand. Confidence decay automatically deprecates stale knowledge. Summaries compress old observations.

#### Phase 6: Context Regeneration

CWA keeps all artifacts in sync and within token budget.

```
Claude Code                              CWA
    │                                     │
    ├── /project:sync-context ───────────→│ Triggers full regeneration
    │                                     │
    │  CWA regenerates:                   │
    │  ├── CLAUDE.md ─────────────────────│ Domain, specs, decisions, work state
    │  ├── agents/ ───────────────────────│ Expert agent per context
    │  ├── skills/ ───────────────────────│ Skill per approved spec
    │  ├── hooks.json ────────────────────│ Invariants as validation hooks
    │  └── design-system.md ──────────────│ Design tokens from screenshots
    │                                     │
    ├── calls cwa tokens optimize ───────→│ Ensures within budget
    └── calls cwa_graph_sync ────────────→│ Updates knowledge graph
```

**What Claude Code gains**: All generated artifacts stay current with the domain model. Token optimization ensures context fits within the model window.

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

The `cwa_plan_software` tool takes your idea and generates a structured planning document with:
1. Clarifying questions for Claude to ask
2. Bounded contexts (DDD)
3. Specifications with acceptance criteria
4. Domain model with entities and invariants
5. Architectural decisions
6. Task breakdown
7. **Ordered CLI bootstrap commands** ready to execute

**Example flow:**

> You: "I want to build a recipe app with user accounts, recipe sharing, and ratings"

Claude Desktop asks clarifying questions about tech stack, scale, and auth method, then generates a planning document with concrete commands:

```bash
cwa init "recipe-app"
cwa domain context new "Recipes" --description "Recipe management and search"
cwa domain context new "Users" --description "User accounts and profiles"
cwa spec new "User Registration" -c "User can sign up with email" -c "Email verification required"
cwa spec new "Recipe CRUD" -c "User can create recipe" -c "User can search by ingredient"
cwa task generate "User Registration"
cwa task generate "Recipe CRUD"
cwa codegen all
```

Take these commands to Claude Code for execution, or let Claude Code call the MCP tools (`cwa_create_context`, `cwa_create_spec`, `cwa_generate_tasks`) directly.

### MCP Tools Reference

| Tool | Phase | Description |
|------|-------|-------------|
| `cwa_create_context` | Planning, Design | Create a new bounded context |
| `cwa_create_spec` | Planning | Create a spec with acceptance criteria |
| `cwa_create_task` | Planning | Create a new task |
| `cwa_get_current_task` | Implementation | Get the current in-progress task |
| `cwa_get_spec` | Planning, Implementation, Review | Get specification with acceptance criteria |
| `cwa_get_context_summary` | Planning | Compact project state overview |
| `cwa_get_domain_model` | Design | Bounded contexts, entities, invariants |
| `cwa_update_task_status` | Implementation, Review | Move task through workflow (enforces WIP) |
| `cwa_add_decision` | All phases | Record architectural decision with rationale |
| `cwa_get_next_steps` | Planning | Suggested next actions based on state |
| `cwa_generate_tasks` | Planning | Auto-create tasks from spec criteria |
| `cwa_search_memory` | All phases | Text search project memory |
| `cwa_graph_query` | Design | Execute Cypher on knowledge graph |
| `cwa_graph_impact` | Design, Planning | Analyze impact of entity changes |
| `cwa_graph_sync` | Design | Trigger SQLite to Neo4j sync |
| `cwa_memory_semantic_search` | All phases | Vector similarity search |
| `cwa_memory_add` | Memory | Store memory with embedding |
| `cwa_observe` | Implementation, Memory | Record structured observation |
| `cwa_memory_timeline` | Memory | Compact timeline (~50 tokens/entry) |
| `cwa_memory_get` | Memory | Full observation details (~500 tokens/entry) |
| `cwa_memory_search_all` | All phases | Search across all memory types |

### MCP Resources

| URI | Description |
|-----|-------------|
| `project://constitution` | Project values and constraints |
| `project://current-spec` | Currently active specification |
| `project://domain-model` | DDD model with contexts |
| `project://kanban-board` | Current board state |
| `project://decisions` | Architectural decision log |

### Generated Artifacts (`.claude/` Directory)

CWA generates a complete Claude Code configuration directory:

| Artifact | Source | Claude Code Feature |
|----------|--------|---------------------|
| `agents/*.md` | Bounded contexts | [Agents](https://docs.anthropic.com/claude-code/agents) - domain expert personas |
| `skills/*/SKILL.md` | Approved specs | [Skills](https://docs.anthropic.com/claude-code/skills) - repeatable workflows |
| `commands/*.md` | Built-in (8) | [Commands](https://docs.anthropic.com/claude-code/commands) - slash commands |
| `rules/*.md` | Built-in (5) | [Rules](https://docs.anthropic.com/claude-code/rules) - code constraints |
| `hooks.json` | Domain invariants | [Hooks](https://docs.anthropic.com/claude-code/hooks) - event-driven validation |
| `design-system.md` | UI screenshots | Design tokens for consistent UI |

#### Built-in Agents (8)

| Agent | Role | Key MCP Tools Used |
|-------|------|--------------------|
| `analyst.md` | Requirements research | `cwa_search_memory`, `cwa_memory_add` |
| `architect.md` | DDD architecture decisions | `cwa_get_domain_model`, `cwa_add_decision`, `cwa_graph_query` |
| `specifier.md` | Spec-driven development | `cwa_get_spec`, `cwa_generate_tasks` |
| `implementer.md` | TDD implementation | `cwa_get_current_task`, `cwa_get_spec`, `cwa_observe`, `cwa_update_task_status` |
| `reviewer.md` | Code review vs. criteria | `cwa_get_current_task`, `cwa_get_spec`, `cwa_update_task_status` |
| `orchestrator.md` | Workflow coordination | All tools - central hub for workflow enforcement |
| `tester.md` | BDD test generation | `cwa_get_spec`, `cwa_observe` |
| `documenter.md` | Docs & ADR maintenance | `cwa_add_decision`, `cwa_memory_add`, codegen tools |

#### Built-in Skills (2)

| Skill | Purpose | Workflow |
|-------|---------|----------|
| `workflow-kickoff` | Feature idea → full workflow | Create spec → generate tasks → generate skill → update CLAUDE.md |
| `refactor-safe` | Safe refactoring with tests | Record decision → run tests → refactor → verify → move to review |

#### Built-in Commands (8)

| Command | Purpose |
|---------|---------|
| `/project:create-spec` | Create specification with acceptance criteria |
| `/project:implement-task` | Load current task + spec, implement, advance |
| `/project:session-summary` | Generate session summary, capture insights |
| `/project:next-task` | Pick next task respecting WIP limits |
| `/project:review-code` | Review against acceptance criteria |
| `/project:domain-discover` | Interactive domain discovery |
| `/project:sync-context` | Regenerate all artifacts |
| `/project:status` | Full project state dashboard |

#### Built-in Rules (5)

| Rule | Enforces |
|------|----------|
| `workflow.md` | Spec before code, WIP limits, decision tracking |
| `domain.md` | DDD principles, ubiquitous language, aggregate boundaries |
| `tests.md` | AAA pattern, coverage gates, test naming |
| `api.md` | REST conventions, input validation, security |
| `memory.md` | When and what to record in memory |

## Web Dashboard

Start with `cwa serve` and open `http://localhost:3030`.

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
| POST | `/api/specs/{id}/generate-tasks` | Generate tasks from spec criteria |
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

`cwa init` creates a complete Docker infrastructure in `.cwa/docker/`:

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| Neo4j | `neo4j:5.26-community` | 7474 (HTTP), 7687 (Bolt) | Knowledge Graph |
| Qdrant | `qdrant/qdrant:v1.13.2` | 6333 (HTTP), 6334 (gRPC) | Vector Store |
| Ollama | `ollama/ollama:0.5.4` | 11434 | Embeddings (nomic-embed-text, 768 dims) |

Default credentials (configurable via `.cwa/docker/.env`):
- Neo4j: `neo4j` / `cwa_dev_2026`

## Project Structure

When you run `cwa init`, the following structure is created:

```
my-project/
├── .cwa/
│   ├── cwa.db                    # SQLite database
│   ├── constitution.md           # Project values & constraints
│   └── docker/                   # Docker infrastructure
│       ├── docker-compose.yml    # Neo4j, Qdrant, Ollama services
│       ├── .env.example          # Environment template
│       └── scripts/
│           ├── init-qdrant.sh    # Qdrant collection setup
│           └── init-neo4j.cypher # Neo4j constraints/indexes
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

# Step 3: Add acceptance criteria and generate tasks
cwa spec add-criteria <spec-id> \
  "Define notification events enum" \
  "Create notification store (SQLite)" \
  "Implement WebSocket delivery" \
  "Add email delivery via SMTP" \
  "Build notification preferences UI"
cwa task generate <spec-id>

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

## MCP-Driven Workflow Example

### Session 1: Project Bootstrap

**You say to Claude Code:**

> "I'm building a subscription billing platform for a SaaS startup.
> Two developers, different timezones. We need user management,
> subscription plans (free/pro/enterprise), Stripe payments,
> and email receipts."

**Claude Code (via MCP) automatically:**
1. Creates bounded contexts: Subscriptions, Payments, Accounts
2. Creates specs with acceptance criteria for each feature
3. Generates tasks from specs, populating the Kanban board
4. Records initial design decisions
5. Generates Claude Code artifacts (agents, skills, CLAUDE.md)

**You see:**
- Kanban board populated with tasks
- `.claude/agents/` with expert agents per context
- `CLAUDE.md` with full project context
- Ready to start implementing

### Session 2: Continue Development

**You say to Claude Code:**

> "What should I work on next?"

**Claude Code (via MCP) automatically:**
1. Reads project state via `cwa_get_context_summary`
2. Checks WIP limits via `cwa_get_current_task`
3. Suggests the highest-priority unblocked task
4. Loads the spec with acceptance criteria
5. Begins TDD implementation

### Session 3: Memory Across Sessions

**You say to Claude Code:**

> "Why did we choose Stripe over PayPal?"

**Claude Code (via MCP) automatically:**
1. Searches memory via `cwa_memory_semantic_search`
2. Finds the decision recorded in Session 1
3. Returns the rationale with full context

---

## Manual CLI Workflow (Reference)

For users who prefer direct CLI control, scripting, or CI/CD integration:

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

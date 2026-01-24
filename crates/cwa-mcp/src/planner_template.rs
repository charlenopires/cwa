//! Planning document template generation.
//!
//! Renders a structured markdown document that instructs Claude Desktop
//! to ask clarifying questions and generate a comprehensive project plan
//! compatible with the CWA workflow.

use std::path::Path;

/// Existing project state read from a CWA database.
pub struct ExistingState {
    pub project_name: String,
    pub contexts: Vec<ContextInfo>,
    pub specs: Vec<SpecInfo>,
    pub tasks: Vec<TaskInfo>,
    pub decisions: Vec<DecisionInfo>,
    pub glossary: Vec<GlossaryInfo>,
}

pub struct ContextInfo {
    pub name: String,
    pub description: Option<String>,
}

pub struct SpecInfo {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub description: Option<String>,
    pub acceptance_criteria: Vec<String>,
}

pub struct TaskInfo {
    pub title: String,
    pub status: String,
    pub priority: String,
    pub spec_id: Option<String>,
}

pub struct DecisionInfo {
    pub title: String,
    pub status: String,
    pub decision: String,
}

pub struct GlossaryInfo {
    pub term: String,
    pub definition: String,
}

/// Read existing project state from a CWA database.
pub fn read_existing_state(project_path: &Path) -> anyhow::Result<ExistingState> {
    let db_path = project_path.join(".cwa/cwa.db");
    if !db_path.exists() {
        anyhow::bail!("No CWA database found at {}", db_path.display());
    }

    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found in database"))?;

    // Read bounded contexts
    let contexts = cwa_core::domain::list_contexts(&pool, &project.id)?
        .into_iter()
        .map(|c| ContextInfo {
            name: c.name,
            description: c.description,
        })
        .collect();

    // Read specs
    let specs = cwa_core::spec::list_specs(&pool, &project.id)?
        .into_iter()
        .map(|s| SpecInfo {
            id: s.id[..8].to_string(),
            title: s.title,
            status: format!("{:?}", s.status).to_lowercase(),
            priority: format!("{:?}", s.priority).to_lowercase(),
            description: s.description,
            acceptance_criteria: s.acceptance_criteria,
        })
        .collect();

    // Read tasks
    let tasks = cwa_core::task::list_tasks(&pool, &project.id)?
        .into_iter()
        .map(|t| TaskInfo {
            title: t.title,
            status: t.status.as_str().to_string(),
            priority: t.priority,
            spec_id: t.spec_id.map(|id| id[..8].to_string()),
        })
        .collect();

    // Read decisions
    let decisions = cwa_core::decision::list_decisions(&pool, &project.id)?
        .into_iter()
        .map(|d| DecisionInfo {
            title: d.title,
            status: d.status.as_str().to_string(),
            decision: d.decision,
        })
        .collect();

    // Read glossary
    let glossary = cwa_core::domain::list_glossary(&pool, &project.id)?
        .into_iter()
        .map(|g| GlossaryInfo {
            term: g.term,
            definition: g.definition,
        })
        .collect();

    Ok(ExistingState {
        project_name: project.name,
        contexts,
        specs,
        tasks,
        decisions,
        glossary,
    })
}

/// Render the complete planning document.
pub fn render_planning_document(prompt: &str, existing: Option<ExistingState>) -> String {
    let mut doc = String::with_capacity(8192);

    // Header and instructions
    doc.push_str(HEADER);
    doc.push_str(&format!("\n## User Prompt\n\n> {}\n", prompt));

    // Template sections
    doc.push_str(TEMPLATE_SECTIONS);

    // If existing state is available, append it
    if let Some(state) = existing {
        doc.push_str("\n---\n\n## Existing Project State\n\n");
        doc.push_str(&format!("**Project:** {}\n\n", state.project_name));

        if !state.contexts.is_empty() {
            doc.push_str("### Bounded Contexts\n\n");
            for ctx in &state.contexts {
                doc.push_str(&format!("- **{}**", ctx.name));
                if let Some(desc) = &ctx.description {
                    doc.push_str(&format!(" — {}", desc));
                }
                doc.push('\n');
            }
            doc.push('\n');
        }

        if !state.specs.is_empty() {
            doc.push_str("### Specifications\n\n");
            for spec in &state.specs {
                doc.push_str(&format!(
                    "#### {} `[{}]` `[{}]` `{}`\n",
                    spec.title, spec.status, spec.priority, spec.id
                ));
                if let Some(desc) = &spec.description {
                    doc.push_str(&format!("{}\n", desc));
                }
                if !spec.acceptance_criteria.is_empty() {
                    doc.push_str("\nCriteria:\n");
                    for c in &spec.acceptance_criteria {
                        doc.push_str(&format!("- [ ] {}\n", c));
                    }
                }
                doc.push('\n');
            }
        }

        if !state.tasks.is_empty() {
            doc.push_str("### Tasks\n\n");
            doc.push_str("| Status | Priority | Title | Spec |\n");
            doc.push_str("|--------|----------|-------|------|\n");
            for task in &state.tasks {
                doc.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    task.status,
                    task.priority,
                    task.title,
                    task.spec_id.as_deref().unwrap_or("-")
                ));
            }
            doc.push('\n');
        }

        if !state.decisions.is_empty() {
            doc.push_str("### Decisions\n\n");
            for dec in &state.decisions {
                doc.push_str(&format!(
                    "- **{}** `[{}]`: {}\n",
                    dec.title, dec.status, dec.decision
                ));
            }
            doc.push('\n');
        }

        if !state.glossary.is_empty() {
            doc.push_str("### Glossary\n\n");
            doc.push_str("| Term | Definition |\n");
            doc.push_str("|------|------------|\n");
            for g in &state.glossary {
                doc.push_str(&format!("| {} | {} |\n", g.term, g.definition));
            }
            doc.push('\n');
        }

        doc.push_str("### Continuation Notes\n\n");
        doc.push_str("Based on the existing state above, integrate new requirements with what already exists. ");
        doc.push_str("Add new specs, extend domain model, and generate additional tasks as needed. ");
        doc.push_str("Use `cwa spec add-criteria` to extend existing specs rather than creating duplicates.\n");
    }

    doc
}

const HEADER: &str = r#"# CWA Software Planning Document

> **INSTRUCTIONS FOR CLAUDE:**
>
> 1. **Do NOT write any code.** Your role is to plan, ask questions, and structure.
> 2. Read the user's prompt below carefully.
> 3. Ask 3-5 clarifying questions about:
>    - Target users and their primary workflows
>    - Technical constraints (language, framework, deployment)
>    - Scale expectations (users, data volume, performance)
>    - Integration requirements (APIs, databases, auth providers)
>    - Non-functional requirements (security, monitoring, compliance)
> 4. Based on the user's answers, fill in ALL sections below.
> 5. After the user approves, output the **completed document as a single markdown**.
> 6. The user will use this document in **Claude Code** with CWA to execute the plan.
>
> **IMPORTANT:** Every section must be filled. Use your best judgment for details
> the user doesn't specify. The goal is a complete, actionable planning document
> that prevents context fragmentation across development sessions.

"#;

const TEMPLATE_SECTIONS: &str = r#"
---

## Project Overview

- **Name**: [Project name — derive from prompt]
- **Description**: [2-3 sentence summary of what the software does]
- **Target Users**: [Who uses this and why]
- **Tech Stack**: [Language, framework, database, deployment target]
- **Key Constraints**: [Performance, security, compliance requirements]

---

## Bounded Contexts (DDD)

Identify the major responsibility boundaries in this system. Each context owns its data and logic.

### Context: [Name]

- **Responsibility**: [What this context exclusively owns]
- **Key Entities**: [Primary domain objects]
- **Upstream Dependencies**: [Contexts it consumes from]
- **Downstream Consumers**: [Contexts that consume from it]

_(Repeat for each bounded context identified)_

---

## Specifications

Each spec represents a distinct feature or capability. Include clear, testable acceptance criteria.

### SPEC-1: [Feature Title]

- **Priority**: [critical | high | medium | low]
- **Description**: [What this feature does and why it exists]
- **Acceptance Criteria**:
  - [ ] [Given/When/Then or clear testable statement]
  - [ ] [Criterion 2]
  - [ ] [Criterion 3]
- **Dependencies**: [Other spec IDs this depends on, if any]

_(Repeat for each specification)_

---

## Domain Model

For each bounded context, define the domain objects:

### [Context Name] Domain

**Entities** (identity-bearing objects):
- `EntityName` — [description]
  - Properties: `property_name: type`, ...
  - Invariants: [business rules that must always hold]
  - Behaviors: [key operations]

**Value Objects** (immutable, no identity):
- `VOName` — [description, properties]

**Aggregates** (consistency boundaries):
- `AggregateName` — root: `EntityName`, contains: [child entities]

**Domain Events** (things that happen):
- `EventName` — [trigger condition, payload, consumers]

**Domain Services** (stateless operations):
- `ServiceName` — [what it coordinates]

---

## Glossary (Ubiquitous Language)

| Term | Definition | Context |
|------|-----------|---------|
| [Term] | [Precise, unambiguous definition] | [Bounded context] |

_(Include all domain-specific terms to ensure team alignment)_

---

## Architectural Decisions

### ADR-1: [Decision Title]

- **Status**: proposed
- **Context**: [Problem or force driving this decision]
- **Decision**: [What was decided and why]
- **Alternatives Considered**: [Other options and why rejected]
- **Consequences**: [Trade-offs accepted, follow-up actions needed]

_(Repeat for major architectural choices)_

---

## Task Breakdown

Group tasks by spec. Each task should be independently implementable.

### Tasks for SPEC-1: [Spec Title]

1. [ ] [Task title] — [brief scope description]
2. [ ] [Task title] — [brief scope description]
3. [ ] [Task title] — [brief scope description]

_(Repeat for each spec)_

---

## CWA Bootstrap Commands

Copy-paste these commands into Claude Code to initialize the full project workflow:

```bash
# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 1: INITIALIZE PROJECT
# ═══════════════════════════════════════════════════════════════════════════════
cwa init "[project-name]"
cd [project-name]

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 2: INFRASTRUCTURE (optional — enables Knowledge Graph + Semantic Memory)
# ═══════════════════════════════════════════════════════════════════════════════
cwa infra up              # Starts Neo4j, Qdrant, Ollama
cwa infra status          # Verify all services healthy

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 3: DOMAIN MODELING — Bounded Contexts
# Define the major responsibility boundaries of the system
# ═══════════════════════════════════════════════════════════════════════════════
cwa domain context new "[context-name-1]" --description "[What this context exclusively owns]"
cwa domain context new "[context-name-2]" --description "[What this context exclusively owns]"
# ... one per bounded context

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 4: DOMAIN GLOSSARY — Ubiquitous Language
# Record domain-specific terms to ensure consistent language across the project
# ═══════════════════════════════════════════════════════════════════════════════
cwa memory add "[Term]: [Precise definition in project context]" --type fact
cwa memory add "[Term]: [Precise definition in project context]" --type fact
# ... one per domain term

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 5: ARCHITECTURAL DECISIONS
# Record early design choices with rationale (prevents re-debating later)
# ═══════════════════════════════════════════════════════════════════════════════
cwa memory add "Using [technology] for [purpose] because [rationale]. Alternatives considered: [alternatives]" --type decision
cwa memory add "Architecture: [pattern/style] because [rationale]" --type decision
# ... one per architectural decision

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 6: SPECIFICATIONS — Features with Acceptance Criteria
# Each spec is a distinct feature. Criteria must be clear and testable.
# ═══════════════════════════════════════════════════════════════════════════════
cwa spec new "[Feature Title]" \
  --description "[What this feature does and why it matters]" \
  --priority [critical|high|medium|low] \
  -c "[Criterion 1 — testable statement]" \
  -c "[Criterion 2 — testable statement]" \
  -c "[Criterion 3 — testable statement]" \
  -c "[Criterion 4 — testable statement]"

cwa spec new "[Feature Title]" \
  --description "[What this feature does and why it matters]" \
  --priority [critical|high|medium|low] \
  -c "[Criterion 1]" \
  -c "[Criterion 2]" \
  -c "[Criterion 3]"

# ... repeat for each specification

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 7: GENERATE TASKS FROM SPECS
# Auto-creates one task per acceptance criterion, populating the Kanban board
# ═══════════════════════════════════════════════════════════════════════════════
cwa task generate "[spec-title-1]"
cwa task generate "[spec-title-2]"
# ... one per spec

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 8: KNOWLEDGE GRAPH SYNC
# Syncs all entities to Neo4j for impact analysis and relationship queries
# ═══════════════════════════════════════════════════════════════════════════════
cwa graph sync
cwa graph status          # Verify nodes and relationships created

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 9: GENERATE CLAUDE CODE ARTIFACTS
# Creates agents, skills, hooks, commands, rules, and CLAUDE.md
# ═══════════════════════════════════════════════════════════════════════════════
cwa codegen all

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 10: VERIFY & ANALYZE
# Confirm everything is set up correctly and within token budget
# ═══════════════════════════════════════════════════════════════════════════════
cwa spec list              # All specs with status and priority
cwa task board             # Kanban board with WIP limits
cwa domain context list    # Bounded contexts
cwa context status         # Project focus summary
cwa tokens analyze --all   # Token budget across all context files
cwa tokens optimize        # Suggestions to stay within model window
```

---

## Claude Code Development Workflow

After bootstrapping, follow this workflow in each Claude Code session:

### Session Start
```bash
# Claude Code automatically reads CLAUDE.md and .claude/ artifacts
# Check current project state:
cwa context status                     # Quick overview of focus and progress
cwa task board                         # See the Kanban board
```

### Pick and Start Work
```bash
cwa task move [task-id] in_progress    # Claim a task (respects WIP limit of 1)
# Claude reads the spec via MCP tool cwa_get_spec
# Tester agent writes tests from acceptance criteria first
# Implementer agent writes code to pass tests
```

### During Implementation
```bash
# Record structured observations as you work:
cwa memory observe "[what happened]" -t bugfix -f "[root cause]"
cwa memory observe "[what you discovered]" -t discovery -f "[key fact]"
cwa memory observe "[refactoring done]" -t refactor --files-modified src/foo.rs

# Record architectural decisions with rationale:
cwa memory add "[Decision with full rationale]" --type decision

# Check impact before changing a domain entity:
cwa graph impact context [context-id]  # What specs/tasks/decisions are affected?
cwa graph impact spec [spec-id]        # What tasks implement this spec?

# Search past knowledge:
cwa memory search "[query]"            # Semantic search across all memory
cwa memory timeline --days 7           # Recent observations timeline
```

### Complete Task
```bash
cwa task move [task-id] review         # Move to review
# Reviewer agent validates against acceptance criteria
cwa task move [task-id] done           # Mark complete
```

### End of Session
```bash
# Record session insights:
cwa memory observe "[session summary]" -t insight -f "[key learning]"

# Regenerate artifacts with updated state:
cwa codegen all                        # Updates CLAUDE.md, agents, skills, hooks

# Verify token budget:
cwa tokens analyze --all               # Ensure context fits model window

# Sync knowledge graph:
cwa graph sync                         # Update relationships in Neo4j

# If UI work was done, capture design system:
cwa design from-image [screenshot-url] # Extract design tokens from UI
```

### Key MCP Tools Reference

| Tool | Phase | When to Use |
|------|-------|-------------|
| `cwa_get_context_summary` | Start | Quick project state overview |
| `cwa_get_current_task` | Work | Check what you're working on |
| `cwa_get_spec` | Work | Read acceptance criteria |
| `cwa_get_domain_model` | Work | Check entity definitions and invariants |
| `cwa_update_task_status` | Work | Move tasks through workflow |
| `cwa_observe` | Work | Record bugfix, discovery, feature, refactor |
| `cwa_add_decision` | Work | Record architectural decisions with ADR |
| `cwa_search_memory` | Any | Recall past observations and patterns |
| `cwa_memory_semantic_search` | Any | Vector similarity search across memory |
| `cwa_memory_timeline` | Start | Compact timeline of recent activity |
| `cwa_graph_impact` | Design | Analyze impact of entity changes |
| `cwa_graph_sync` | End | Sync SQLite to Neo4j |
| `cwa_generate_tasks` | Plan | Auto-create tasks from spec criteria |
| `cwa_create_spec` | Plan | Create spec with criteria via MCP |
| `cwa_create_context` | Plan | Create bounded context via MCP |
| `cwa_create_task` | Plan | Create individual task via MCP |
| `cwa_get_next_steps` | Start | Get suggested next actions |
| `cwa_memory_add` | Any | Store facts, preferences, patterns |

"#;

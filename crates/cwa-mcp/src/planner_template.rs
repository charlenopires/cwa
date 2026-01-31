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
    doc.push_str(&format!("\nUSER REQUEST: {}\n", prompt));

    // Template sections
    doc.push_str(TEMPLATE_SECTIONS);

    // If existing state is available, show it as already-executed commands
    if let Some(state) = existing {
        doc.push_str(&format!("\nALREADY EXISTS (project: \"{}\") — do NOT recreate these, only ADD new ones:\n\n", state.project_name));

        if !state.contexts.is_empty() {
            doc.push_str("# Existing contexts:\n");
            for ctx in &state.contexts {
                let desc = ctx.description.as_deref().unwrap_or("");
                doc.push_str(&format!("# - {} ({})\n", ctx.name, desc));
            }
        }

        if !state.specs.is_empty() {
            doc.push_str("# Existing specs:\n");
            for spec in &state.specs {
                doc.push_str(&format!("# - {} [{}] [{}]\n", spec.title, spec.status, spec.priority));
            }
        }

        if !state.glossary.is_empty() {
            doc.push_str("# Existing glossary:\n");
            for g in &state.glossary {
                doc.push_str(&format!("# - {}: {}\n", g.term, g.definition));
            }
        }

        doc.push_str("# Generate ONLY new commands that extend the project. Use cwa spec add-criteria for existing specs.\n\n");
    }

    doc
}

const HEADER: &str = r#"You are a software architect using Domain-Driven Design (DDD) and Specification-Driven Development (SDD) principles.

## METHODOLOGY

### Domain-Driven Design (DDD) Strategic Patterns
- **Subdomains**: Identify Core, Supporting, and Generic subdomains
- **Bounded Contexts**: Define clear boundaries where domain models apply
- **Ubiquitous Language**: Establish shared vocabulary between team and code
- **Context Mapping**: Define relationships between bounded contexts

### Specification-Driven Development (SDD)
- Specifications are the SOURCE OF TRUTH, not code
- Requirements and acceptance criteria BEFORE implementation
- Each spec is a contract that drives design, testing, and documentation

## CWA CLI COMMANDS REFERENCE

### Project Management
- `cwa init "<name>"` — Initialize new project
- `cwa update` — Update project info and regenerate context files
- `cwa context status` — Show project context status

### Infrastructure
- `cwa infra up` — Start Docker infrastructure (Neo4j, Qdrant)
- `cwa infra down` — Stop infrastructure
- `cwa infra status` — Check infrastructure status

### Specifications (SDD)
- `cwa spec new "<title>" --description "<desc>" --priority <critical|high|medium|low> -c "<criterion>"` — Create spec with criteria
- `cwa spec add-criteria "<spec-id>" -c "<criterion>"` — Add criteria to existing spec
- `cwa spec list` — List all specs
- `cwa spec get "<id>"` — Get spec details
- `cwa spec validate "<id>"` — Validate spec completeness

### Domain Modeling (DDD)
- `cwa domain context new "<name>" --description "<desc>"` — Create bounded context
- `cwa domain context list` — List all contexts
- `cwa domain glossary` — Show domain glossary

### Memory & Observations
- `cwa memory add "<content>" --type <fact|decision|preference|pattern>` — Store memory with embedding
- `cwa memory observe "<title>" --type <bugfix|feature|discovery|decision|insight> --narrative "<text>"` — Record observation
- `cwa memory search "<query>"` — Semantic search
- `cwa memory timeline` — View recent observations

### Tasks (Kanban)
- `cwa task new "<title>" --priority <critical|high|medium|low>` — Create task
- `cwa task generate "<spec-id>"` — Generate tasks from spec criteria
- `cwa task move "<task-id>" <backlog|todo|in_progress|review|done>` — Move task
- `cwa task board` — Show Kanban board
- `cwa task wip` — Show WIP status

### Knowledge Graph
- `cwa graph sync` — Sync entities to Neo4j
- `cwa graph status` — Check graph status

### Code Generation
- `cwa codegen all` — Generate all artifacts

## RULES

1. Ask 3-5 clarifying questions first:
   - Who are the users/actors?
   - What is the core domain vs supporting/generic?
   - What are the key business invariants?
   - What external systems need integration?
   - What are the scalability and performance constraints?

2. After answers, create a SINGLE MARKDOWN ARTIFACT titled "CWA Bootstrap — [project-name]".

3. The artifact must be ONLY a ```bash block with CWA commands. No other text.

4. Structure your plan using DDD/SDD phases (adapt as needed):
   - INITIALIZATION: Project setup
   - INFRASTRUCTURE: Enable Knowledge Graph + Semantic Memory
   - STRATEGIC DESIGN: Identify subdomains and bounded contexts
   - UBIQUITOUS LANGUAGE: Define domain glossary terms
   - ARCHITECTURAL DECISIONS: Record ADRs for key choices
   - SPECIFICATIONS: Feature specs with acceptance criteria (SDD)
   - KNOWLEDGE GRAPH: Sync domain model
   - VERIFICATION: Validate project state

5. ALL data must be REAL (from user answers), NOT placeholders.

6. Include ALL specs with ALL acceptance criteria. Do NOT abbreviate.

7. CRITICAL: Generate a SINGLE EXECUTABLE SCRIPT with ALL commands chained via &&:
   - Every command (except the last) must end with " && \"
   - Use line continuation (\) for readability
   - Comments (# lines) are allowed between commands
   - The ENTIRE script must be copy-pasteable and run as ONE command

"#;

const TEMPLATE_SECTIONS: &str = r#"
## EXAMPLE OUTPUT (for a "Session Manager" Chrome extension)

```bash
# ═══════════════════════════════════════════════════════════════════════════════
# CWA BOOTSTRAP SCRIPT — Session Manager
# Domain-Driven Design + Specification-Driven Development
# Copy and paste this entire block to execute all commands at once
# ═══════════════════════════════════════════════════════════════════════════════

# ─────────────────────────────────────────────────────────────────────────────
# INITIALIZATION
# ─────────────────────────────────────────────────────────────────────────────
cwa init "session-manager" && \

# ─────────────────────────────────────────────────────────────────────────────
# INFRASTRUCTURE (Knowledge Graph + Semantic Memory)
# ─────────────────────────────────────────────────────────────────────────────
cwa infra up && \
cwa infra status && \

# ─────────────────────────────────────────────────────────────────────────────
# STRATEGIC DESIGN — Bounded Contexts (DDD)
# Core Domain: Session management (what differentiates the product)
# Supporting: Tab handling, Tag categorization
# ─────────────────────────────────────────────────────────────────────────────
cwa domain context new "Session" --description "Core Domain: Lifecycle management of tab session snapshots" && \
cwa domain context new "Tab" --description "Supporting: Capture and representation of browser tabs" && \
cwa domain context new "Tag" --description "Supporting: Categorization and filtering via colored tags" && \

# ─────────────────────────────────────────────────────────────────────────────
# UBIQUITOUS LANGUAGE — Domain Glossary (DDD)
# Shared vocabulary between developers, users, and code
# ─────────────────────────────────────────────────────────────────────────────
cwa memory add "Session: Named snapshot of open tabs at a given moment" --type fact && \
cwa memory add "Tab: Browser tab representation with URL, title, favicon, and state" --type fact && \
cwa memory add "Tag: Colored label for categorizing and filtering sessions" --type fact && \
cwa memory add "Restore: Action of reopening all tabs from a saved session" --type fact && \
cwa memory add "Pin: Tab fixation state in the browser bar" --type fact && \

# ─────────────────────────────────────────────────────────────────────────────
# ARCHITECTURAL DECISIONS (ADRs)
# Key technical choices with rationale and alternatives considered
# ─────────────────────────────────────────────────────────────────────────────
cwa memory add "ADR-001: Using Chrome Storage API (sync) for persistence. Rationale: cross-device sync, 100KB space. Alternative rejected: IndexedDB (no sync)" --type decision && \
cwa memory add "ADR-002: Manifest V3 with Service Worker. Rationale: current Chrome standard, required for new extensions. Background pages deprecated" --type decision && \
cwa memory add "ADR-003: React + TailwindCSS for popup UI. Rationale: componentization and rapid development. Alternative rejected: Vanilla JS (complex for reactive UI)" --type decision && \
cwa memory add "ADR-004: UUID v4 for session IDs. Rationale: uniqueness without coordination. Alternative: timestamp (collision risk on rapid saves)" --type decision && \

# ─────────────────────────────────────────────────────────────────────────────
# SPECIFICATIONS — Source of Truth (SDD)
# Each spec is a contract with acceptance criteria that drive implementation
# ─────────────────────────────────────────────────────────────────────────────
cwa spec new "Session Save" \
  --description "Save current session capturing all open tabs with metadata" \
  --priority critical \
  -c "User can save current session with custom name" \
  -c "System captures URL, title, and favicon of each tab" \
  -c "System preserves pin state of each tab" \
  -c "System preserves tab order" \
  -c "Saved session includes creation timestamp" \
  -c "System prevents duplicate session names" \
  -c "Visual feedback confirms successful save" && \

cwa spec new "Session List" \
  --description "View and manage list of saved sessions" \
  --priority critical \
  -c "User views list of all saved sessions" \
  -c "Each session displays name, date, and tab count" \
  -c "User can rename existing session" \
  -c "User can delete session with confirmation" \
  -c "List sorted by creation date (newest first)" \
  -c "Text search filters sessions by name" && \

cwa spec new "Session Restore" \
  --description "Restore saved sessions by reopening all tabs with original properties" \
  --priority critical \
  -c "User can restore all tabs from a session" \
  -c "Option to restore in new window" \
  -c "Option to restore in current window" \
  -c "System preserves pin state of restored tabs" \
  -c "System preserves original tab order" \
  -c "Visual progress feedback during restoration" \
  -c "Handle invalid/inaccessible URLs with warning" && \

cwa spec new "Tag Management" \
  --description "Colored tag system for session categorization" \
  --priority medium \
  -c "User can create new tag with name and color" \
  -c "System offers predefined color palette for tags" \
  -c "User can edit existing tag name and color" \
  -c "User can delete tag (removes from all associated sessions)" \
  -c "Tags displayed as colored badges on sessions" \
  -c "System prevents duplicate tag names" \
  -c "Tag name limited to 30 characters" && \

cwa spec new "Tag Filtering" \
  --description "Filter sessions by tags for quick location" \
  --priority medium \
  -c "User can filter session list by one or more tags" \
  -c "Multiple tag filter uses OR logic (sessions with any selected tag)" \
  -c "User can combine tag filter with text search" \
  -c "Active filter tags are visually highlighted" \
  -c "Clear filters button removes all active filters" \
  -c "Counter shows session count after filter applied" && \

# ─────────────────────────────────────────────────────────────────────────────
# KNOWLEDGE GRAPH SYNC
# Synchronize domain model to Neo4j for relationship queries
# ─────────────────────────────────────────────────────────────────────────────
cwa graph sync && \
cwa graph status && \

# ─────────────────────────────────────────────────────────────────────────────
# VERIFICATION
# Generate artifacts and verify project state
# ─────────────────────────────────────────────────────────────────────────────
cwa codegen all && \
cwa spec list && \
cwa domain context list && \
cwa context status
```

## END OF EXAMPLE

Generate commands for the user's prompt below using DDD/SDD methodology.
Adapt the structure to the domain complexity — simpler projects need fewer phases.
ALL data must be REAL (from user answers). ALL commands chained with && in a SINGLE executable script.

"#;

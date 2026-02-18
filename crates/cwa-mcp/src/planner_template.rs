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
    pub decisions: Vec<DecisionInfo>,
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

pub struct DecisionInfo {
    pub title: String,
    pub status: String,
    pub decision: String,
}

/// Read existing project state from a CWA database.
pub async fn read_existing_state(project_path: &Path) -> anyhow::Result<ExistingState> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let pool = cwa_db::init_pool(&redis_url).await?;

    let project = cwa_core::project::get_default_project(&pool).await?
        .ok_or_else(|| anyhow::anyhow!("No project found in database"))?;

    // Read bounded contexts
    let contexts = cwa_core::domain::list_contexts(&pool, &project.id).await?
        .into_iter()
        .map(|c| ContextInfo {
            name: c.name,
            description: c.description,
        })
        .collect();

    // Read specs
    let specs = cwa_core::spec::list_specs(&pool, &project.id).await?
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

    // Read decisions
    let decisions = cwa_core::decision::list_decisions(&pool, &project.id).await?
        .into_iter()
        .map(|d| DecisionInfo {
            title: d.title,
            status: d.status.as_str().to_string(),
            decision: d.decision,
        })
        .collect();

    Ok(ExistingState {
        project_name: project.name,
        contexts,
        specs,
        decisions,
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

        if !state.decisions.is_empty() {
            doc.push_str("# Existing decisions:\n");
            for d in &state.decisions {
                doc.push_str(&format!("# - {} [{}]\n", d.title, d.status));
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
- **Context Mapping**: Define relationships between bounded contexts
- **Ubiquitous Language**: Domain glossary with shared vocabulary across team

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
- `cwa infra up` — Start infrastructure services (Qdrant, Neo4j)
- `cwa infra status` — Check infrastructure health

### Specifications (SDD)
- `cwa spec new "<title>" --description "<desc>" --priority <critical|high|medium|low> -c "<criterion>"` — Create spec with criteria
- `cwa spec add-criteria "<spec-id>" -c "<criterion>"` — Add criteria to existing spec
- `cwa spec list` — List all specs
- `cwa spec get "<id>"` — Get spec details
- `cwa spec validate "<id>"` — Validate spec completeness

### Domain Modeling (DDD)
- `cwa domain context new "<name>" --description "<desc>"` — Create bounded context
- `cwa domain context list` — List all contexts
- `cwa domain object new "<name>" --context "<ctx>" --type <aggregate|entity|value_object|service|event> --description "<desc>"` — Create domain object

### Memory & Observations
- `cwa memory add "<content>" --type <fact|decision|preference|pattern>` — Store memory with embedding

### Knowledge Graph
- `cwa graph sync` — Sync all project data to Neo4j knowledge graph
- `cwa graph query "<cypher>"` — Execute Cypher query

### Tech Stack
- `cwa stack set <tech> [<tech2>...]` — Set project tech stack (writes .cwa/stack.json)
- `cwa stack show` — Show current tech stack and available agent templates

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

3. The artifact must be ONLY a ```bash block with CWA commands. No other text outside the code block (except a TECH STACK table at the very end).

4. Include `cwa stack set <technologies>` in Phase 9 BEFORE `cwa codegen all`. Use ONLY the technologies from the TECH STACK table you generate. This ensures tech-stack-aware expert agents are automatically selected.

5. Structure your plan using these 9 phases (adapt as needed):
   - Phase 1 — INITIALIZATION: Project setup
   - Phase 2 — INFRASTRUCTURE: Start services
   - Phase 3 — STRATEGIC DESIGN: Identify subdomains and bounded contexts
   - Phase 4 — DOMAIN GLOSSARY: Ubiquitous language terms as facts
   - Phase 5 — ARCHITECTURAL DECISIONS: Record ADRs as memories (--type decision)
   - Phase 6 — SPECIFICATIONS: Feature specs with acceptance criteria (SDD)
   - Phase 7 — DOMAIN OBJECTS: Define aggregates, entities, value objects, services, events
   - Phase 8 — KNOWLEDGE GRAPH SYNC: Sync project to Neo4j
   - Phase 9 — GENERATE ARTIFACTS & VERIFY: Generate code and verify state

6. ALL data must be REAL (from user answers), NOT placeholders.

7. Include ALL specs with ALL acceptance criteria. Do NOT abbreviate.

8. CRITICAL: Generate a SINGLE EXECUTABLE SCRIPT with ALL commands chained via &&:
   - Every command (except the last) must end with " && \"
   - Use line continuation (\) for readability
   - Comments (# lines) are allowed between commands
   - The ENTIRE script must be copy-pasteable and run as ONE command

9. After the bash block, include a TECH STACK summary table in markdown with columns: Component | Decision | Rationale.

"#;

const TEMPLATE_SECTIONS: &str = r#"
## EXAMPLE OUTPUT (for a "Session Manager" Chrome extension)

```bash
# ═══════════════════════════════════════════════════════════════════════════════
# CWA BOOTSTRAP SCRIPT — Session Manager
# Domain-Driven Design + Specification-Driven Development
# Copy and paste this entire block to execute all commands at once
# ═══════════════════════════════════════════════════════════════════════════════

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 1 — INITIALIZATION
# ═══════════════════════════════════════════════════════════════════════════════
cwa init "session-manager" && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 2 — INFRASTRUCTURE
# Start backing services (Qdrant for vector search, Neo4j for knowledge graph)
# ═══════════════════════════════════════════════════════════════════════════════
cwa infra up && \
cwa infra status && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 3 — STRATEGIC DESIGN — Bounded Contexts (DDD)
# Core Domain: Session management (what differentiates the product)
# Supporting: Tab handling, Tag categorization
# ═══════════════════════════════════════════════════════════════════════════════
cwa domain context new "Session" --description "Core Domain: Lifecycle management of tab session snapshots" && \
cwa domain context new "Tab" --description "Supporting: Capture and representation of browser tabs" && \
cwa domain context new "Tag" --description "Supporting: Categorization and filtering via colored tags" && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 4 — DOMAIN GLOSSARY — Ubiquitous Language
# Shared vocabulary so every team member and AI agent speaks the same language
# ═══════════════════════════════════════════════════════════════════════════════
cwa memory add "Session: A named snapshot of all currently open browser tabs, including their URLs, titles, favicons, pin states, and ordering" --type fact && \
cwa memory add "Tab: A single browser tab captured within a session, represented by URL, title, favicon, and pin state" --type fact && \
cwa memory add "Tag: A colored label used to categorize and filter sessions for quick retrieval" --type fact && \
cwa memory add "Restore: The action of reopening all tabs from a saved session, preserving their original properties" --type fact && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 5 — ARCHITECTURAL DECISIONS (ADRs)
# Key technical choices with rationale and alternatives considered
# ═══════════════════════════════════════════════════════════════════════════════
cwa memory add "ADR-001: Using Chrome Storage API (sync) for persistence. Rationale: cross-device sync, 100KB space. Alternative rejected: IndexedDB (no sync)" --type decision && \
cwa memory add "ADR-002: Manifest V3 with Service Worker. Rationale: current Chrome standard, required for new extensions. Background pages deprecated" --type decision && \
cwa memory add "ADR-003: React + TailwindCSS for popup UI. Rationale: componentization and rapid development. Alternative rejected: Vanilla JS (complex for reactive UI)" --type decision && \
cwa memory add "ADR-004: UUID v4 for session IDs. Rationale: uniqueness without coordination. Alternative: timestamp (collision risk on rapid saves)" --type decision && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 6 — SPECIFICATIONS — Source of Truth (SDD)
# Each spec is a contract with acceptance criteria that drive implementation
# ═══════════════════════════════════════════════════════════════════════════════
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

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 7 — DOMAIN OBJECTS
# Define aggregates, entities, value objects, services, and events per context
# ═══════════════════════════════════════════════════════════════════════════════
cwa domain object new "Session" --context "Session" --type aggregate --description "Root aggregate representing a named snapshot of browser tabs" && \
cwa domain object new "SessionMetadata" --context "Session" --type value_object --description "Immutable metadata: name, creation timestamp, tab count" && \
cwa domain object new "SessionSaved" --context "Session" --type event --description "Event emitted when a session is successfully saved" && \
cwa domain object new "Tab" --context "Tab" --type entity --description "A single browser tab with URL, title, favicon, and pin state" && \
cwa domain object new "TabCaptureService" --context "Tab" --type service --description "Service that captures current browser tabs into a session snapshot" && \
cwa domain object new "Tag" --context "Tag" --type entity --description "A colored label used to categorize sessions" && \
cwa domain object new "TagColor" --context "Tag" --type value_object --description "Predefined color from the tag palette" && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 8 — KNOWLEDGE GRAPH SYNC
# Sync all project data (contexts, specs, decisions, domain objects) into Neo4j
# ═══════════════════════════════════════════════════════════════════════════════
cwa graph sync && \

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 9 — GENERATE ARTIFACTS & VERIFY
# Set tech stack BEFORE codegen so expert agents are selected automatically
# ═══════════════════════════════════════════════════════════════════════════════
cwa stack set typescript react tailwindcss && \

# Generate all artifacts (expert agents, skills, commands, hooks, CLAUDE.md, .mcp.json)
cwa codegen all && \

# Install CWA MCP to Claude Code (project-local .mcp.json auto-detection)
cwa mcp install claude-code && \

# Verify project state
cwa spec list && \
cwa domain context list && \
cwa context status
```

### TECH STACK

| Component | Decision | Rationale |
|-----------|----------|-----------|
| Platform | Chrome Extension (Manifest V3) | Current standard, service worker model |
| UI Framework | React 18 | Component model, ecosystem, rapid development |
| Styling | TailwindCSS | Utility-first, small bundle, fast iteration |
| Storage | Chrome Storage API (sync) | Cross-device sync, no backend needed |
| IDs | UUID v4 | Uniqueness without coordination |

## END OF EXAMPLE

Generate commands for the user's prompt below using DDD/SDD methodology.
Adapt the structure to the domain complexity — simpler projects need fewer phases.
ALL data must be REAL (from user answers). ALL commands chained with && in a SINGLE executable script.
Include a TECH STACK summary table after the bash block.

"#;

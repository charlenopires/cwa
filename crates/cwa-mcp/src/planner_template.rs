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
> 1. **Do NOT write any code.** Your role is to plan, ask questions, and generate CWA commands.
> 2. Read the user's prompt below carefully.
> 3. Ask 3-5 clarifying questions about:
>    - Target users and their primary workflows
>    - Technical constraints (language, framework, deployment)
>    - Scale expectations (users, data volume, performance)
>    - Integration requirements (APIs, databases, auth providers)
>    - Non-functional requirements (security, monitoring, compliance)
> 4. Based on the user's answers, think through the sections below internally.
> 5. **YOUR FINAL OUTPUT MUST BE A SINGLE MARKDOWN WITH ONLY EXECUTABLE CWA CLI COMMANDS.**
>    - Do NOT output descriptive sections (no "SPEC-1: Title" with bullet points).
>    - Output ONLY the `CWA Bootstrap Commands` section as a bash code block.
>    - ALL placeholders MUST be replaced with actual project data.
>    - Each command must be complete, copy-pasteable, and runnable in Claude Code.
> 6. The user will paste this output directly into **Claude Code** terminal.
>
> **OUTPUT FORMAT RULES:**
> - The final output is a single ```bash code block with all commands.
> - Use the 10-phase workflow structure shown in the template below.
> - Replace ALL `[placeholders]` with real data derived from the user's answers.
> - Specs MUST include `--description`, `--priority`, and multiple `-c` flags with real criteria.
> - Glossary terms MUST use real domain-specific terms from the project.
> - Decisions MUST include real technology choices and rationale.
> - DO NOT include phases that are not relevant (e.g., skip infra if not needed).
> - DO NOT abbreviate — include ALL specs, ALL criteria, ALL terms, ALL decisions.

"#;

const TEMPLATE_SECTIONS: &str = r#"
---

## Internal Planning (use as reasoning scaffold — DO NOT include in final output)

Think through these sections to derive the commands, but DO NOT output them:

- **Project Overview**: Name, description, tech stack, constraints
- **Bounded Contexts**: Major responsibility boundaries, key entities
- **Domain Model**: Entities, value objects, aggregates, invariants, events, services
- **Glossary**: Domain-specific terms with precise definitions
- **Architectural Decisions**: Technology choices with rationale
- **Specifications**: Features with testable acceptance criteria
- **Task Breakdown**: One task per criterion

---

## YOUR OUTPUT — CWA Bootstrap Commands

**This is the ONLY section you output.** Replace ALL placeholders with real project data.
Output as a single markdown document with bash code blocks. Include all 10 phases.

### Example output (for a "Session Manager" Chrome extension):

```bash
# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 1: INITIALIZE PROJECT
# ═══════════════════════════════════════════════════════════════════════════════
cwa init "session-manager"

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 2: INFRASTRUCTURE (optional — enables Knowledge Graph + Semantic Memory)
# Skip if project doesn't need graph queries or semantic search
# ═══════════════════════════════════════════════════════════════════════════════
cwa infra up
cwa infra status

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 3: DOMAIN MODELING — Bounded Contexts
# ═══════════════════════════════════════════════════════════════════════════════
cwa domain context new "Session" --description "Gerenciamento do ciclo de vida de sessões de abas"
cwa domain context new "Tab" --description "Captura e representação de abas individuais do navegador"
cwa domain context new "Tag" --description "Categorização e filtragem de sessões via tags coloridas"

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 4: DOMAIN GLOSSARY — Ubiquitous Language
# ═══════════════════════════════════════════════════════════════════════════════
cwa memory add "Session: Snapshot nomeado de um conjunto de abas abertas em um dado momento" --type fact
cwa memory add "Tab: Representação de uma aba do navegador com URL, título, favicon e estado" --type fact
cwa memory add "Tag: Rótulo colorido para categorizar e filtrar sessões" --type fact
cwa memory add "Restore: Ação de reabrir todas as abas de uma sessão salva" --type fact
cwa memory add "Pin: Estado de fixação de uma aba na barra do navegador" --type fact

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 5: ARCHITECTURAL DECISIONS
# ═══════════════════════════════════════════════════════════════════════════════
cwa memory add "Usando Chrome Storage API (sync) para persistência porque permite sincronização entre dispositivos e tem 100KB de espaço. Alternativa descartada: IndexedDB (não sincroniza)" --type decision
cwa memory add "Arquitetura: Manifest V3 com Service Worker porque é o padrão atual do Chrome e obrigatório para novas extensões. Background pages foram depreciadas" --type decision
cwa memory add "UI: React + TailwindCSS no popup porque permite componentização e desenvolvimento rápido. Alternativa descartada: Vanilla JS (mais complexo para UI reativa)" --type decision
cwa memory add "Usando UUID v4 para IDs de sessões porque garante unicidade sem coordenação. Alternativa: timestamp (risco de colisão em salvamentos rápidos)" --type decision

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 6: SPECIFICATIONS — Features with Acceptance Criteria
# ═══════════════════════════════════════════════════════════════════════════════
cwa spec new "Session Save" \
  --description "Salvamento de sessões capturando todas as abas abertas com seus metadados" \
  --priority critical \
  -c "Usuário pode salvar sessão atual com nome personalizado" \
  -c "Sistema captura URL, título e favicon de cada aba" \
  -c "Sistema preserva estado de pin de cada aba" \
  -c "Sistema preserva ordem das abas" \
  -c "Sessão salva inclui timestamp de criação" \
  -c "Sistema previne nomes de sessão duplicados" \
  -c "Feedback visual confirma salvamento com sucesso"

cwa spec new "Session List" \
  --description "Visualização e gerenciamento da lista de sessões salvas" \
  --priority critical \
  -c "Usuário visualiza lista de todas as sessões salvas" \
  -c "Cada sessão exibe nome, data e quantidade de abas" \
  -c "Usuário pode renomear uma sessão existente" \
  -c "Usuário pode excluir sessão com confirmação" \
  -c "Lista ordenada por data de criação (mais recente primeiro)" \
  -c "Busca por texto filtra sessões pelo nome"

cwa spec new "Session Restore" \
  --description "Restauração de sessões salvas reabrindo todas as abas com propriedades originais" \
  --priority critical \
  -c "Usuário pode restaurar todas as abas de uma sessão" \
  -c "Opção de restaurar em nova janela" \
  -c "Opção de restaurar na janela atual" \
  -c "Sistema preserva estado de pin das abas restauradas" \
  -c "Sistema preserva ordem original das abas" \
  -c "Feedback visual de progresso durante restauração" \
  -c "Tratamento de URLs inválidas ou inacessíveis com aviso"

cwa spec new "Tag Management" \
  --description "Sistema de tags coloridas para categorização de sessões" \
  --priority medium \
  -c "Usuário pode criar nova tag com nome e cor" \
  -c "Sistema oferece paleta de cores predefinidas para tags" \
  -c "Usuário pode editar nome e cor de tag existente" \
  -c "Usuário pode excluir tag (remove de todas sessões associadas)" \
  -c "Tags são exibidas como badges coloridos nas sessões" \
  -c "Sistema previne tags com nomes duplicados" \
  -c "Nome da tag tem limite de 30 caracteres"

cwa spec new "Tag Filtering" \
  --description "Filtragem de sessões por tags para localização rápida" \
  --priority medium \
  -c "Usuário pode filtrar lista de sessões por uma ou mais tags" \
  -c "Filtro por múltiplas tags usa lógica OR (sessões com qualquer tag selecionada)" \
  -c "Usuário pode combinar filtro de tags com busca por texto" \
  -c "Tags ativas no filtro são destacadas visualmente" \
  -c "Botão limpar filtros remove todos os filtros ativos" \
  -c "Contador exibe quantidade de sessões após filtro aplicado"

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 7: GENERATE TASKS FROM SPECS
# ═══════════════════════════════════════════════════════════════════════════════
cwa task generate "Session Save"
cwa task generate "Session List"
cwa task generate "Session Restore"
cwa task generate "Tag Management"
cwa task generate "Tag Filtering"

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 8: KNOWLEDGE GRAPH SYNC
# ═══════════════════════════════════════════════════════════════════════════════
cwa graph sync
cwa graph status

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 9: GENERATE CLAUDE CODE ARTIFACTS
# ═══════════════════════════════════════════════════════════════════════════════
cwa codegen all

# ═══════════════════════════════════════════════════════════════════════════════
# PHASE 10: VERIFY & ANALYZE
# ═══════════════════════════════════════════════════════════════════════════════
cwa spec list
cwa task board
cwa domain context list
cwa context status
cwa tokens analyze --all
```

**END OF EXAMPLE.** Now generate the commands for the user's prompt below, following the same format with ALL 10 phases and real project data.

---

## Claude Code Development Workflow

After the user executes the bootstrap commands, they use this workflow in Claude Code:

### Session Start
```bash
cwa context status                     # Quick overview
cwa task board                         # See Kanban board
```

### Work Cycle
```bash
cwa task move [task-id] in_progress    # Claim a task (WIP limit: 1)
# Implement following spec acceptance criteria
cwa memory observe "[what happened]" -t bugfix -f "[root cause]"
cwa memory observe "[discovery]" -t discovery -f "[key fact]"
cwa memory add "[Decision with rationale]" --type decision
cwa graph impact spec [spec-id]        # Check impact before changes
cwa task move [task-id] done           # Complete task
```

### End of Session
```bash
cwa memory observe "[session summary]" -t insight -f "[learning]"
cwa codegen all                        # Regenerate artifacts
cwa graph sync                         # Update knowledge graph
cwa tokens analyze --all               # Verify token budget
```

"#;

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

const HEADER: &str = r#"You are a software architect that outputs ONLY executable CWA CLI commands.

RULES:
1. Ask 3-5 clarifying questions first (users, tech stack, scale, integrations, constraints).
2. After the user answers, create a SINGLE MARKDOWN ARTIFACT (document) titled "CWA Bootstrap — [project-name]".
3. The artifact content must be ONLY a bash code block with CWA commands. Nothing else before or after it.
4. NO descriptions, NO bullet points, NO markdown sections, NO explanations — ONLY the ```bash block.
5. Follow the 10-phase structure shown in the example below.
6. ALL data must be real (derived from the user's answers), NOT placeholders.
7. Include ALL specs with ALL acceptance criteria. Do NOT abbreviate.
8. The user will copy the content of this artifact directly into Claude Code terminal.

"#;

const TEMPLATE_SECTIONS: &str = r#"
EXAMPLE OUTPUT (for a "Session Manager" Chrome extension):

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

END OF EXAMPLE. Now generate commands for the user's prompt below. Same format. All 10 phases. Real data only.

"#;

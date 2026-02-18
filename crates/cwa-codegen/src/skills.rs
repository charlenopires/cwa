//! Generate Claude skill files from specs.
//!
//! Each approved Spec produces a skill definition with steps
//! and acceptance criteria.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use cwa_db::DbPool;

/// A generated skill definition.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedSkill {
    pub dirname: String,
    pub filename: String,
    pub content: String,
    pub spec_title: String,
}

/// Generate a skill from a spec.
pub async fn generate_skill(db: &DbPool, project_id: &str, spec_id: &str) -> Result<GeneratedSkill> {
    let spec = cwa_core::spec::get_spec(db, project_id, spec_id).await
        .map_err(|e| anyhow::anyhow!("Spec not found: {}", e))?;

    let slug = slugify(&spec.title);
    let dirname = slug.clone();
    let filename = "SKILL.md".to_string();

    let mut content = String::new();

    content.push_str(&format!("# {}\n\n", spec.title));

    if let Some(ref desc) = spec.description {
        content.push_str(&format!("{}\n\n", desc));
    }

    content.push_str(&format!("**Priority:** {:?}\n", spec.priority));
    content.push_str(&format!("**Status:** {:?}\n\n", spec.status));

    // Acceptance criteria
    if !spec.acceptance_criteria.is_empty() {
        content.push_str("## Acceptance Criteria\n\n");
        for (i, criterion) in spec.acceptance_criteria.iter().enumerate() {
            content.push_str(&format!("{}. {}\n", i + 1, criterion));
        }
        content.push('\n');
    }

    // Dependencies
    if !spec.dependencies.is_empty() {
        content.push_str("## Dependencies\n\n");
        for dep in &spec.dependencies {
            content.push_str(&format!("- {}\n", dep));
        }
        content.push('\n');
    }

    // Implementation steps (generated from title + description)
    content.push_str("## Steps\n\n");
    content.push_str("1. Understand the requirements above\n");
    content.push_str("2. Review related code and dependencies\n");
    content.push_str("3. Implement the changes\n");
    content.push_str("4. Verify acceptance criteria are met\n");
    content.push_str("5. Update task status when complete\n");

    Ok(GeneratedSkill {
        dirname,
        filename,
        content,
        spec_title: spec.title,
    })
}

/// Generate skills for all approved/active specs in a project.
pub async fn generate_all_skills(db: &DbPool, project_id: &str) -> Result<Vec<GeneratedSkill>> {
    let specs = cwa_db::queries::specs::list_specs(db, project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list specs: {}", e))?;

    let mut skills = Vec::new();
    for spec in &specs {
        // Only generate skills for active/approved specs
        if spec.status == "active" || spec.status == "approved" {
            skills.push(generate_skill(db, project_id, &spec.id).await?);
        }
    }

    Ok(skills)
}

/// Write generated skills to disk.
pub fn write_skills(skills: &[GeneratedSkill], output_dir: &Path) -> Result<Vec<String>> {
    let mut written = Vec::new();

    for skill in skills {
        let skill_dir = output_dir.join(&skill.dirname);
        std::fs::create_dir_all(&skill_dir)?;

        let path = skill_dir.join(&skill.filename);
        std::fs::write(&path, &skill.content)?;
        written.push(path.display().to_string());
    }

    Ok(written)
}

/// Generate the built-in default skills (always created by `cwa codegen all`).
///
/// These skills capture key SDD/TDD/DDD workflows and are included in every
/// project regardless of tech stack.
pub fn generate_default_skills() -> Vec<GeneratedSkill> {
    vec![
        GeneratedSkill {
            dirname: "write-spec".to_string(),
            filename: "SKILL.md".to_string(),
            spec_title: "Write a Spec".to_string(),
            content: r#"# Write a Spec

Create a high-quality Specification-Driven Development (SDD) spec that captures
business requirements as testable acceptance criteria.

## When to Use

Use this skill when you need to define a new feature, behaviour change, or
business rule before implementation begins.

## Steps

1. **Identify the business need** — ask: why does this feature exist? who benefits?
2. **Name the spec** — use an imperative verb phrase: "Allow users to reset their password"
3. **Write the description** — explain the business context in 1-3 sentences
4. **Define acceptance criteria** — each criterion must be:
   - Testable: can be verified with an automated test
   - Specific: no vague terms ("fast", "intuitive", "correct")
   - Using Given-When-Then: "Given [state], When [action], Then [outcome]"
5. **Set priority** — high / medium / low based on business impact
6. **Link to bounded context** — associate with the correct DDD context
7. **Create the spec** using MCP tool `cwa_create_spec`
8. **Validate** using `cwa_validate_spec` — fix any issues flagged
9. **Review** using the spec-reviewer agent before moving to active

## Acceptance Criteria Template

```
Given <initial context>
When <event or action occurs>
Then <expected outcome>
And <additional expected outcome>
```

## Anti-Patterns to Avoid

- ✗ "The system should work correctly" (not testable)
- ✗ "Performance should be acceptable" (not specific)
- ✗ Mixing implementation details into criteria ("use Redis for caching")
- ✗ One giant spec — split into smaller, independently deliverable specs
"#.to_string(),
        },
        GeneratedSkill {
            dirname: "run-tdd-cycle".to_string(),
            filename: "SKILL.md".to_string(),
            spec_title: "Run TDD Cycle".to_string(),
            content: r#"# Run TDD Cycle

Execute the Red-Green-Refactor TDD cycle for a CWA task, linking each step
to the task board for full traceability.

## When to Use

Use this skill when starting work on a task that has clear acceptance criteria.

## Steps

### 1. RED — Write a Failing Test

1. Get the task details: `cwa_get_current_task`
2. Identify which acceptance criterion to implement first
3. Write a test that captures the criterion — it MUST fail
4. Commit: `git commit -m "test: [task-title] failing test for [criterion]"`

### 2. GREEN — Make It Pass

5. Write the minimum code to make the test pass (no gold-plating)
6. Run the test suite — ensure only this test was fixed
7. Commit: `git commit -m "feat: [task-title] implement [criterion]"`

### 3. REFACTOR — Clean Up

8. Improve code structure without changing behaviour
9. Run full test suite — all tests must still pass
10. Commit: `git commit -m "refactor: [task-title] clean up [component]"`

### 4. REPEAT

11. Pick the next acceptance criterion and go back to step 3

### 5. DONE

12. All criteria implemented and tested
13. Move task to review: `cwa_update_task_status(task_id, "review")`

## Rules

- Each commit must be green (all tests pass)
- Never skip the refactor step — technical debt accumulates fast
- Tests must be fast (<500ms per test) — mock external dependencies
- One test at a time — do not write multiple failing tests simultaneously
"#.to_string(),
        },
        GeneratedSkill {
            dirname: "domain-discovery".to_string(),
            filename: "SKILL.md".to_string(),
            spec_title: "Domain Discovery".to_string(),
            content: r#"# Domain Discovery

Discover the domain model using Event Storming, then encode findings into CWA
bounded contexts, domain objects, and ubiquitous language terms.

## When to Use

Use when starting a new project or feature area where the domain is poorly understood.

## Steps

### Phase 1: Event Storming

1. List all **Domain Events** (things that happened — past tense):
   - Example: `OrderPlaced`, `PaymentProcessed`, `ShipmentDispatched`
2. Identify **Commands** that trigger each event:
   - Example: `PlaceOrder` → `OrderPlaced`
3. Group events into natural **clusters** — these become Bounded Contexts
4. Identify **Aggregates** that handle commands:
   - Example: `Order` aggregate handles `PlaceOrder`

### Phase 2: Bounded Context Design

5. For each cluster, create a bounded context: `cwa_create_context`
   - Name: Clear noun phrase ("Ordering", "Payments", "Shipping")
   - Description: What business capability does this context own?
   - Responsibilities: 3-5 bullet points
6. Map relationships between contexts (upstream → downstream):
   - Partnership: both change together
   - Customer-Supplier: upstream serves downstream
   - Conformist: downstream conforms to upstream
   - ACL: downstream translates upstream's model

### Phase 3: Domain Object Definition

7. For each bounded context, create domain objects: `cwa_create_domain_object`
   - Aggregates: consistency boundaries (e.g., `Order`)
   - Entities: have identity and lifecycle (e.g., `OrderLine`)
   - Value Objects: immutable, equality by value (e.g., `Money`)
   - Domain Events: things that happened (e.g., `OrderPlaced`)
   - Services: stateless domain operations

### Phase 4: Ubiquitous Language

8. For each key term, add a glossary entry: `cwa_add_glossary_term`
   - Use the term EXACTLY as business people use it
   - Include synonyms to avoid confusion
   - Note if the same word means different things in different contexts

## Output

- `cwa domain context list` — all bounded contexts
- `cwa domain object list <context-id>` — all objects per context
- `cwa domain glossary list` — ubiquitous language
"#.to_string(),
        },
    ]
}

/// Convert a name to a URL-safe slug.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

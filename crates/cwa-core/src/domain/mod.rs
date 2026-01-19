//! Domain modeling (DDD).

pub mod model;

use crate::error::CwaResult;
use cwa_db::DbPool;
use cwa_db::queries::domains as queries;
use model::{BoundedContext, DomainObject, GlossaryTerm, DomainModel, ContextMap};
use uuid::Uuid;

/// Create a bounded context.
pub fn create_context(
    pool: &DbPool,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> CwaResult<BoundedContext> {
    let id = Uuid::new_v4().to_string();

    queries::create_context(pool, &id, project_id, name, description)?;

    let row = queries::get_context(pool, &id)?;
    Ok(BoundedContext::from_row(row))
}

/// Get a bounded context by ID.
pub fn get_context(pool: &DbPool, id: &str) -> CwaResult<BoundedContext> {
    let row = queries::get_context(pool, id)?;
    Ok(BoundedContext::from_row(row))
}

/// List all bounded contexts for a project.
pub fn list_contexts(pool: &DbPool, project_id: &str) -> CwaResult<Vec<BoundedContext>> {
    let rows = queries::list_contexts(pool, project_id)?;
    Ok(rows.into_iter().map(BoundedContext::from_row).collect())
}

/// Create a domain object.
pub fn create_domain_object(
    pool: &DbPool,
    context_id: &str,
    name: &str,
    object_type: &str,
    description: Option<&str>,
) -> CwaResult<()> {
    let id = Uuid::new_v4().to_string();
    queries::create_domain_object(pool, &id, context_id, name, object_type, description)?;
    Ok(())
}

/// List domain objects for a context.
pub fn list_domain_objects(pool: &DbPool, context_id: &str) -> CwaResult<Vec<DomainObject>> {
    let rows = queries::list_domain_objects(pool, context_id)?;
    Ok(rows.into_iter().map(DomainObject::from_row).collect())
}

/// Add a glossary term.
pub fn add_glossary_term(
    pool: &DbPool,
    project_id: &str,
    term: &str,
    definition: &str,
    context_id: Option<&str>,
) -> CwaResult<()> {
    let id = Uuid::new_v4().to_string();
    queries::create_glossary_term(pool, &id, project_id, term, definition, context_id)?;
    Ok(())
}

/// List glossary terms.
pub fn list_glossary(pool: &DbPool, project_id: &str) -> CwaResult<Vec<GlossaryTerm>> {
    let rows = queries::list_glossary(pool, project_id)?;
    Ok(rows.into_iter().map(GlossaryTerm::from_row).collect())
}

/// Get the complete domain model for a project.
pub fn get_domain_model(pool: &DbPool, project_id: &str) -> CwaResult<DomainModel> {
    let contexts = list_contexts(pool, project_id)?;

    let mut contexts_with_objects = Vec::new();
    for context in contexts {
        let objects = list_domain_objects(pool, &context.id)?;
        contexts_with_objects.push(model::ContextWithObjects {
            context,
            objects,
        });
    }

    let glossary = list_glossary(pool, project_id)?;

    Ok(DomainModel {
        contexts: contexts_with_objects,
        glossary,
    })
}

/// Get the context map (relationships between contexts).
pub fn get_context_map(pool: &DbPool, project_id: &str) -> CwaResult<ContextMap> {
    let contexts = list_contexts(pool, project_id)?;

    let mut relationships = Vec::new();
    for context in &contexts {
        for downstream_id in &context.downstream_contexts {
            relationships.push(model::ContextRelationship {
                upstream_id: context.id.clone(),
                downstream_id: downstream_id.clone(),
                relationship_type: "customer-supplier".to_string(),
            });
        }
    }

    Ok(ContextMap {
        contexts: contexts.into_iter().map(|c| c.name).collect(),
        relationships,
    })
}

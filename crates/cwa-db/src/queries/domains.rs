//! Domain-related database queries (Bounded Contexts, Domain Objects).

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Bounded context row from database.
#[derive(Debug, Clone)]
pub struct BoundedContextRow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub responsibilities: Option<String>,
    pub upstream_contexts: Option<String>,
    pub downstream_contexts: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Domain object row from database.
#[derive(Debug, Clone)]
pub struct DomainObjectRow {
    pub id: String,
    pub context_id: String,
    pub name: String,
    pub object_type: String,
    pub description: Option<String>,
    pub properties: Option<String>,
    pub behaviors: Option<String>,
    pub invariants: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Glossary term row from database.
#[derive(Debug, Clone)]
pub struct GlossaryTermRow {
    pub id: String,
    pub project_id: String,
    pub context_id: Option<String>,
    pub term: String,
    pub definition: String,
    pub aliases: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Create a bounded context.
pub fn create_context(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO bounded_contexts (id, project_id, name, description)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, project_id, name, description],
        )?;
        Ok(())
    })
}

/// Get a bounded context by ID.
pub fn get_context(pool: &DbPool, id: &str) -> DbResult<BoundedContextRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, project_id, name, description, responsibilities,
                    upstream_contexts, downstream_contexts, created_at, updated_at
             FROM bounded_contexts WHERE id = ?1",
            params![id],
            |row| {
                Ok(BoundedContextRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    responsibilities: row.get(4)?,
                    upstream_contexts: row.get(5)?,
                    downstream_contexts: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Context: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// List all bounded contexts for a project.
pub fn list_contexts(pool: &DbPool, project_id: &str) -> DbResult<Vec<BoundedContextRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, responsibilities,
                    upstream_contexts, downstream_contexts, created_at, updated_at
             FROM bounded_contexts WHERE project_id = ?1 ORDER BY name",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(BoundedContextRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                responsibilities: row.get(4)?,
                upstream_contexts: row.get(5)?,
                downstream_contexts: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Create a domain object.
pub fn create_domain_object(
    pool: &DbPool,
    id: &str,
    context_id: &str,
    name: &str,
    object_type: &str,
    description: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO domain_objects (id, context_id, name, object_type, description)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, context_id, name, object_type, description],
        )?;
        Ok(())
    })
}

/// List domain objects for a context.
pub fn list_domain_objects(pool: &DbPool, context_id: &str) -> DbResult<Vec<DomainObjectRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, context_id, name, object_type, description,
                    properties, behaviors, invariants, created_at, updated_at
             FROM domain_objects WHERE context_id = ?1 ORDER BY object_type, name",
        )?;

        let rows = stmt.query_map(params![context_id], |row| {
            Ok(DomainObjectRow {
                id: row.get(0)?,
                context_id: row.get(1)?,
                name: row.get(2)?,
                object_type: row.get(3)?,
                description: row.get(4)?,
                properties: row.get(5)?,
                behaviors: row.get(6)?,
                invariants: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Create a glossary term.
pub fn create_glossary_term(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    term: &str,
    definition: &str,
    context_id: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO glossary (id, project_id, term, definition, context_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, project_id, term, definition, context_id],
        )?;
        Ok(())
    })
}

/// List all glossary terms for a project.
pub fn list_glossary(pool: &DbPool, project_id: &str) -> DbResult<Vec<GlossaryTermRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, context_id, term, definition, aliases, created_at, updated_at
             FROM glossary WHERE project_id = ?1 ORDER BY term",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(GlossaryTermRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                context_id: row.get(2)?,
                term: row.get(3)?,
                definition: row.get(4)?,
                aliases: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

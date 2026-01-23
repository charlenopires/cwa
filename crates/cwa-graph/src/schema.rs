//! Neo4j schema initialization (constraints and indexes).

use anyhow::Result;
use neo4rs::Query;
use tracing::info;

use crate::GraphClient;

/// Cypher statements for schema initialization.
const SCHEMA_STATEMENTS: &[&str] = &[
    // Uniqueness constraints
    "CREATE CONSTRAINT project_id IF NOT EXISTS FOR (p:Project) REQUIRE p.id IS UNIQUE",
    "CREATE CONSTRAINT spec_id IF NOT EXISTS FOR (s:Spec) REQUIRE s.id IS UNIQUE",
    "CREATE CONSTRAINT task_id IF NOT EXISTS FOR (t:Task) REQUIRE t.id IS UNIQUE",
    "CREATE CONSTRAINT context_id IF NOT EXISTS FOR (c:BoundedContext) REQUIRE c.id IS UNIQUE",
    "CREATE CONSTRAINT entity_id IF NOT EXISTS FOR (e:DomainEntity) REQUIRE e.id IS UNIQUE",
    "CREATE CONSTRAINT term_name IF NOT EXISTS FOR (t:Term) REQUIRE t.name IS UNIQUE",
    "CREATE CONSTRAINT decision_id IF NOT EXISTS FOR (d:Decision) REQUIRE d.id IS UNIQUE",
    "CREATE CONSTRAINT memory_id IF NOT EXISTS FOR (m:Memory) REQUIRE m.id IS UNIQUE",
    // Full-text search indexes
    "CREATE FULLTEXT INDEX spec_search IF NOT EXISTS FOR (s:Spec) ON EACH [s.title, s.description]",
    "CREATE FULLTEXT INDEX term_search IF NOT EXISTS FOR (t:Term) ON EACH [t.name, t.definition]",
    "CREATE FULLTEXT INDEX memory_search IF NOT EXISTS FOR (m:Memory) ON EACH [m.content, m.context]",
];

/// Initialize Neo4j schema with constraints and indexes.
///
/// Safe to run multiple times - uses IF NOT EXISTS clauses.
pub async fn initialize_schema(client: &GraphClient) -> Result<()> {
    info!("Initializing Neo4j schema...");

    for statement in SCHEMA_STATEMENTS {
        client.execute(Query::new(statement.to_string())).await?;
    }

    info!("Neo4j schema initialized ({} statements)", SCHEMA_STATEMENTS.len());
    Ok(())
}

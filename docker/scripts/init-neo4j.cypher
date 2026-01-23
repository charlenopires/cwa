// CWA Knowledge Graph Schema Initialization
// Run after Neo4j is healthy: cypher-shell -f init-neo4j.cypher

// Uniqueness Constraints
CREATE CONSTRAINT project_id IF NOT EXISTS
FOR (p:Project) REQUIRE p.id IS UNIQUE;

CREATE CONSTRAINT spec_id IF NOT EXISTS
FOR (s:Spec) REQUIRE s.id IS UNIQUE;

CREATE CONSTRAINT task_id IF NOT EXISTS
FOR (t:Task) REQUIRE t.id IS UNIQUE;

CREATE CONSTRAINT context_id IF NOT EXISTS
FOR (c:BoundedContext) REQUIRE c.id IS UNIQUE;

CREATE CONSTRAINT entity_id IF NOT EXISTS
FOR (e:DomainEntity) REQUIRE e.id IS UNIQUE;

CREATE CONSTRAINT term_name IF NOT EXISTS
FOR (t:Term) REQUIRE t.name IS UNIQUE;

CREATE CONSTRAINT decision_id IF NOT EXISTS
FOR (d:Decision) REQUIRE d.id IS UNIQUE;

CREATE CONSTRAINT memory_id IF NOT EXISTS
FOR (m:Memory) REQUIRE m.id IS UNIQUE;

// Full-text Search Indexes
CREATE FULLTEXT INDEX spec_search IF NOT EXISTS
FOR (s:Spec) ON EACH [s.title, s.description];

CREATE FULLTEXT INDEX term_search IF NOT EXISTS
FOR (t:Term) ON EACH [t.name, t.definition];

CREATE FULLTEXT INDEX memory_search IF NOT EXISTS
FOR (m:Memory) ON EACH [m.content, m.context];

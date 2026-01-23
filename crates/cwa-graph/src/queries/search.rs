//! Full-text search queries against Neo4j indexes.
//!
//! Uses the fulltext indexes created in schema.rs:
//! - spec_search (title, description)
//! - term_search (name, definition)
//! - memory_search (content, context)

use anyhow::Result;
use neo4rs::Query;
use serde::Serialize;

use crate::GraphClient;

/// A search result from the graph.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub label: String,
    pub name: String,
    pub score: f64,
    pub snippet: String,
}

/// Search specs by title or description.
pub async fn search_specs(client: &GraphClient, query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let query = Query::new(
        "CALL db.index.fulltext.queryNodes('spec_search', $query)
         YIELD node, score
         RETURN node.id as id, 'Spec' as label, node.title as name,
                score, COALESCE(node.description, '') as snippet
         LIMIT $limit"
            .to_string(),
    )
    .param("query", query_text)
    .param("limit", limit as i64);

    parse_search_results(client.query(query).await?)
}

/// Search glossary terms by name or definition.
pub async fn search_terms(client: &GraphClient, query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let query = Query::new(
        "CALL db.index.fulltext.queryNodes('term_search', $query)
         YIELD node, score
         RETURN node.name as id, 'Term' as label, node.name as name,
                score, COALESCE(node.definition, '') as snippet
         LIMIT $limit"
            .to_string(),
    )
    .param("query", query_text)
    .param("limit", limit as i64);

    parse_search_results(client.query(query).await?)
}

/// Search memories by content or context.
pub async fn search_memories(client: &GraphClient, query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let query = Query::new(
        "CALL db.index.fulltext.queryNodes('memory_search', $query)
         YIELD node, score
         RETURN node.id as id, 'Memory' as label,
                substring(node.content, 0, 80) as name,
                score, COALESCE(node.context, '') as snippet
         LIMIT $limit"
            .to_string(),
    )
    .param("query", query_text)
    .param("limit", limit as i64);

    parse_search_results(client.query(query).await?)
}

/// Search across all indexed entities.
pub async fn search_all(client: &GraphClient, query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    // Search each index and merge results
    let per_index_limit = (limit / 3).max(5);

    let specs = search_specs(client, query_text, per_index_limit).await?;
    results.extend(specs);

    let terms = search_terms(client, query_text, per_index_limit).await?;
    results.extend(terms);

    let memories = search_memories(client, query_text, per_index_limit).await?;
    results.extend(memories);

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Trim to limit
    results.truncate(limit);

    Ok(results)
}

/// Execute a raw Cypher query and return results as JSON.
pub async fn raw_query(client: &GraphClient, cypher: &str) -> Result<Vec<serde_json::Value>> {
    let query = Query::new(cypher.to_string());
    let rows = client.query(query).await?;

    let mut results = Vec::new();
    for row in rows {
        // Convert each row to a JSON value by iterating known field patterns
        // Neo4rs Row doesn't have a generic "all fields" method,
        // so we return a simplified representation
        let value = serde_json::json!({
            "row": format!("{:?}", row)
        });
        results.push(value);
    }

    Ok(results)
}

fn parse_search_results(rows: Vec<neo4rs::Row>) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    for row in rows {
        let id: String = row.get("id").unwrap_or_default();
        let label: String = row.get("label").unwrap_or_default();
        let name: String = row.get("name").unwrap_or_default();
        let score: f64 = row.get("score").unwrap_or(0.0);
        let snippet: String = row.get("snippet").unwrap_or_default();

        if !id.is_empty() {
            results.push(SearchResult {
                id,
                label,
                name,
                score,
                snippet,
            });
        }
    }

    Ok(results)
}

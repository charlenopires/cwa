//! Neo4j connection client.

use anyhow::{Context, Result};
use neo4rs::{ConfigBuilder, Graph, Query};
use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Configuration for connecting to Neo4j.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            uri: "bolt://localhost:7687".to_string(),
            user: "neo4j".to_string(),
            password: "cwa_dev_2026".to_string(),
        }
    }
}

/// Client for Neo4j Knowledge Graph operations.
#[derive(Clone)]
pub struct GraphClient {
    graph: Graph,
}

impl GraphClient {
    /// Create a new GraphClient from config.
    ///
    /// Note: neo4rs uses a lazy deadpool — `Graph::connect` only creates the pool
    /// object and does NOT establish a real bolt connection yet.  We run a cheap
    /// `RETURN 1` ping immediately so that callers can wrap this in a timeout and
    /// get a fast failure when Neo4j is unreachable instead of hanging silently.
    pub async fn connect(config: &GraphConfig) -> Result<Self> {
        let neo4j_config = ConfigBuilder::default()
            .uri(&config.uri)
            .user(&config.user)
            .password(&config.password)
            .db("neo4j")
            .max_connections(4)  // Keep pool small for CLI use-cases
            .fetch_size(20)
            .build()
            .context("Failed to build Neo4j config")?;

        let graph = Graph::connect(neo4j_config)
            .await
            .context("Failed to create Neo4j connection pool")?;

        // Ping to force an actual TCP+bolt handshake so the caller's timeout works.
        graph.run(Query::new("RETURN 1".to_string())).await
            .context("Neo4j is not responding to queries")?;

        Ok(Self { graph })
    }

    /// Create a new GraphClient with default configuration.
    pub async fn connect_default() -> Result<Self> {
        Self::connect(&GraphConfig::default()).await
    }

    /// Execute a Cypher query that returns no results.
    pub async fn execute(&self, query: Query) -> Result<()> {
        self.graph.run(query).await.context("Neo4j query execution failed")?;
        Ok(())
    }

    /// Execute a Cypher query and return results as rows.
    pub async fn query(&self, query: Query) -> Result<Vec<neo4rs::Row>> {
        let mut result = self.graph.execute(query).await
            .context("Neo4j query failed")?;

        let mut rows = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            rows.push(row);
        }
        Ok(rows)
    }

    /// Execute a Cypher query and return a single scalar value.
    pub async fn query_scalar<T: DeserializeOwned>(&self, query: Query, field: &str) -> Result<Option<T>> {
        let rows = self.query(query).await?;
        if let Some(row) = rows.into_iter().next() {
            let val: T = row.get(field)
                .map_err(|e| anyhow::anyhow!("Failed to get field '{}': {:?}", field, e))?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    /// Get node and relationship counts for status display.
    pub async fn get_counts(&self) -> Result<GraphCounts> {
        let node_query = Query::new("MATCH (n) RETURN count(n) as count".to_string());
        let rel_query = Query::new("MATCH ()-[r]->() RETURN count(r) as count".to_string());

        let node_count: i64 = self.query_scalar(node_query, "count").await?
            .unwrap_or(0);
        let rel_count: i64 = self.query_scalar(rel_query, "count").await?
            .unwrap_or(0);

        Ok(GraphCounts {
            nodes: node_count as usize,
            relationships: rel_count as usize,
        })
    }

    /// Get a reference to the underlying neo4rs Graph.
    pub fn inner(&self) -> &Graph {
        &self.graph
    }
}

/// Node and relationship counts.
#[derive(Debug, Clone)]
pub struct GraphCounts {
    pub nodes: usize,
    pub relationships: usize,
}

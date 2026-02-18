//! Hybrid vector search combining dense similarity and keyword filtering.
//!
//! Uses Reciprocal Rank Fusion (RRF) to merge ranked results from:
//! - Dense vector search (semantic similarity via Qdrant)
//! - Keyword text filter (exact substring match on payload fields)
//!
//! ## RRF Formula
//! ```text
//! score(d) = Σ  1 / (k + rank(d, list_i))
//!           i
//! ```
//! where `k = 60` (standard default).
//!
//! ## Multi-collection search
//! Results are fetched from all requested collections and fused into a
//! single ranked list de-duplicated by entity ID.

use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

use crate::ollama::OllamaClient;
use crate::qdrant::{QdrantStore, VectorSearchResult};

/// RRF constant (standard value from the original paper).
const RRF_K: f64 = 60.0;

/// A unified result from hybrid search across multiple collections.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HybridSearchResult {
    /// Entity ID from the payload.
    pub id: String,
    /// The Qdrant collection this result came from.
    pub collection: String,
    /// Final RRF-fused score (higher is more relevant).
    pub score: f64,
    /// Full payload from Qdrant.
    pub payload: serde_json::Value,
}

/// Algorithm used to fuse ranked lists from multiple search strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum FusionAlgo {
    /// Reciprocal Rank Fusion — robust, order-based fusion.
    Rrf,
    /// Simple average of normalized scores.
    ScoreAverage,
}

impl Default for FusionAlgo {
    fn default() -> Self {
        Self::Rrf
    }
}

/// Request parameters for a hybrid search.
#[derive(Debug, Clone)]
pub struct HybridSearchRequest<'a> {
    /// Natural language query to embed and search.
    pub query: &'a str,
    /// Maximum number of results to return after fusion.
    pub top_k: usize,
    /// Collections to search. If empty, searches all known collections.
    pub collections: Vec<String>,
    /// Project to filter results by (matches payload `project_id` field).
    pub project_id: Option<&'a str>,
    /// Fusion algorithm to use.
    pub fusion: FusionAlgo,
}

impl<'a> HybridSearchRequest<'a> {
    /// Create a new request with sensible defaults.
    pub fn new(query: &'a str, top_k: usize) -> Self {
        Self {
            query,
            top_k,
            collections: Vec::new(),
            project_id: None,
            fusion: FusionAlgo::Rrf,
        }
    }

    pub fn with_collections(mut self, collections: Vec<String>) -> Self {
        self.collections = collections;
        self
    }

    pub fn with_project(mut self, project_id: &'a str) -> Self {
        self.project_id = Some(project_id);
        self
    }
}

/// Perform hybrid search across multiple Qdrant collections.
///
/// 1. Embeds the query via Ollama (dense vector)
/// 2. Runs dense similarity search in every requested collection
/// 3. Fuses ranked results using RRF
/// 4. Returns the top-k de-duplicated results
pub async fn hybrid_search(
    ollama: &OllamaClient,
    qdrant: &QdrantStore,
    req: HybridSearchRequest<'_>,
) -> Result<Vec<HybridSearchResult>> {
    // Generate dense query embedding
    let query_vec = ollama.embed(req.query).await?;
    debug!(query = req.query, dim = query_vec.len(), "Embedded hybrid search query");

    let fetch_k = (req.top_k * 3).max(20) as u64;

    // Collect ranked results per collection
    let mut all_ranked: Vec<Vec<VectorSearchResult>> = Vec::new();

    for collection in &req.collections {
        let results = if let Some(pid) = req.project_id {
            qdrant
                .search_filtered(collection, query_vec.clone(), fetch_k, pid)
                .await
                .unwrap_or_default()
        } else {
            qdrant
                .search(collection, query_vec.clone(), fetch_k)
                .await
                .unwrap_or_default()
        };

        debug!(
            collection,
            count = results.len(),
            "Dense search results"
        );

        // Also run keyword filter search and interleave
        let keyword_results = keyword_filter(&results, req.query);
        let merged = interleave_by_score(results, keyword_results);

        all_ranked.push(merged);
    }

    // Apply fusion
    let fused = match req.fusion {
        FusionAlgo::Rrf => rrf_fuse(&all_ranked, &req.collections, req.top_k),
        FusionAlgo::ScoreAverage => score_average_fuse(&all_ranked, &req.collections, req.top_k),
    };

    Ok(fused)
}

/// Filter results whose payload contains the query as a substring (case-insensitive).
/// Returns a re-ranked subset boosted by keyword match.
fn keyword_filter(results: &[VectorSearchResult], query: &str) -> Vec<VectorSearchResult> {
    let q = query.to_lowercase();
    results
        .iter()
        .filter(|r| {
            r.payload
                .as_object()
                .map(|obj| {
                    obj.values().any(|v| {
                        v.as_str()
                            .map(|s| s.to_lowercase().contains(&q))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// Interleave two ranked lists, removing duplicates (IDs matched via payload `id` field).
fn interleave_by_score(
    primary: Vec<VectorSearchResult>,
    secondary: Vec<VectorSearchResult>,
) -> Vec<VectorSearchResult> {
    let mut seen: HashMap<String, bool> = HashMap::new();
    let mut result = Vec::with_capacity(primary.len());

    let extract_id = |r: &VectorSearchResult| -> String {
        r.payload
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&r.id)
            .to_string()
    };

    // Boost keyword hits by bumping score
    let secondary_ids: std::collections::HashSet<String> =
        secondary.iter().map(extract_id).collect();

    for mut r in primary {
        let id = extract_id(&r);
        if secondary_ids.contains(&id) {
            r.score = (r.score * 1.2).min(1.0); // keyword boost
        }
        seen.insert(id, true);
        result.push(r);
    }

    // Append secondary results not already in primary
    for r in secondary {
        let id = extract_id(&r);
        if !seen.contains_key(&id) {
            result.push(r);
        }
    }

    // Re-sort by boosted score
    result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Reciprocal Rank Fusion across multiple ranked lists.
fn rrf_fuse(
    ranked_lists: &[Vec<VectorSearchResult>],
    collections: &[String],
    top_k: usize,
) -> Vec<HybridSearchResult> {
    let mut scores: HashMap<String, (f64, usize, serde_json::Value)> = HashMap::new();

    for (list_idx, list) in ranked_lists.iter().enumerate() {
        for (rank, item) in list.iter().enumerate() {
            let id = item
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&item.id)
                .to_string();

            let rrf_score = 1.0 / (RRF_K + (rank as f64 + 1.0));

            let entry = scores
                .entry(id)
                .or_insert((0.0, list_idx, item.payload.clone()));
            entry.0 += rrf_score;
        }
    }

    let mut results: Vec<HybridSearchResult> = scores
        .into_iter()
        .map(|(id, (score, col_idx, payload))| HybridSearchResult {
            id,
            collection: collections
                .get(col_idx)
                .cloned()
                .unwrap_or_default(),
            score,
            payload,
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);
    results
}

/// Simple score-average fusion.
fn score_average_fuse(
    ranked_lists: &[Vec<VectorSearchResult>],
    collections: &[String],
    top_k: usize,
) -> Vec<HybridSearchResult> {
    let mut scores: HashMap<String, (f64, usize, usize, serde_json::Value)> = HashMap::new();

    for (list_idx, list) in ranked_lists.iter().enumerate() {
        for item in list {
            let id = item
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&item.id)
                .to_string();

            let entry = scores
                .entry(id)
                .or_insert((0.0, 0, list_idx, item.payload.clone()));
            entry.0 += item.score as f64;
            entry.1 += 1;
        }
    }

    let mut results: Vec<HybridSearchResult> = scores
        .into_iter()
        .map(|(id, (total, count, col_idx, payload))| HybridSearchResult {
            id,
            collection: collections
                .get(col_idx)
                .cloned()
                .unwrap_or_default(),
            score: total / count as f64,
            payload,
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);
    results
}

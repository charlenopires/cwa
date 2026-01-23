//! Qdrant vector store client.
//!
//! Manages collections, upserts vectors, and performs similarity search
//! against the Qdrant service via the qdrant-client gRPC library.

use anyhow::{Context, Result};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, VectorParamsBuilder,
    PointStruct, UpsertPointsBuilder, SearchPointsBuilder,
    value::Kind, Value,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use uuid::Uuid;

use crate::ollama::EMBEDDING_DIM;

/// Default Qdrant gRPC URL.
pub const DEFAULT_QDRANT_URL: &str = "http://localhost:6334";

/// Collection name for memories.
pub const MEMORIES_COLLECTION: &str = "cwa_memories";

/// Collection name for terms.
pub const TERMS_COLLECTION: &str = "cwa_terms";

/// A search result from Qdrant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub payload: serde_json::Value,
}

/// Qdrant vector store client for CWA.
#[derive(Clone)]
pub struct QdrantStore {
    client: Qdrant,
}

impl QdrantStore {
    /// Create a new QdrantStore client.
    pub fn new(url: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .context("Failed to create Qdrant client")?;

        Ok(Self { client })
    }

    /// Create a store with default settings (localhost:6334).
    pub fn default_store() -> Result<Self> {
        Self::new(DEFAULT_QDRANT_URL)
    }

    /// Ensure the memories collection exists with the correct configuration.
    pub async fn ensure_collection(&self, collection_name: &str) -> Result<()> {
        let exists = self.client
            .collection_exists(collection_name)
            .await
            .context("Failed to check collection")?;

        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(collection_name)
                        .vectors_config(VectorParamsBuilder::new(
                            EMBEDDING_DIM as u64,
                            Distance::Cosine,
                        )),
                )
                .await
                .context("Failed to create collection")?;

            info!(collection = collection_name, "Created Qdrant collection");
        } else {
            debug!(collection = collection_name, "Collection already exists");
        }

        Ok(())
    }

    /// Initialize all required collections.
    pub async fn init_collections(&self) -> Result<()> {
        self.ensure_collection(MEMORIES_COLLECTION).await?;
        self.ensure_collection(TERMS_COLLECTION).await?;
        Ok(())
    }

    /// Upsert a vector with payload into a collection.
    pub async fn upsert(
        &self,
        collection: &str,
        id: &str,
        vector: Vec<f32>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let point_id = uuid_to_point_id(id);
        let qdrant_payload = json_to_payload(&payload);

        let point = PointStruct::new(point_id, vector, qdrant_payload);

        self.client
            .upsert_points(UpsertPointsBuilder::new(collection, vec![point]))
            .await
            .context("Failed to upsert point")?;

        debug!(collection, id, "Upserted vector");
        Ok(())
    }

    /// Search for similar vectors in a collection.
    pub async fn search(
        &self,
        collection: &str,
        query_vector: Vec<f32>,
        top_k: u64,
    ) -> Result<Vec<VectorSearchResult>> {
        let response = self.client
            .search_points(
                SearchPointsBuilder::new(collection, query_vector, top_k)
                    .with_payload(true),
            )
            .await
            .context("Failed to search points")?;

        let results = response.result.into_iter().map(|point| {
            let id = match point.id {
                Some(id) => format!("{:?}", id),
                None => String::new(),
            };
            let payload = payload_to_json(&point.payload);
            VectorSearchResult {
                id,
                score: point.score,
                payload,
            }
        }).collect();

        Ok(results)
    }

    /// Search with a filter on a payload field.
    pub async fn search_filtered(
        &self,
        collection: &str,
        query_vector: Vec<f32>,
        top_k: u64,
        project_id: &str,
    ) -> Result<Vec<VectorSearchResult>> {
        let response = self.client
            .search_points(
                SearchPointsBuilder::new(collection, query_vector, top_k)
                    .with_payload(true)
                    .filter(qdrant_client::qdrant::Filter::must([
                        qdrant_client::qdrant::Condition::matches("project_id", project_id.to_string()),
                    ])),
            )
            .await
            .context("Failed to search points with filter")?;

        let results = response.result.into_iter().map(|point| {
            let id = match point.id {
                Some(id) => format!("{:?}", id),
                None => String::new(),
            };
            let payload = payload_to_json(&point.payload);
            VectorSearchResult {
                id,
                score: point.score,
                payload,
            }
        }).collect();

        Ok(results)
    }

    /// Delete a point by ID from a collection.
    pub async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let point_id = uuid_to_point_id(id);

        use qdrant_client::qdrant::{DeletePointsBuilder, PointsIdsList, PointId, point_id::PointIdOptions};

        let ids_list = PointsIdsList {
            ids: vec![PointId {
                point_id_options: Some(PointIdOptions::Uuid(point_id)),
            }],
        };

        self.client
            .delete_points(
                DeletePointsBuilder::new(collection).points(ids_list)
            )
            .await
            .context("Failed to delete point")?;

        debug!(collection, id, "Deleted vector");
        Ok(())
    }

    /// Get the number of points in a collection.
    pub async fn count(&self, collection: &str) -> Result<u64> {
        let info = self.client
            .collection_info(collection)
            .await
            .context("Failed to get collection info")?;

        Ok(info.result
            .map(|r| r.points_count.unwrap_or(0))
            .unwrap_or(0))
    }
}

/// Convert a string ID to a UUID-based point ID for Qdrant.
fn uuid_to_point_id(id: &str) -> String {
    // If the id is already a valid UUID, use it directly
    if Uuid::parse_str(id).is_ok() {
        id.to_string()
    } else {
        // Generate a deterministic UUID from the string by hashing it
        // Use a simple approach: take first 16 bytes of the string padded/truncated
        let mut bytes = [0u8; 16];
        let id_bytes = id.as_bytes();
        for (i, b) in id_bytes.iter().enumerate().take(16) {
            bytes[i] = *b;
        }
        // XOR remaining bytes for longer strings
        for (i, b) in id_bytes.iter().enumerate().skip(16) {
            bytes[i % 16] ^= *b;
        }
        // Set version 4 and variant bits
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid::from_bytes(bytes).to_string()
    }
}

/// Convert a serde_json::Value to Qdrant payload (HashMap<String, Value>).
fn json_to_payload(json: &serde_json::Value) -> std::collections::HashMap<String, Value> {
    let mut payload = std::collections::HashMap::new();

    if let serde_json::Value::Object(map) = json {
        for (key, val) in map {
            if let Some(qdrant_val) = json_value_to_qdrant(val) {
                payload.insert(key.clone(), qdrant_val);
            }
        }
    }

    payload
}

/// Convert a serde_json value to a Qdrant Value.
fn json_value_to_qdrant(val: &serde_json::Value) -> Option<Value> {
    match val {
        serde_json::Value::String(s) => Some(Value {
            kind: Some(Kind::StringValue(s.clone())),
        }),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Some(Value {
                    kind: Some(Kind::DoubleValue(f)),
                })
            } else if let Some(i) = n.as_i64() {
                Some(Value {
                    kind: Some(Kind::IntegerValue(i)),
                })
            } else {
                None
            }
        }
        serde_json::Value::Bool(b) => Some(Value {
            kind: Some(Kind::BoolValue(*b)),
        }),
        _ => None,
    }
}

/// Convert Qdrant payload back to serde_json::Value.
fn payload_to_json(payload: &std::collections::HashMap<String, Value>) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for (key, val) in payload {
        if let Some(kind) = &val.kind {
            let json_val = match kind {
                Kind::StringValue(s) => serde_json::Value::String(s.clone()),
                Kind::DoubleValue(f) => serde_json::json!(*f),
                Kind::IntegerValue(i) => serde_json::json!(*i),
                Kind::BoolValue(b) => serde_json::Value::Bool(*b),
                _ => continue,
            };
            map.insert(key.clone(), json_val);
        }
    }

    serde_json::Value::Object(map)
}

//! # CWA Graph
//!
//! Neo4j Knowledge Graph integration for CWA.
//!
//! Provides synchronization of SQLite entities to Neo4j,
//! graph traversal queries, and impact analysis.

pub mod client;
pub mod hyperedge;
pub mod schema;
pub mod sync;
pub mod queries;

pub use client::{GraphClient, GraphConfig, GraphCounts};
pub use hyperedge::{EntityType, HyperedgeInfo, create_hyperedge, find_hyperedges_for_entity};
pub use sync::{SyncResult, run_full_sync, get_last_sync_time, save_last_sync_time};

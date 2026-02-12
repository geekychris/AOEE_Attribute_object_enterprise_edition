//! Storage trait definitions

use aoee_core::{EntityId, EdgeKey, EdgeType};
use async_trait::async_trait;
use thiserror::Error;

/// Storage errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Key not found: {0:?}")]
    NotFound(EdgeKey),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Stored edge with metadata
#[derive(Debug, Clone)]
pub struct StoredEdge {
    pub dst: EntityId,
    pub timestamp: u64,
    pub deleted: bool,
}

impl StoredEdge {
    pub fn new(dst: EntityId, timestamp: u64) -> Self {
        StoredEdge {
            dst,
            timestamp,
            deleted: false,
        }
    }

    pub fn deleted(dst: EntityId, timestamp: u64) -> Self {
        StoredEdge {
            dst,
            timestamp,
            deleted: true,
        }
    }
}

/// Trait for persistent edge storage.
///
/// Implementations of this trait provide durable storage for edges,
/// serving as the source of truth for the relationship graph.
/// The cache layer (AOEE) sits in front of this storage.
///
/// # Design Notes
/// - All operations are async to support network-based backends
/// - Edges are keyed by (src, edge_type) and store destination IDs with timestamps
/// - Timestamps enable conflict resolution (last-write-wins)
/// - Tombstones track deletions until compaction
#[async_trait]
pub trait EdgeStore: Send + Sync {
    /// Persist a new edge.
    ///
    /// If the edge already exists, this updates the timestamp.
    async fn persist_edge(&self, key: EdgeKey, dst: EntityId, timestamp: u64) -> Result<()>;

    /// Persist a deletion (tombstone).
    ///
    /// The tombstone will be stored until compaction removes it.
    async fn persist_delete(&self, key: EdgeKey, dst: EntityId, timestamp: u64) -> Result<()>;

    /// Load all edges for a key.
    ///
    /// Returns edges sorted by destination ID, with deletions filtered out
    /// (based on timestamps - a delete after an add means the edge is gone).
    async fn load_edges(&self, key: EdgeKey) -> Result<Vec<StoredEdge>>;

    /// Load edges for a key with pagination.
    ///
    /// Returns up to `limit` edges starting from `cursor` (exclusive).
    /// If cursor is None, starts from the beginning.
    async fn load_edges_paginated(
        &self,
        key: EdgeKey,
        cursor: Option<EntityId>,
        limit: usize,
    ) -> Result<Vec<StoredEdge>>;

    /// Check if a specific edge exists.
    async fn edge_exists(&self, key: EdgeKey, dst: EntityId) -> Result<bool>;

    /// Get the count of edges for a key (approximate is acceptable).
    async fn count_edges(&self, key: EdgeKey) -> Result<usize>;

    /// Scan all keys with an optional prefix filter.
    ///
    /// Used for cache warming and maintenance operations.
    async fn scan_keys(&self, prefix: Option<EntityId>) -> Result<Vec<EdgeKey>>;

    /// Batch persist multiple edges.
    ///
    /// Implementations should make this atomic if possible.
    async fn persist_batch(&self, operations: Vec<(EdgeKey, EntityId, u64, bool)>) -> Result<()> {
        // Default implementation: sequential writes
        for (key, dst, ts, is_delete) in operations {
            if is_delete {
                self.persist_delete(key, dst, ts).await?;
            } else {
                self.persist_edge(key, dst, ts).await?;
            }
        }
        Ok(())
    }

    /// Flush any buffered writes to durable storage.
    async fn flush(&self) -> Result<()> {
        Ok(()) // Default: no-op
    }

    /// Get storage statistics.
    async fn stats(&self) -> StorageStats {
        StorageStats::default()
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_keys: usize,
    pub total_edges: usize,
    pub total_tombstones: usize,
    pub bytes_used: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests are in the implementation modules
}

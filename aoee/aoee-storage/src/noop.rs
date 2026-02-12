//! No-op storage implementation for cache-only mode.
//!
//! All writes are acknowledged but not persisted.
//! Useful for testing or when persistence is handled externally.

use crate::traits::{EdgeStore, Result, StorageStats, StoredEdge};
use aoee_core::{EdgeKey, EntityId};
use async_trait::async_trait;

/// No-op storage that discards all writes.
///
/// Use this when:
/// - Running in cache-only mode
/// - Persistence is handled by an external system
/// - Testing cache behavior without storage overhead
pub struct NoopStore;

impl NoopStore {
    pub fn new() -> Self {
        NoopStore
    }
}

impl Default for NoopStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EdgeStore for NoopStore {
    async fn persist_edge(&self, _key: EdgeKey, _dst: EntityId, _timestamp: u64) -> Result<()> {
        Ok(())
    }

    async fn persist_delete(&self, _key: EdgeKey, _dst: EntityId, _timestamp: u64) -> Result<()> {
        Ok(())
    }

    async fn load_edges(&self, _key: EdgeKey) -> Result<Vec<StoredEdge>> {
        Ok(Vec::new())
    }

    async fn load_edges_paginated(
        &self,
        _key: EdgeKey,
        _cursor: Option<EntityId>,
        _limit: usize,
    ) -> Result<Vec<StoredEdge>> {
        Ok(Vec::new())
    }

    async fn edge_exists(&self, _key: EdgeKey, _dst: EntityId) -> Result<bool> {
        Ok(false)
    }

    async fn count_edges(&self, _key: EdgeKey) -> Result<usize> {
        Ok(0)
    }

    async fn scan_keys(&self, _prefix: Option<EntityId>) -> Result<Vec<EdgeKey>> {
        Ok(Vec::new())
    }

    async fn stats(&self) -> StorageStats {
        StorageStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aoee_core::{EntityType, EdgeType};

    fn make_id(raw: u64) -> EntityId {
        EntityId::new(EntityType::User, raw)
    }

    fn make_key(src: u64, edge_type: EdgeType) -> EdgeKey {
        EdgeKey::new(make_id(src), edge_type)
    }

    #[tokio::test]
    async fn test_noop_persist() {
        let store = NoopStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        // Should succeed but not store anything
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        
        let edges = store.load_edges(key).await.unwrap();
        assert!(edges.is_empty());
    }

    #[tokio::test]
    async fn test_noop_edge_exists() {
        let store = NoopStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        let exists = store.edge_exists(key, make_id(2)).await.unwrap();
        assert!(!exists);
    }
}

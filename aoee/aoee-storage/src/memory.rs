//! In-memory storage implementation for testing and development.

use crate::traits::{EdgeStore, Result, StorageError, StorageStats, StoredEdge};
use aoee_core::{EdgeKey, EntityId};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};

/// In-memory edge storage.
///
/// Uses a HashMap of BTreeMaps for efficient storage and retrieval.
/// The outer HashMap is keyed by EdgeKey, and the inner BTreeMap
/// is keyed by destination EntityId for sorted iteration.
///
/// Thread-safe via RwLock.
pub struct InMemoryStore {
    /// Storage: EdgeKey -> (dst -> (timestamp, deleted))
    data: RwLock<HashMap<EdgeKey, BTreeMap<EntityId, (u64, bool)>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        InMemoryStore {
            data: RwLock::new(HashMap::new()),
        }
    }

    /// Create with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        InMemoryStore {
            data: RwLock::new(HashMap::with_capacity(capacity)),
        }
    }

    /// Clear all data (for testing)
    pub fn clear(&self) {
        let mut data = self.data.write();
        data.clear();
    }

    /// Get the number of keys stored
    pub fn key_count(&self) -> usize {
        self.data.read().len()
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EdgeStore for InMemoryStore {
    async fn persist_edge(&self, key: EdgeKey, dst: EntityId, timestamp: u64) -> Result<()> {
        let mut data = self.data.write();
        let edges = data.entry(key).or_insert_with(BTreeMap::new);
        
        // Only update if this is newer
        match edges.get(&dst) {
            Some((existing_ts, _)) if *existing_ts >= timestamp => {
                // Existing entry is newer, skip
            }
            _ => {
                edges.insert(dst, (timestamp, false));
            }
        }
        
        Ok(())
    }

    async fn persist_delete(&self, key: EdgeKey, dst: EntityId, timestamp: u64) -> Result<()> {
        let mut data = self.data.write();
        let edges = data.entry(key).or_insert_with(BTreeMap::new);
        
        // Only update if this is newer
        match edges.get(&dst) {
            Some((existing_ts, _)) if *existing_ts >= timestamp => {
                // Existing entry is newer, skip
            }
            _ => {
                edges.insert(dst, (timestamp, true));
            }
        }
        
        Ok(())
    }

    async fn load_edges(&self, key: EdgeKey) -> Result<Vec<StoredEdge>> {
        let data = self.data.read();
        
        match data.get(&key) {
            Some(edges) => {
                let result: Vec<StoredEdge> = edges
                    .iter()
                    .filter(|(_, (_, deleted))| !deleted)
                    .map(|(&dst, &(timestamp, deleted))| StoredEdge { dst, timestamp, deleted })
                    .collect();
                Ok(result)
            }
            None => Ok(Vec::new()),
        }
    }

    async fn load_edges_paginated(
        &self,
        key: EdgeKey,
        cursor: Option<EntityId>,
        limit: usize,
    ) -> Result<Vec<StoredEdge>> {
        let data = self.data.read();
        
        match data.get(&key) {
            Some(edges) => {
                let iter: Box<dyn Iterator<Item = _>> = match cursor {
                    Some(c) => Box::new(edges.range((std::ops::Bound::Excluded(c), std::ops::Bound::Unbounded))),
                    None => Box::new(edges.iter()),
                };
                
                let result: Vec<StoredEdge> = iter
                    .filter(|(_, (_, deleted))| !deleted)
                    .take(limit)
                    .map(|(&dst, &(timestamp, deleted))| StoredEdge { dst, timestamp, deleted })
                    .collect();
                Ok(result)
            }
            None => Ok(Vec::new()),
        }
    }

    async fn edge_exists(&self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        let data = self.data.read();
        
        match data.get(&key) {
            Some(edges) => {
                match edges.get(&dst) {
                    Some((_, deleted)) => Ok(!deleted),
                    None => Ok(false),
                }
            }
            None => Ok(false),
        }
    }

    async fn count_edges(&self, key: EdgeKey) -> Result<usize> {
        let data = self.data.read();
        
        match data.get(&key) {
            Some(edges) => {
                let count = edges.values().filter(|(_, deleted)| !deleted).count();
                Ok(count)
            }
            None => Ok(0),
        }
    }

    async fn scan_keys(&self, prefix: Option<EntityId>) -> Result<Vec<EdgeKey>> {
        let data = self.data.read();
        
        let keys: Vec<EdgeKey> = match prefix {
            Some(p) => data.keys().filter(|k| k.src == p).copied().collect(),
            None => data.keys().copied().collect(),
        };
        
        Ok(keys)
    }

    async fn persist_batch(&self, operations: Vec<(EdgeKey, EntityId, u64, bool)>) -> Result<()> {
        let mut data = self.data.write();
        
        for (key, dst, timestamp, is_delete) in operations {
            let edges = data.entry(key).or_insert_with(BTreeMap::new);
            
            match edges.get(&dst) {
                Some((existing_ts, _)) if *existing_ts >= timestamp => {
                    // Existing entry is newer, skip
                }
                _ => {
                    edges.insert(dst, (timestamp, is_delete));
                }
            }
        }
        
        Ok(())
    }

    async fn stats(&self) -> StorageStats {
        let data = self.data.read();
        
        let mut total_edges = 0;
        let mut total_tombstones = 0;
        
        for edges in data.values() {
            for (_, deleted) in edges.values() {
                if *deleted {
                    total_tombstones += 1;
                } else {
                    total_edges += 1;
                }
            }
        }
        
        StorageStats {
            total_keys: data.len(),
            total_edges,
            total_tombstones,
            bytes_used: 0, // Could calculate but not critical for in-memory
        }
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
    async fn test_persist_and_load() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        store.persist_edge(key, make_id(3), 101).await.unwrap();
        store.persist_edge(key, make_id(4), 102).await.unwrap();
        
        let edges = store.load_edges(key).await.unwrap();
        
        assert_eq!(edges.len(), 3);
        assert_eq!(edges[0].dst, make_id(2));
        assert_eq!(edges[1].dst, make_id(3));
        assert_eq!(edges[2].dst, make_id(4));
    }

    #[tokio::test]
    async fn test_persist_delete() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        store.persist_edge(key, make_id(3), 101).await.unwrap();
        store.persist_delete(key, make_id(2), 102).await.unwrap();
        
        let edges = store.load_edges(key).await.unwrap();
        
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].dst, make_id(3));
    }

    #[tokio::test]
    async fn test_timestamp_ordering() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        // Delete then add with older timestamp - delete should win
        store.persist_delete(key, make_id(2), 200).await.unwrap();
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        
        let exists = store.edge_exists(key, make_id(2)).await.unwrap();
        assert!(!exists);
        
        // Add with newer timestamp - add should win
        store.persist_edge(key, make_id(2), 300).await.unwrap();
        
        let exists = store.edge_exists(key, make_id(2)).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_pagination() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        for i in 1..=10 {
            store.persist_edge(key, make_id(i), 100).await.unwrap();
        }
        
        // First page
        let page1 = store.load_edges_paginated(key, None, 3).await.unwrap();
        assert_eq!(page1.len(), 3);
        assert_eq!(page1[0].dst, make_id(1));
        assert_eq!(page1[2].dst, make_id(3));
        
        // Second page
        let page2 = store.load_edges_paginated(key, Some(make_id(3)), 3).await.unwrap();
        assert_eq!(page2.len(), 3);
        assert_eq!(page2[0].dst, make_id(4));
    }

    #[tokio::test]
    async fn test_count_edges() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        for i in 1..=5 {
            store.persist_edge(key, make_id(i), 100).await.unwrap();
        }
        store.persist_delete(key, make_id(3), 101).await.unwrap();
        
        let count = store.count_edges(key).await.unwrap();
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn test_scan_keys() {
        let store = InMemoryStore::new();
        
        let key1 = make_key(1, EdgeType::Follows);
        let key2 = make_key(1, EdgeType::Likes);
        let key3 = make_key(2, EdgeType::Follows);
        
        store.persist_edge(key1, make_id(10), 100).await.unwrap();
        store.persist_edge(key2, make_id(20), 100).await.unwrap();
        store.persist_edge(key3, make_id(30), 100).await.unwrap();
        
        // Scan all
        let all_keys = store.scan_keys(None).await.unwrap();
        assert_eq!(all_keys.len(), 3);
        
        // Scan with prefix
        let user1_keys = store.scan_keys(Some(make_id(1))).await.unwrap();
        assert_eq!(user1_keys.len(), 2);
    }

    #[tokio::test]
    async fn test_batch_persist() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        let ops = vec![
            (key, make_id(2), 100, false),
            (key, make_id(3), 101, false),
            (key, make_id(4), 102, false),
            (key, make_id(3), 103, true), // Delete
        ];
        
        store.persist_batch(ops).await.unwrap();
        
        let edges = store.load_edges(key).await.unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let store = InMemoryStore::new();
        let key = make_key(1, EdgeType::Follows);
        
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        store.persist_edge(key, make_id(3), 101).await.unwrap();
        store.persist_delete(key, make_id(3), 102).await.unwrap();
        
        let stats = store.stats().await;
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.total_edges, 1);
        assert_eq!(stats.total_tombstones, 1);
    }
}

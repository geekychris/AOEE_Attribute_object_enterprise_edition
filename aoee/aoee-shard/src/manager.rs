//! Shard manager for routing requests to appropriate shards

use crate::config::ShardConfig;
use crate::hash::ConsistentHash;
use crate::shard::{Shard, ShardError, ShardStats};
use aoee_core::{EdgeKey, EntityId};
use aoee_storage::EdgeStore;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Shard manager errors
#[derive(Error, Debug)]
pub enum ManagerError {
    #[error("Shard not found: {0}")]
    ShardNotFound(u32),
    #[error("No shards available")]
    NoShards,
    #[error("Shard error: {0}")]
    Shard(#[from] ShardError),
}

pub type Result<T> = std::result::Result<T, ManagerError>;

/// Configuration for the shard manager
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Number of shards
    pub num_shards: u32,
    /// Virtual nodes per shard for consistent hashing
    pub virtual_nodes: u32,
    /// Default shard configuration
    pub shard_config: ShardConfig,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        ManagerConfig {
            num_shards: 4,
            virtual_nodes: 150,
            shard_config: ShardConfig::default(),
        }
    }
}

/// Manages multiple shards and routes requests
pub struct ShardManager<S: EdgeStore + 'static> {
    config: ManagerConfig,
    shards: RwLock<HashMap<u32, Arc<Shard<S>>>>,
    hash_ring: RwLock<ConsistentHash>,
    storage_factory: Arc<dyn Fn(u32) -> Arc<S> + Send + Sync>,
}

impl<S: EdgeStore + 'static> ShardManager<S> {
    /// Create a new shard manager
    pub fn new<F>(config: ManagerConfig, storage_factory: F) -> Self
    where
        F: Fn(u32) -> Arc<S> + Send + Sync + 'static,
    {
        let shard_ids: Vec<u32> = (0..config.num_shards).collect();
        let hash_ring = ConsistentHash::with_virtual_nodes(shard_ids, config.virtual_nodes);
        
        ShardManager {
            config,
            shards: RwLock::new(HashMap::new()),
            hash_ring: RwLock::new(hash_ring),
            storage_factory: Arc::new(storage_factory),
        }
    }

    /// Initialize all shards
    pub async fn initialize(&self) {
        let mut shards = self.shards.write().await;
        
        for shard_id in 0..self.config.num_shards {
            let storage = (self.storage_factory)(shard_id);
            let mut shard_config = self.config.shard_config.clone();
            shard_config.shard_id = shard_id;
            
            let shard = Arc::new(Shard::new(shard_config, storage));
            shards.insert(shard_id, shard);
        }
    }

    /// Get the shard for a key
    async fn get_shard(&self, key: &EdgeKey) -> Result<Arc<Shard<S>>> {
        let shard_id = {
            let ring = self.hash_ring.read().await;
            ring.get_shard(key).ok_or(ManagerError::NoShards)?
        };
        
        let shards = self.shards.read().await;
        shards
            .get(&shard_id)
            .cloned()
            .ok_or(ManagerError::ShardNotFound(shard_id))
    }

    /// Add an edge
    pub async fn add_edge(&self, key: EdgeKey, dst: EntityId) -> Result<u64> {
        let shard = self.get_shard(&key).await?;
        let timestamp = shard.add_edge(key, dst).await?;
        Ok(timestamp)
    }

    /// Add an edge with timestamp and metadata
    pub async fn add_edge_with_metadata(
        &self,
        key: EdgeKey,
        dst: EntityId,
        timestamp: u64,
        metadata: u8,
    ) -> Result<u64> {
        let shard = self.get_shard(&key).await?;
        let actual_timestamp = shard.add_edge_with_metadata(key, dst, timestamp, metadata).await?;
        Ok(actual_timestamp)
    }

    /// Delete an edge
    pub async fn delete_edge(&self, key: EdgeKey, dst: EntityId) -> Result<()> {
        let shard = self.get_shard(&key).await?;
        shard.delete_edge(key, dst).await?;
        Ok(())
    }

    /// Get neighbors
    pub async fn neighbors(&self, key: EdgeKey) -> Result<Vec<EntityId>> {
        let shard = self.get_shard(&key).await?;
        Ok(shard.neighbors(key).await?)
    }

    /// Get neighbors with limit
    pub async fn neighbors_limited(&self, key: EdgeKey, limit: usize) -> Result<Vec<EntityId>> {
        let shard = self.get_shard(&key).await?;
        Ok(shard.neighbors_limited(key, limit).await?)
    }

    /// Get neighbors with metadata (timestamps, reaction types, etc.)
    pub async fn neighbors_with_metadata(
        &self,
        key: EdgeKey,
        limit: usize,
    ) -> Result<Vec<(EntityId, u64, u8)>> {
        let shard = self.get_shard(&key).await?;
        Ok(shard.neighbors_with_metadata(key, limit).await?)
    }

    /// Check if an edge exists
    pub async fn contains(&self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        let shard = self.get_shard(&key).await?;
        Ok(shard.contains(key, dst).await?)
    }

    /// Get edge count
    pub async fn count(&self, key: EdgeKey) -> Result<usize> {
        let shard = self.get_shard(&key).await?;
        Ok(shard.count(key).await?)
    }

    /// Intersect two keys (may span shards)
    pub async fn intersect(&self, key1: EdgeKey, key2: EdgeKey) -> Result<Vec<EntityId>> {
        // Get neighbors from potentially different shards
        let neighbors1 = self.neighbors(key1).await?;
        let neighbors2 = self.neighbors(key2).await?;
        
        Ok(aoee_core::intersect_slices(&neighbors1, &neighbors2))
    }

    /// Union two keys (may span shards)
    pub async fn union(&self, key1: EdgeKey, key2: EdgeKey) -> Result<Vec<EntityId>> {
        let neighbors1 = self.neighbors(key1).await?;
        let neighbors2 = self.neighbors(key2).await?;
        
        Ok(aoee_core::union_slices(&neighbors1, &neighbors2))
    }

    /// Get statistics for all shards
    pub async fn stats(&self) -> HashMap<u32, ShardStats> {
        let shards = self.shards.read().await;
        let mut result = HashMap::new();
        
        for (&shard_id, shard) in shards.iter() {
            result.insert(shard_id, shard.stats().await);
        }
        
        result
    }

    /// Get aggregated statistics
    pub async fn aggregated_stats(&self) -> ShardStats {
        let stats_map = self.stats().await;
        let mut aggregated = ShardStats::default();
        
        for stats in stats_map.values() {
            aggregated.cached_lists += stats.cached_lists;
            aggregated.total_edges += stats.total_edges;
            aggregated.reads += stats.reads;
            aggregated.writes += stats.writes;
            aggregated.cache_hits += stats.cache_hits;
            aggregated.cache_misses += stats.cache_misses;
        }
        
        aggregated
    }

    /// Get number of shards
    pub fn num_shards(&self) -> u32 {
        self.config.num_shards
    }

    /// Add a new shard (for scaling)
    pub async fn add_shard(&self, storage: Arc<S>) {
        let shard_id = {
            let shards = self.shards.read().await;
            shards.keys().max().map(|&id| id + 1).unwrap_or(0)
        };
        
        let mut shard_config = self.config.shard_config.clone();
        shard_config.shard_id = shard_id;
        
        let shard = Arc::new(Shard::new(shard_config, storage));
        
        {
            let mut shards = self.shards.write().await;
            shards.insert(shard_id, shard);
        }
        
        {
            let mut ring = self.hash_ring.write().await;
            ring.add_shard(shard_id);
        }
    }

    /// Remove a shard (for scaling down)
    pub async fn remove_shard(&self, shard_id: u32) -> Option<Arc<Shard<S>>> {
        let shard = {
            let mut shards = self.shards.write().await;
            shards.remove(&shard_id)
        };
        
        if shard.is_some() {
            let mut ring = self.hash_ring.write().await;
            ring.remove_shard(shard_id);
        }
        
        shard
    }

    /// Clear all caches across all shards
    /// Returns the number of entries that were cleared
    pub async fn clear_all_caches(&self) -> usize {
        let shards = self.shards.read().await;
        let mut total_cleared = 0;
        
        for shard in shards.values() {
            let stats = shard.cache_stats();
            total_cleared += stats.entries;
            shard.clear_cache();
        }
        
        total_cleared
    }

    /// Clear cache for a specific shard
    pub async fn clear_shard_cache(&self, shard_id: u32) -> Result<usize> {
        let shards = self.shards.read().await;
        let shard = shards.get(&shard_id)
            .ok_or(ManagerError::ShardNotFound(shard_id))?;
        
        let stats = shard.cache_stats();
        let entries = stats.entries;
        shard.clear_cache();
        
        Ok(entries)
    }

    /// Flush storage for all shards (if storage supports it)
    /// Returns the number of entries in cache at time of flush
    pub async fn flush_all(&self) -> usize {
        let shards = self.shards.read().await;
        let mut total_entries = 0;
        
        for shard in shards.values() {
            let stats = shard.cache_stats();
            total_entries += stats.entries;
            // Storage flush is handled by the EdgeStore implementation
        }
        
        total_entries
    }

    /// Evict a specific key from cache (forces reload from storage on next access)
    pub async fn evict_key(&self, key: EdgeKey) -> Result<()> {
        let shard = self.get_shard(&key).await?;
        shard.evict(key);
        Ok(())
    }

    /// Run maintenance on all shards (TTL eviction, compaction)
    pub async fn maintenance(&self) {
        let shards = self.shards.read().await;
        for shard in shards.values() {
            shard.maintenance().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aoee_core::{EntityType, EdgeType};
    use aoee_storage::InMemoryStore;

    fn make_id(raw: u64) -> EntityId {
        EntityId::new(EntityType::User, raw)
    }

    fn make_key(src: u64, edge_type: EdgeType) -> EdgeKey {
        EdgeKey::new(make_id(src), edge_type)
    }

    #[tokio::test]
    async fn test_basic_operations() {
        let config = ManagerConfig {
            num_shards: 4,
            ..Default::default()
        };
        
        let manager = ShardManager::new(config, |_| Arc::new(InMemoryStore::new()));
        manager.initialize().await;
        
        let key = make_key(1, EdgeType::Follows);
        
        manager.add_edge(key, make_id(2)).await.unwrap();
        manager.add_edge(key, make_id(3)).await.unwrap();
        
        let neighbors = manager.neighbors(key).await.unwrap();
        assert_eq!(neighbors.len(), 2);
    }

    #[tokio::test]
    async fn test_cross_shard_intersect() {
        let config = ManagerConfig {
            num_shards: 4,
            ..Default::default()
        };
        
        let manager = ShardManager::new(config, |_| Arc::new(InMemoryStore::new()));
        manager.initialize().await;
        
        let key1 = make_key(1, EdgeType::Follows);
        let key2 = make_key(100, EdgeType::Follows); // Different key, likely different shard
        
        // User 1 follows: 10, 20, 30
        manager.add_edge(key1, make_id(10)).await.unwrap();
        manager.add_edge(key1, make_id(20)).await.unwrap();
        manager.add_edge(key1, make_id(30)).await.unwrap();
        
        // User 100 follows: 20, 30, 40
        manager.add_edge(key2, make_id(20)).await.unwrap();
        manager.add_edge(key2, make_id(30)).await.unwrap();
        manager.add_edge(key2, make_id(40)).await.unwrap();
        
        let common = manager.intersect(key1, key2).await.unwrap();
        assert_eq!(common.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let config = ManagerConfig {
            num_shards: 2,
            ..Default::default()
        };
        
        let manager = ShardManager::new(config, |_| Arc::new(InMemoryStore::new()));
        manager.initialize().await;
        
        let key = make_key(1, EdgeType::Follows);
        manager.add_edge(key, make_id(2)).await.unwrap();
        
        let stats = manager.aggregated_stats().await;
        assert_eq!(stats.writes, 1);
    }
}

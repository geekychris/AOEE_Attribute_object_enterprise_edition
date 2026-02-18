//! Individual shard implementation

use crate::cache::{LruCache, CacheStats as LruCacheStats};
use crate::config::ShardConfig;
use aoee_core::{
    compaction::CompactionConfig,
    types::{BufferEntry, PostingList, SharedPostingList, new_shared_posting_list},
    EdgeKey, EntityId,
};
use aoee_storage::{EdgeStore, StorageError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Shard errors
#[derive(Error, Debug)]
pub enum ShardError {
    #[error("Key not found: {0:?}")]
    NotFound(EdgeKey),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ShardError>;

/// A shard manages posting lists for a subset of keys
pub struct Shard<S: EdgeStore> {
    config: ShardConfig,
    storage: Arc<S>,
    /// LRU cache for posting lists
    cache: LruCache,
    /// Stats
    stats: RwLock<ShardStats>,
}

#[derive(Debug, Default, Clone)]
pub struct ShardStats {
    pub cached_lists: usize,
    pub total_edges: usize,
    pub reads: u64,
    pub writes: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub cache_memory_bytes: usize,
}

impl<S: EdgeStore> Shard<S> {
    pub fn new(config: ShardConfig, storage: Arc<S>) -> Self {
        let cache = LruCache::new(config.cache_config());
        Shard {
            config,
            storage,
            cache,
            stats: RwLock::new(ShardStats::default()),
        }
    }

    /// Get current timestamp (nanoseconds for ordering precision)
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    /// Add an edge
    pub async fn add_edge(&self, key: EdgeKey, dst: EntityId) -> Result<u64> {
        self.add_edge_with_metadata(key, dst, 0, 0).await
    }

    /// Add an edge with optional timestamp and metadata
    /// If timestamp is 0, auto-generate. Returns the actual timestamp used.
    pub async fn add_edge_with_metadata(
        &self,
        key: EdgeKey,
        dst: EntityId,
        timestamp: u64,
        metadata: u8,
    ) -> Result<u64> {
        let timestamp = if timestamp == 0 { Self::now() } else { timestamp };
        
        // Write-through to storage if enabled
        if self.config.write_through {
            self.storage.persist_edge_with_metadata(key, dst, timestamp, metadata).await?;
        }
        
        // Update cache
        let pl = self.get_or_create_list(key);
        {
            let mut list = pl.write();
            list.add_with_metadata(dst, timestamp, metadata);
            
            // Check if compaction is needed
            if list.buffer_len() > self.config.compaction.buffer_threshold {
                list.compact(&self.config.compaction);
            }
        }
        
        // Update size estimate after modification
        self.cache.update_size(&key);
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
        }
        
        Ok(timestamp)
    }

    /// Delete an edge
    pub async fn delete_edge(&self, key: EdgeKey, dst: EntityId) -> Result<()> {
        let timestamp = Self::now();
        
        // Write-through to storage if enabled
        if self.config.write_through {
            self.storage.persist_delete(key, dst, timestamp).await?;
        }
        
        // Update cache if key exists
        if let Some(pl) = self.cache.get(&key) {
            let mut list = pl.write();
            list.delete(dst, timestamp);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
        }
        
        Ok(())
    }

    /// Get neighbors for a key
    pub async fn neighbors(&self, key: EdgeKey) -> Result<Vec<EntityId>> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.reads += 1;
        }
        
        // Check cache (LruCache.get() updates access time automatically)
        if let Some(pl) = self.cache.get(&key) {
            let list = pl.read();
            return Ok(list.neighbors());
        }
        
        // Cache miss - load from storage
        let pl = self.load_from_storage(key).await?;
        let neighbors = {
            let list = pl.read();
            list.neighbors()
        };
        
        Ok(neighbors)
    }

    /// Get neighbors with limit
    pub async fn neighbors_limited(&self, key: EdgeKey, limit: usize) -> Result<Vec<EntityId>> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.reads += 1;
        }
        
        // Check cache
        if let Some(pl) = self.cache.get(&key) {
            let list = pl.read();
            return Ok(list.neighbors_limited(limit));
        }
        
        // Cache miss - load from storage
        let pl = self.load_from_storage(key).await?;
        let neighbors = {
            let list = pl.read();
            list.neighbors_limited(limit)
        };
        
        Ok(neighbors)
    }

    /// Get neighbors with metadata (timestamp, reaction type, etc.)
    /// Returns Vec<(entity_id, timestamp_nanos, metadata_byte)>
    pub async fn neighbors_with_metadata(&self, key: EdgeKey, limit: usize) -> Result<Vec<(EntityId, u64, u8)>> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.reads += 1;
        }
        
        // Check cache
        if let Some(pl) = self.cache.get(&key) {
            let list = pl.read();
            return Ok(if limit > 0 {
                list.neighbors_with_metadata_limited(limit)
            } else {
                list.neighbors_with_metadata()
            });
        }
        
        // Cache miss - load from storage
        let pl = self.load_from_storage(key).await?;
        let result = {
            let list = pl.read();
            if limit > 0 {
                list.neighbors_with_metadata_limited(limit)
            } else {
                list.neighbors_with_metadata()
            }
        };
        
        Ok(result)
    }

    /// Check if an edge exists
    pub async fn contains(&self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        // Check cache
        if let Some(pl) = self.cache.get(&key) {
            let list = pl.read();
            return Ok(list.contains(dst));
        }
        
        // Cache miss - load from storage
        let pl = self.load_from_storage(key).await?;
        let contains = {
            let list = pl.read();
            list.contains(dst)
        };
        
        Ok(contains)
    }

    /// Get edge count
    pub async fn count(&self, key: EdgeKey) -> Result<usize> {
        // Check cache
        if let Some(pl) = self.cache.get(&key) {
            let list = pl.read();
            return Ok(list.count());
        }
        
        // Use storage count
        let count = self.storage.count_edges(key).await?;
        Ok(count)
    }

    /// Get or create a posting list in cache
    fn get_or_create_list(&self, key: EdgeKey) -> SharedPostingList {
        self.cache.get_or_insert_with(key, new_shared_posting_list)
    }

    /// Load posting list from storage
    async fn load_from_storage(&self, key: EdgeKey) -> Result<SharedPostingList> {
        let stored_edges = self.storage.load_edges(key).await?;
        
        let pl = new_shared_posting_list();
        {
            let mut list = pl.write();
            for edge in stored_edges {
                if !edge.deleted {
                    list.add_with_metadata(edge.dst, edge.timestamp, edge.metadata);
                }
            }
            // Note: We don't force compact here because segments don't preserve metadata.
            // Regular compaction will happen when buffer exceeds threshold.
            // For edge types with metadata (like LIKES), this keeps metadata accessible.
        }
        
        // Add to LRU cache
        self.cache.insert(key, pl.clone());
        
        // Update stats for cache miss
        {
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
        }
        
        Ok(pl)
    }

    /// Intersect two keys
    pub async fn intersect(&self, key1: EdgeKey, key2: EdgeKey) -> Result<Vec<EntityId>> {
        let neighbors1 = self.neighbors(key1).await?;
        let neighbors2 = self.neighbors(key2).await?;
        
        // Use set operations from core
        Ok(aoee_core::intersect_slices(&neighbors1, &neighbors2))
    }

    /// Union two keys
    pub async fn union(&self, key1: EdgeKey, key2: EdgeKey) -> Result<Vec<EntityId>> {
        let neighbors1 = self.neighbors(key1).await?;
        let neighbors2 = self.neighbors(key2).await?;
        
        Ok(aoee_core::union_slices(&neighbors1, &neighbors2))
    }

    /// Get shard statistics
    pub async fn stats(&self) -> ShardStats {
        let mut stats = self.stats.read().await.clone();
        let cache_stats = self.cache.stats();
        stats.cached_lists = cache_stats.entries;
        stats.cache_hits = cache_stats.hits;
        stats.cache_misses = cache_stats.misses;
        stats.cache_evictions = cache_stats.evictions;
        stats.cache_memory_bytes = cache_stats.memory_bytes;
        stats
    }

    /// Get detailed cache statistics
    pub fn cache_stats(&self) -> LruCacheStats {
        self.cache.stats()
    }

    /// Get shard ID
    pub fn shard_id(&self) -> u32 {
        self.config.shard_id
    }

    /// Clear cache (for testing)
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Evict a specific key from cache
    pub fn evict(&self, key: EdgeKey) {
        self.cache.remove(&key);
    }

    /// Check if a key is in the cache
    pub fn cache_contains(&self, key: &EdgeKey) -> bool {
        self.cache.contains(key)
    }

    /// Evict entries older than the given duration
    pub fn evict_older_than(&self, max_age: Duration) {
        self.cache.evict_older_than(max_age);
    }

    /// Evict to reach target memory usage
    pub fn evict_to_memory(&self, target_bytes: usize) {
        self.cache.evict_to_memory_limit(target_bytes);
    }

    /// Force LRU eviction
    pub fn evict_lru(&self) {
        self.cache.evict_lru();
    }

    /// Force compaction on all cached lists
    pub async fn compact_all(&self) {
        for key in self.cache.keys() {
            if let Some(pl) = self.cache.get(&key) {
                let mut list = pl.write();
                list.compact(&self.config.compaction);
            }
        }
    }

    /// Run periodic maintenance (TTL eviction, compaction)
    pub async fn maintenance(&self) {
        // TTL-based eviction if configured
        if self.config.cache_ttl_seconds > 0 {
            let ttl = Duration::from_secs(self.config.cache_ttl_seconds);
            self.evict_older_than(ttl);
        }
        
        // Compact if needed
        if self.config.background_compaction {
            self.compact_all().await;
        }
        
        let stats = self.cache.stats();
        debug!(
            "Shard {} maintenance: {} entries, {} bytes, {} evictions",
            self.config.shard_id, stats.entries, stats.memory_bytes, stats.evictions
        );
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
    async fn test_add_and_get() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key = make_key(1, EdgeType::Follows);
        
        shard.add_edge(key, make_id(2)).await.unwrap();
        shard.add_edge(key, make_id(3)).await.unwrap();
        shard.add_edge(key, make_id(4)).await.unwrap();
        
        let neighbors = shard.neighbors(key).await.unwrap();
        assert_eq!(neighbors.len(), 3);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key = make_key(1, EdgeType::Follows);
        
        shard.add_edge(key, make_id(2)).await.unwrap();
        shard.add_edge(key, make_id(3)).await.unwrap();
        shard.delete_edge(key, make_id(2)).await.unwrap();
        
        let neighbors = shard.neighbors(key).await.unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], make_id(3));
    }

    #[tokio::test]
    async fn test_contains() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key = make_key(1, EdgeType::Follows);
        
        shard.add_edge(key, make_id(2)).await.unwrap();
        
        assert!(shard.contains(key, make_id(2)).await.unwrap());
        assert!(!shard.contains(key, make_id(3)).await.unwrap());
    }

    #[tokio::test]
    async fn test_intersect() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key1 = make_key(1, EdgeType::Follows);
        let key2 = make_key(2, EdgeType::Follows);
        
        // User 1 follows: 10, 20, 30
        shard.add_edge(key1, make_id(10)).await.unwrap();
        shard.add_edge(key1, make_id(20)).await.unwrap();
        shard.add_edge(key1, make_id(30)).await.unwrap();
        
        // User 2 follows: 20, 30, 40
        shard.add_edge(key2, make_id(20)).await.unwrap();
        shard.add_edge(key2, make_id(30)).await.unwrap();
        shard.add_edge(key2, make_id(40)).await.unwrap();
        
        let common = shard.intersect(key1, key2).await.unwrap();
        assert_eq!(common.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key = make_key(1, EdgeType::Follows);
        
        shard.add_edge(key, make_id(2)).await.unwrap();
        
        // First read
        let _ = shard.neighbors(key).await.unwrap();
        // Second read (should be cache hit)
        let _ = shard.neighbors(key).await.unwrap();
        
        let stats = shard.stats().await;
        assert!(stats.cache_hits >= 1);
    }

    #[tokio::test]
    async fn test_evict() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key = make_key(1, EdgeType::Follows);
        
        shard.add_edge(key, make_id(2)).await.unwrap();
        assert!(shard.cache_contains(&key));
        
        shard.evict(key);
        assert!(!shard.cache_contains(&key));
        
        // Should still be able to load from storage
        let neighbors = shard.neighbors(key).await.unwrap();
        assert_eq!(neighbors.len(), 1);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let store = Arc::new(InMemoryStore::new());
        // Create config with small cache and allow eviction with few entries
        let mut config = ShardConfig::default();
        config.cache.max_entries = 3;
        config.cache.min_entries = 0; // Allow eviction with few entries
        config.cache.eviction_target_ratio = 0.66;
        let shard = Shard::new(config, store);
        
        // Add 5 keys, exceeding the 3-entry limit
        for i in 0..5 {
            let key = make_key(i, EdgeType::Follows);
            shard.add_edge(key, make_id(100 + i)).await.unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        
        // Cache should have evicted some entries
        let stats = shard.cache_stats();
        assert!(stats.evictions > 0, "Expected evictions, got {}", stats.evictions);
        assert!(stats.entries <= 3, "Expected at most 3 entries, got {}", stats.entries);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        
        let key = make_key(1, EdgeType::Follows);
        shard.add_edge(key, make_id(2)).await.unwrap();
        
        // Cache hit
        let _ = shard.neighbors(key).await.unwrap();
        
        // Cache miss (new key not in cache)
        let key2 = make_key(999, EdgeType::Follows);
        let _ = shard.neighbors(key2).await.unwrap();
        
        let stats = shard.stats().await;
        assert!(stats.cache_hits >= 1);
        assert!(stats.cache_memory_bytes > 0);
    }
}

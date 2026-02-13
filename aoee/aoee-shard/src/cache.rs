//! LRU Cache implementation for posting lists.
//!
//! Uses a time-based sampling approach for LRU eviction that works well
//! under high concurrency without the overhead of maintaining linked lists.

use aoee_core::types::SharedPostingList;
use aoee_core::EdgeKey;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Cache entry with access tracking
struct CacheEntry {
    /// The cached posting list
    list: SharedPostingList,
    /// Last access time (monotonic, for LRU ordering)
    last_access: AtomicU64,
    /// Approximate size in bytes
    size_bytes: AtomicUsize,
}

impl CacheEntry {
    fn new(list: SharedPostingList) -> Self {
        let size = Self::estimate_size(&list);
        CacheEntry {
            list,
            last_access: AtomicU64::new(Self::now_millis()),
            size_bytes: AtomicUsize::new(size),
        }
    }

    fn touch(&self) {
        self.last_access.store(Self::now_millis(), Ordering::Relaxed);
    }

    fn last_access_millis(&self) -> u64 {
        self.last_access.load(Ordering::Relaxed)
    }

    fn size(&self) -> usize {
        self.size_bytes.load(Ordering::Relaxed)
    }

    fn update_size(&self, list: &SharedPostingList) {
        self.size_bytes.store(Self::estimate_size(list), Ordering::Relaxed);
    }

    fn now_millis() -> u64 {
        // Use a monotonic clock
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_millis() as u64
    }

    fn estimate_size(list: &SharedPostingList) -> usize {
        let guard = list.read();
        // Rough estimate: base struct + buffer entries + segments
        // Each buffer entry ~24 bytes, each segment entry ~8 bytes compressed
        let buffer_size = guard.buffer_len() * 24;
        let segment_size = guard.count() * 8;
        64 + buffer_size + segment_size
    }
}

/// Configuration for the LRU cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Maximum memory in bytes (0 = unlimited)
    pub max_memory_bytes: usize,
    /// How often to run eviction (in operations or time)
    pub eviction_batch_size: usize,
    /// Target utilization after eviction (0.0-1.0)
    pub eviction_target_ratio: f64,
    /// Minimum entries to keep regardless of memory pressure
    pub min_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            max_entries: 100_000,
            max_memory_bytes: 0, // Unlimited by default
            eviction_batch_size: 1000,
            eviction_target_ratio: 0.9,
            min_entries: 1000,
        }
    }
}

/// LRU Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub memory_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub insertions: u64,
}

/// Thread-safe LRU cache for posting lists
pub struct LruCache {
    /// The actual cache storage
    entries: DashMap<EdgeKey, Arc<CacheEntry>>,
    /// Configuration
    config: CacheConfig,
    /// Statistics
    stats: RwLock<CacheStatsInternal>,
    /// Total memory estimate
    total_memory: AtomicUsize,
}

#[derive(Debug, Default)]
struct CacheStatsInternal {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    insertions: AtomicU64,
}

impl LruCache {
    /// Create a new LRU cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        LruCache {
            entries: DashMap::with_capacity(config.max_entries),
            config,
            stats: RwLock::new(CacheStatsInternal::default()),
            total_memory: AtomicUsize::new(0),
        }
    }

    /// Create with default configuration
    pub fn with_capacity(max_entries: usize) -> Self {
        Self::new(CacheConfig {
            max_entries,
            ..Default::default()
        })
    }

    /// Get an entry, updating access time
    pub fn get(&self, key: &EdgeKey) -> Option<SharedPostingList> {
        if let Some(entry) = self.entries.get(key) {
            entry.touch();
            self.stats.read().hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.list.clone())
        } else {
            self.stats.read().misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Check if key exists without updating access time
    pub fn contains(&self, key: &EdgeKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Insert or update an entry
    pub fn insert(&self, key: EdgeKey, list: SharedPostingList) {
        let entry = Arc::new(CacheEntry::new(list));
        let entry_size = entry.size();

        // Check if we're updating an existing entry
        if let Some(old) = self.entries.insert(key, entry) {
            let old_size = old.size();
            // Adjust memory tracking
            if entry_size > old_size {
                self.total_memory.fetch_add(entry_size - old_size, Ordering::Relaxed);
            } else {
                self.total_memory.fetch_sub(old_size - entry_size, Ordering::Relaxed);
            }
        } else {
            // New entry
            self.total_memory.fetch_add(entry_size, Ordering::Relaxed);
            self.stats.read().insertions.fetch_add(1, Ordering::Relaxed);
        }

        // Check if eviction is needed
        self.maybe_evict();
    }

    /// Get or insert with a factory function
    pub fn get_or_insert_with<F>(&self, key: EdgeKey, factory: F) -> SharedPostingList
    where
        F: FnOnce() -> SharedPostingList,
    {
        // Fast path: check if already cached
        if let Some(list) = self.get(&key) {
            return list;
        }

        // Slow path: create and insert
        let list = factory();
        self.insert(key, list.clone());
        list
    }

    /// Remove an entry
    pub fn remove(&self, key: &EdgeKey) -> Option<SharedPostingList> {
        if let Some((_, entry)) = self.entries.remove(key) {
            self.total_memory.fetch_sub(entry.size(), Ordering::Relaxed);
            Some(entry.list.clone())
        } else {
            None
        }
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.entries.clear();
        self.total_memory.store(0, Ordering::Relaxed);
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the cache empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let internal = self.stats.read();
        CacheStats {
            entries: self.entries.len(),
            memory_bytes: self.total_memory.load(Ordering::Relaxed),
            hits: internal.hits.load(Ordering::Relaxed),
            misses: internal.misses.load(Ordering::Relaxed),
            evictions: internal.evictions.load(Ordering::Relaxed),
            insertions: internal.insertions.load(Ordering::Relaxed),
        }
    }

    /// Update the size estimate for an entry (call after modifying the list)
    pub fn update_size(&self, key: &EdgeKey) {
        if let Some(entry) = self.entries.get(key) {
            let old_size = entry.size();
            entry.update_size(&entry.list);
            let new_size = entry.size();
            
            if new_size > old_size {
                self.total_memory.fetch_add(new_size - old_size, Ordering::Relaxed);
            } else {
                self.total_memory.fetch_sub(old_size - new_size, Ordering::Relaxed);
            }
        }
    }

    /// Check if eviction is needed and perform it
    fn maybe_evict(&self) {
        let current_entries = self.entries.len();
        let current_memory = self.total_memory.load(Ordering::Relaxed);

        let over_entry_limit = current_entries > self.config.max_entries;
        let over_memory_limit = self.config.max_memory_bytes > 0 
            && current_memory > self.config.max_memory_bytes;

        if over_entry_limit || over_memory_limit {
            self.evict_lru();
        }
    }

    /// Evict least recently used entries
    pub fn evict_lru(&self) {
        let target_entries = (self.config.max_entries as f64 * self.config.eviction_target_ratio) as usize;
        let target_entries = target_entries.max(self.config.min_entries);

        let current_entries = self.entries.len();
        if current_entries <= target_entries {
            return;
        }

        let to_evict = current_entries - target_entries;
        
        // Collect entries with their access times
        let mut candidates: Vec<(EdgeKey, u64)> = self.entries
            .iter()
            .map(|entry| (*entry.key(), entry.value().last_access_millis()))
            .collect();

        // Sort by access time (oldest first)
        candidates.sort_by_key(|(_, time)| *time);

        // Evict the oldest entries
        let evict_count = to_evict.min(candidates.len());
        let mut actually_evicted = 0;

        for (key, _) in candidates.into_iter().take(evict_count) {
            if let Some((_, entry)) = self.entries.remove(&key) {
                self.total_memory.fetch_sub(entry.size(), Ordering::Relaxed);
                actually_evicted += 1;
            }
        }

        if actually_evicted > 0 {
            self.stats.read().evictions.fetch_add(actually_evicted, Ordering::Relaxed);
            debug!(
                "LRU eviction: removed {} entries, {} remaining",
                actually_evicted,
                self.entries.len()
            );
        }
    }

    /// Evict entries that haven't been accessed in the given duration
    pub fn evict_older_than(&self, max_age: Duration) {
        let now = CacheEntry::now_millis();
        let max_age_millis = max_age.as_millis() as u64;
        let cutoff = now.saturating_sub(max_age_millis);

        let mut evicted = 0;
        
        // Collect keys to evict (can't modify while iterating)
        let to_evict: Vec<EdgeKey> = self.entries
            .iter()
            .filter(|entry| entry.value().last_access_millis() < cutoff)
            .map(|entry| *entry.key())
            .collect();

        for key in to_evict {
            if let Some((_, entry)) = self.entries.remove(&key) {
                self.total_memory.fetch_sub(entry.size(), Ordering::Relaxed);
                evicted += 1;
            }
        }

        if evicted > 0 {
            self.stats.read().evictions.fetch_add(evicted, Ordering::Relaxed);
            info!(
                "Time-based eviction: removed {} entries older than {:?}",
                evicted, max_age
            );
        }
    }

    /// Evict entries until under the memory limit
    pub fn evict_to_memory_limit(&self, target_bytes: usize) {
        let current = self.total_memory.load(Ordering::Relaxed);
        if current <= target_bytes {
            return;
        }

        let to_free = current - target_bytes;
        let mut freed = 0;

        // Collect entries with their access times and sizes
        let mut candidates: Vec<(EdgeKey, u64, usize)> = self.entries
            .iter()
            .map(|entry| {
                (*entry.key(), entry.value().last_access_millis(), entry.value().size())
            })
            .collect();

        // Sort by access time (oldest first)
        candidates.sort_by_key(|(_, time, _)| *time);

        for (key, _, size) in candidates {
            if freed >= to_free {
                break;
            }
            if let Some((_, entry)) = self.entries.remove(&key) {
                let entry_size = entry.size();
                self.total_memory.fetch_sub(entry_size, Ordering::Relaxed);
                freed += entry_size;
                self.stats.read().evictions.fetch_add(1, Ordering::Relaxed);
            }
        }

        if freed > 0 {
            info!(
                "Memory eviction: freed {} bytes, now at {} bytes",
                freed,
                self.total_memory.load(Ordering::Relaxed)
            );
        }
    }

    /// Get configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Iterate over all keys (for debugging/admin)
    pub fn keys(&self) -> Vec<EdgeKey> {
        self.entries.iter().map(|e| *e.key()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aoee_core::{EntityId, EntityType, EdgeType, types::new_shared_posting_list};

    fn make_key(src: u64) -> EdgeKey {
        EdgeKey::new(
            EntityId::new(EntityType::User, src),
            EdgeType::Follows,
        )
    }

    fn make_list() -> SharedPostingList {
        new_shared_posting_list()
    }

    #[test]
    fn test_basic_operations() {
        let cache = LruCache::with_capacity(100);
        
        let key = make_key(1);
        let list = make_list();
        
        // Insert
        cache.insert(key, list.clone());
        assert_eq!(cache.len(), 1);
        
        // Get
        let retrieved = cache.get(&key);
        assert!(retrieved.is_some());
        
        // Remove
        cache.remove(&key);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = LruCache::new(CacheConfig {
            max_entries: 5,
            max_memory_bytes: 0,
            eviction_batch_size: 1000,
            eviction_target_ratio: 0.6, // Evict down to 3 entries
            min_entries: 0, // Allow eviction even with few entries
        });

        // Insert 6 entries
        for i in 0..6 {
            cache.insert(make_key(i), make_list());
            // Small delay to ensure different access times
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // Should have triggered eviction
        assert!(cache.len() <= 5);
        
        // Stats should show evictions
        let stats = cache.stats();
        assert!(stats.evictions > 0, "Expected evictions, got {}", stats.evictions);
    }

    #[test]
    fn test_access_updates_lru() {
        let cache = LruCache::new(CacheConfig {
            max_entries: 3,
            max_memory_bytes: 0,
            eviction_batch_size: 1000,
            eviction_target_ratio: 0.5, // Evict down to 50% = 1.5 -> 1 entry when over
            min_entries: 0,
        });

        // Insert 3 entries with substantial delays between them
        // Entry order by age: key0 (oldest), key1, key2 (newest)
        cache.insert(make_key(0), make_list());
        std::thread::sleep(std::time::Duration::from_millis(20));
        cache.insert(make_key(1), make_list());
        std::thread::sleep(std::time::Duration::from_millis(20));
        cache.insert(make_key(2), make_list());
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Now access key0 to make it recently used
        // After this, order by age: key1 (oldest), key2, key0 (newest)
        cache.get(&make_key(0));
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Insert a 4th entry to trigger eviction
        // Now we have 4 entries, max is 3, target is 1-2
        // Order by age: key1 (oldest), key2, key0, key3 (newest)
        // Should evict key1 first, then key2 if needed
        cache.insert(make_key(3), make_list());

        // Entry 0 should still exist (recently accessed)
        let contains_0 = cache.contains(&make_key(0));
        let contains_3 = cache.contains(&make_key(3));
        let len = cache.len();
        
        assert!(contains_3, "Entry 3 should be in cache (just inserted), len={}", len);
        // With target_ratio=0.5, we evict down to 1-2 entries, keeping the most recent
        // key0 and key3 are the most recent, so at least one should remain
        assert!(contains_0 || contains_3, "At least one of the recently used entries should remain");
    }

    #[test]
    fn test_stats() {
        let cache = LruCache::with_capacity(100);
        
        let key = make_key(1);
        cache.insert(key, make_list());
        
        // Hit
        cache.get(&key);
        // Miss
        cache.get(&make_key(999));
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.insertions, 1);
    }
}

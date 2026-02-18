//! Shard configuration

use crate::cache::CacheConfig;
use aoee_core::compaction::CompactionConfig;
use serde::{Deserialize, Serialize};

/// Configuration for a shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Shard identifier
    pub shard_id: u32,
    /// Compaction settings
    pub compaction: CompactionConfig,
    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfigSerde,
    /// Enable background compaction
    pub background_compaction: bool,
    /// Compaction check interval in milliseconds
    pub compaction_interval_ms: u64,
    /// Enable write-through to storage
    pub write_through: bool,
    /// TTL for cache entries in seconds (0 = no TTL)
    pub cache_ttl_seconds: u64,
}

/// Serializable version of CacheConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfigSerde {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Maximum memory in bytes (0 = unlimited)
    pub max_memory_bytes: usize,
    /// Target utilization after eviction (0.0-1.0)
    pub eviction_target_ratio: f64,
    /// Minimum entries to keep regardless of memory pressure
    pub min_entries: usize,
}

impl Default for CacheConfigSerde {
    fn default() -> Self {
        CacheConfigSerde {
            max_entries: 100_000,
            max_memory_bytes: 0,
            eviction_target_ratio: 0.9,
            min_entries: 1000,
        }
    }
}

impl From<CacheConfigSerde> for CacheConfig {
    fn from(s: CacheConfigSerde) -> Self {
        CacheConfig {
            max_entries: s.max_entries,
            max_memory_bytes: s.max_memory_bytes,
            eviction_target_ratio: s.eviction_target_ratio,
            min_entries: s.min_entries,
            eviction_batch_size: 1000,
        }
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        ShardConfig {
            shard_id: 0,
            compaction: CompactionConfig::default(),
            cache: CacheConfigSerde::default(),
            background_compaction: true,
            compaction_interval_ms: 1000,
            write_through: true,
            cache_ttl_seconds: 0, // No TTL by default
        }
    }
}

impl ShardConfig {
    pub fn new(shard_id: u32) -> Self {
        ShardConfig {
            shard_id,
            ..Default::default()
        }
    }

    pub fn with_compaction(mut self, config: CompactionConfig) -> Self {
        self.compaction = config;
        self
    }

    pub fn with_write_through(mut self, enabled: bool) -> Self {
        self.write_through = enabled;
        self
    }

    pub fn with_background_compaction(mut self, enabled: bool) -> Self {
        self.background_compaction = enabled;
        self
    }

    pub fn with_max_cache_entries(mut self, max_entries: usize) -> Self {
        self.cache.max_entries = max_entries;
        self
    }

    pub fn with_max_cache_memory(mut self, max_bytes: usize) -> Self {
        self.cache.max_memory_bytes = max_bytes;
        self
    }

    pub fn with_cache_ttl(mut self, ttl_seconds: u64) -> Self {
        self.cache_ttl_seconds = ttl_seconds;
        self
    }

    /// Get the cache configuration
    pub fn cache_config(&self) -> CacheConfig {
        self.cache.clone().into()
    }
}

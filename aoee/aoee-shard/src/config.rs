//! Shard configuration

use aoee_core::compaction::CompactionConfig;
use serde::{Deserialize, Serialize};

/// Configuration for a shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Shard identifier
    pub shard_id: u32,
    /// Compaction settings
    pub compaction: CompactionConfig,
    /// Maximum number of posting lists to cache
    pub max_cached_lists: usize,
    /// Enable background compaction
    pub background_compaction: bool,
    /// Compaction check interval in milliseconds
    pub compaction_interval_ms: u64,
    /// Enable write-through to storage
    pub write_through: bool,
}

impl Default for ShardConfig {
    fn default() -> Self {
        ShardConfig {
            shard_id: 0,
            compaction: CompactionConfig::default(),
            max_cached_lists: 100_000,
            background_compaction: true,
            compaction_interval_ms: 1000,
            write_through: true,
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
}

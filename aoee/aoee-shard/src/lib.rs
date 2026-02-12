//! AOEE Shard - Shard management and routing
//!
//! Provides sharding with consistent hashing for horizontal scalability.
//! Each shard owns a range of keys and manages its own posting lists.

pub mod config;
pub mod shard;
pub mod manager;
pub mod hash;

pub use config::ShardConfig;
pub use shard::Shard;
pub use manager::ShardManager;
pub use hash::ConsistentHash;

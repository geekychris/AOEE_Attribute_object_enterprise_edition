//! AOEE Storage - Persistence abstraction layer
//!
//! Provides a trait-based abstraction for edge persistence, allowing
//! pluggable backends like RocksDB, HTTP services, or in-memory storage.
//!
//! # Feature Flags
//!
//! - `rocksdb-backend`: Enable RocksDB persistent storage
//! - `http-backend`: Enable HTTP storage (calls external REST API)
//! - `all-backends`: Enable all storage backends

pub mod traits;
pub mod memory;
pub mod noop;

#[cfg(feature = "rocksdb-backend")]
pub mod rocksdb;

#[cfg(feature = "http-backend")]
pub mod http;

pub use traits::{EdgeStore, StorageError, StorageStats, StoredEdge};
pub use memory::InMemoryStore;
pub use noop::NoopStore;

#[cfg(feature = "rocksdb-backend")]
pub use rocksdb::RocksDbStore;

#[cfg(feature = "http-backend")]
pub use http::HttpStore;

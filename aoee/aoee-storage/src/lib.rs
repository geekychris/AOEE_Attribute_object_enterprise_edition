//! AOEE Storage - Persistence abstraction layer
//!
//! Provides a trait-based abstraction for edge persistence, allowing
//! pluggable backends like DynamoDB, HTTP services, or in-memory storage.

pub mod traits;
pub mod memory;
pub mod noop;

pub use traits::{EdgeStore, StorageError};
pub use memory::InMemoryStore;
pub use noop::NoopStore;

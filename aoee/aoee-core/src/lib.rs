//! AOEE Core - Attribute Object Enterprise Edition
//!
//! Core data structures and algorithms for the AOEE relationship cache.
//! Provides compressed postings lists with set operations optimized for
//! social graph queries.

pub mod id;
pub mod types;
pub mod encoding;
pub mod iterator;
pub mod set_ops;
pub mod posting_list;
pub mod compaction;
pub mod fof;

// Re-exports for convenience
pub use id::{EntityId, EntityType};
pub use types::{EdgeKey, EdgeType, BufferEntry, WriteBuffer, Segment, PostingList};
pub use encoding::{EncodedList, EncodingStrategy, PostingEncoder};
pub use iterator::{PostingIterator, MergedIterator};
pub use set_ops::{intersect, intersect_galloping, intersect_slices, union, union_slices, difference};
pub use compaction::{CompactionConfig, Compactor};
pub use fof::{FofConfig, FofQuery, FofResult};

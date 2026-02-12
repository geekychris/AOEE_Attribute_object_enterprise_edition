//! High-level posting list operations.
//!
//! Provides a convenient API for working with posting lists,
//! including reads, writes, and iteration.

use crate::compaction::{CompactionConfig, Compactor, CompactionStats};
use crate::encoding::{AutoEncoder, PostingEncoder};
use crate::id::EntityId;
use crate::iterator::{posting_list_iterator, VecIterator, PostingIterator};
use crate::set_ops;
use crate::types::{BufferEntry, PostingList, Segment, WriteBuffer};
use std::sync::Arc;

/// Operations on a posting list
impl PostingList {
    /// Add an edge to the posting list
    pub fn add(&mut self, dst: EntityId, timestamp: u64) {
        self.buffer.push(BufferEntry::add(dst, timestamp));
        self.last_modified = timestamp;
    }

    /// Add an edge with metadata to the posting list
    pub fn add_with_metadata(&mut self, dst: EntityId, timestamp: u64, metadata: u8) {
        self.buffer.push(BufferEntry::add_with_metadata(dst, timestamp, metadata));
        self.last_modified = timestamp;
    }

    /// Delete an edge from the posting list
    pub fn delete(&mut self, dst: EntityId, timestamp: u64) {
        self.buffer.push(BufferEntry::delete(dst, timestamp));
        self.last_modified = timestamp;
    }

    /// Get all neighbors (destination IDs) as a vector
    pub fn neighbors(&self) -> Vec<EntityId> {
        let iter = posting_list_iterator(&self.segments, &self.buffer);
        iter.collect()
    }

    /// Get neighbors with a limit
    pub fn neighbors_limited(&self, limit: usize) -> Vec<EntityId> {
        let iter = posting_list_iterator(&self.segments, &self.buffer);
        iter.take(limit).collect()
    }

    /// Get neighbors with their timestamps and metadata.
    /// Note: metadata is only available for buffer entries (not yet compacted).
    /// Returns (entity_id, timestamp, metadata) for each neighbor.
    pub fn neighbors_with_metadata(&self) -> Vec<(EntityId, u64, u8)> {
        use std::collections::HashMap;
        
        // Build a map of dst -> (timestamp, metadata) from buffer
        let snapshot = self.buffer.snapshot();
        let mut buffer_meta: HashMap<EntityId, (u64, u8, bool)> = HashMap::new();
        for entry in snapshot.iter() {
            buffer_meta.insert(entry.dst, (entry.timestamp, entry.metadata, entry.tombstone));
        }
        
        // Get all neighbors (merged and sorted)
        let neighbors = self.neighbors();
        
        // Build result with metadata where available
        neighbors.into_iter().map(|id| {
            if let Some(&(ts, meta, _tombstone)) = buffer_meta.get(&id) {
                (id, ts, meta)
            } else {
                // From segment - no metadata available
                (id, 0, 0)
            }
        }).collect()
    }

    /// Get neighbors with metadata, limited
    pub fn neighbors_with_metadata_limited(&self, limit: usize) -> Vec<(EntityId, u64, u8)> {
        let all = self.neighbors_with_metadata();
        all.into_iter().take(limit).collect()
    }

    /// Check if a destination ID exists in this posting list
    pub fn contains(&self, dst: EntityId) -> bool {
        // Check buffer first (most recent)
        let snapshot = self.buffer.snapshot();
        for entry in snapshot.iter().rev() {
            if entry.dst == dst {
                return !entry.tombstone;
            }
        }
        
        // Check segments
        for segment in &self.segments {
            if segment.might_contain(dst) {
                let ids = AutoEncoder::decode(&segment.data).unwrap_or_default();
                if ids.binary_search(&dst).is_ok() {
                    return true;
                }
            }
        }
        
        false
    }

    /// Get approximate count of edges
    pub fn count(&self) -> usize {
        // This is approximate; exact count requires full iteration
        let segment_count: usize = self.segments.iter().map(|s| s.count as usize).sum();
        let buffer_adds = self.buffer.entries().iter().filter(|e| !e.tombstone).count();
        let buffer_deletes = self.buffer.entries().iter().filter(|e| e.tombstone).count();
        
        segment_count + buffer_adds - buffer_deletes.min(segment_count + buffer_adds)
    }

    /// Get exact count (requires full iteration)
    pub fn exact_count(&self) -> usize {
        self.neighbors().len()
    }

    /// Compact the posting list if needed
    pub fn compact(&mut self, config: &CompactionConfig) -> Option<CompactionStats> {
        let compactor = Compactor::new(config.clone());
        
        if !compactor.needs_compaction(self) && !compactor.needs_merge(self) {
            return None;
        }
        
        let (new_segments, _, stats) = compactor.full_compact(self);
        
        // Update the posting list
        self.segments = new_segments;
        self.buffer = WriteBuffer::new();
        self.total_count = self.segments.iter().map(|s| s.count as u64).sum();
        
        Some(stats)
    }

    /// Force compaction regardless of thresholds
    pub fn force_compact(&mut self, config: &CompactionConfig) -> CompactionStats {
        let compactor = Compactor::new(config.clone());
        let (new_segments, _, stats) = compactor.full_compact(self);
        
        self.segments = new_segments;
        self.buffer = WriteBuffer::new();
        self.total_count = self.segments.iter().map(|s| s.count as u64).sum();
        
        stats
    }

    /// Intersect this posting list with another
    pub fn intersect(&self, other: &PostingList) -> Vec<EntityId> {
        let iter_a = posting_list_iterator(&self.segments, &self.buffer);
        let iter_b = posting_list_iterator(&other.segments, &other.buffer);
        
        set_ops::intersect(iter_a, iter_b)
    }

    /// Union this posting list with another
    pub fn union(&self, other: &PostingList) -> Vec<EntityId> {
        let iter_a = posting_list_iterator(&self.segments, &self.buffer);
        let iter_b = posting_list_iterator(&other.segments, &other.buffer);
        
        set_ops::union(iter_a, iter_b)
    }

    /// Difference: elements in this list but not in other
    pub fn difference(&self, other: &PostingList) -> Vec<EntityId> {
        let iter_a = posting_list_iterator(&self.segments, &self.buffer);
        let iter_b = posting_list_iterator(&other.segments, &other.buffer);
        
        set_ops::difference(iter_a, iter_b)
    }

    /// Create from a vector of IDs (for testing/initialization)
    pub fn from_ids(ids: &[EntityId], timestamp: u64) -> Self {
        let mut pl = PostingList::new();
        for &id in ids {
            pl.add(id, timestamp);
        }
        pl
    }

    /// Create from a vector of IDs with immediate compaction
    pub fn from_ids_compacted(ids: &[EntityId], timestamp: u64) -> Self {
        let mut pl = Self::from_ids(ids, timestamp);
        pl.force_compact(&CompactionConfig::default());
        pl
    }
}

/// Intersect two posting lists directly from their components
pub fn intersect_posting_lists(a: &PostingList, b: &PostingList) -> Vec<EntityId> {
    a.intersect(b)
}

/// Union two posting lists
pub fn union_posting_lists(a: &PostingList, b: &PostingList) -> Vec<EntityId> {
    a.union(b)
}

/// Intersect a posting list with a vector of IDs
pub fn intersect_with_ids(pl: &PostingList, ids: &[EntityId]) -> Vec<EntityId> {
    let iter_a = posting_list_iterator(&pl.segments, &pl.buffer);
    let iter_b = VecIterator::new(ids.to_vec());
    set_ops::intersect(iter_a, iter_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityType;

    fn make_id(raw: u64) -> EntityId {
        EntityId::new(EntityType::User, raw)
    }

    fn make_ids(raws: &[u64]) -> Vec<EntityId> {
        raws.iter().map(|&r| make_id(r)).collect()
    }

    #[test]
    fn test_add_and_neighbors() {
        let mut pl = PostingList::new();
        
        pl.add(make_id(3), 100);
        pl.add(make_id(1), 101);
        pl.add(make_id(2), 102);
        
        let neighbors = pl.neighbors();
        assert_eq!(neighbors, make_ids(&[1, 2, 3]));
    }

    #[test]
    fn test_delete() {
        let mut pl = PostingList::new();
        
        pl.add(make_id(1), 100);
        pl.add(make_id(2), 101);
        pl.add(make_id(3), 102);
        pl.delete(make_id(2), 103);
        
        let neighbors = pl.neighbors();
        assert_eq!(neighbors, make_ids(&[1, 3]));
    }

    #[test]
    fn test_contains() {
        let mut pl = PostingList::new();
        
        pl.add(make_id(1), 100);
        pl.add(make_id(2), 101);
        pl.add(make_id(3), 102);
        
        assert!(pl.contains(make_id(2)));
        assert!(!pl.contains(make_id(5)));
    }

    #[test]
    fn test_contains_with_delete() {
        let mut pl = PostingList::new();
        
        pl.add(make_id(1), 100);
        pl.add(make_id(2), 101);
        pl.delete(make_id(2), 102);
        
        assert!(pl.contains(make_id(1)));
        assert!(!pl.contains(make_id(2)));
    }

    #[test]
    fn test_neighbors_limited() {
        let mut pl = PostingList::new();
        
        for i in 1..=10 {
            pl.add(make_id(i), 100);
        }
        
        let limited = pl.neighbors_limited(5);
        assert_eq!(limited.len(), 5);
        assert_eq!(limited, make_ids(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_compact() {
        let mut pl = PostingList::new();
        
        for i in 1..=100 {
            pl.add(make_id(i), 100);
        }
        
        assert_eq!(pl.buffer_len(), 100);
        assert_eq!(pl.segment_count(), 0);
        
        let config = CompactionConfig {
            buffer_threshold: 50,
            ..Default::default()
        };
        
        let stats = pl.compact(&config);
        assert!(stats.is_some());
        
        assert_eq!(pl.buffer_len(), 0);
        assert!(pl.segment_count() > 0);
        
        // Verify data integrity
        let neighbors = pl.neighbors();
        assert_eq!(neighbors.len(), 100);
    }

    #[test]
    fn test_intersect() {
        let pl1 = PostingList::from_ids(&make_ids(&[1, 2, 3, 4, 5]), 100);
        let pl2 = PostingList::from_ids(&make_ids(&[3, 4, 5, 6, 7]), 100);
        
        let result = pl1.intersect(&pl2);
        assert_eq!(result, make_ids(&[3, 4, 5]));
    }

    #[test]
    fn test_union() {
        let pl1 = PostingList::from_ids(&make_ids(&[1, 2, 3]), 100);
        let pl2 = PostingList::from_ids(&make_ids(&[3, 4, 5]), 100);
        
        let result = pl1.union(&pl2);
        assert_eq!(result, make_ids(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_difference() {
        let pl1 = PostingList::from_ids(&make_ids(&[1, 2, 3, 4, 5]), 100);
        let pl2 = PostingList::from_ids(&make_ids(&[2, 4]), 100);
        
        let result = pl1.difference(&pl2);
        assert_eq!(result, make_ids(&[1, 3, 5]));
    }

    #[test]
    fn test_from_ids_compacted() {
        let ids = make_ids(&[5, 3, 1, 4, 2]);
        let pl = PostingList::from_ids_compacted(&ids, 100);
        
        assert_eq!(pl.buffer_len(), 0);
        assert!(pl.segment_count() > 0);
        
        let neighbors = pl.neighbors();
        assert_eq!(neighbors, make_ids(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_count() {
        let mut pl = PostingList::new();
        
        for i in 1..=10 {
            pl.add(make_id(i), 100);
        }
        
        assert_eq!(pl.count(), 10);
        assert_eq!(pl.exact_count(), 10);
        
        pl.delete(make_id(5), 101);
        // count() is approximate, exact_count() is precise
        assert_eq!(pl.exact_count(), 9);
    }
}

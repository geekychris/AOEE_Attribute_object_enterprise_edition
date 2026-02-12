//! Iterators for efficient streaming over posting lists.
//!
//! Provides iterators that decode data on-the-fly to avoid materializing
//! large vectors in memory. Supports merging multiple sorted streams.

use crate::encoding::{AutoEncoder, EncodedList, PostingEncoder};
use crate::id::EntityId;
use crate::types::{BufferEntry, Segment, WriteBuffer};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

/// Trait for iterating over sorted EntityIds
pub trait PostingIterator: Iterator<Item = EntityId> {
    /// Peek at the next value without consuming it
    fn peek(&self) -> Option<EntityId>;
    
    /// Seek to the first value >= target, returning it if found
    fn seek(&mut self, target: EntityId) -> Option<EntityId>;
    
    /// Get approximate remaining count (may be inaccurate)
    fn size_hint_lower(&self) -> usize;
}

/// Blanket implementation for boxed trait objects
impl PostingIterator for Box<dyn PostingIterator + Send> {
    fn peek(&self) -> Option<EntityId> {
        (**self).peek()
    }
    
    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        (**self).seek(target)
    }
    
    fn size_hint_lower(&self) -> usize {
        (**self).size_hint_lower()
    }
}

// ============================================================================
// Vec Iterator (for SmallVec and materialized lists)
// ============================================================================

/// Iterator over a sorted slice of EntityIds
pub struct VecIterator {
    ids: Vec<EntityId>,
    pos: usize,
}

impl VecIterator {
    pub fn new(ids: Vec<EntityId>) -> Self {
        VecIterator { ids, pos: 0 }
    }

    pub fn empty() -> Self {
        VecIterator {
            ids: Vec::new(),
            pos: 0,
        }
    }
}

impl Iterator for VecIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.ids.len() {
            let id = self.ids[self.pos];
            self.pos += 1;
            Some(id)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.ids.len() - self.pos;
        (remaining, Some(remaining))
    }
}

impl PostingIterator for VecIterator {
    fn peek(&self) -> Option<EntityId> {
        if self.pos < self.ids.len() {
            Some(self.ids[self.pos])
        } else {
            None
        }
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        // Binary search from current position
        let remaining = &self.ids[self.pos..];
        match remaining.binary_search(&target) {
            Ok(idx) => {
                self.pos += idx;
                Some(self.ids[self.pos])
            }
            Err(idx) => {
                self.pos += idx;
                self.peek()
            }
        }
    }

    fn size_hint_lower(&self) -> usize {
        self.ids.len() - self.pos
    }
}

// ============================================================================
// Segment Iterator (decode on-the-fly)
// ============================================================================

/// Iterator that decodes a segment on-the-fly
/// 
/// For now, we materialize the segment into a VecIterator.
/// A more optimized version would decode incrementally.
pub struct SegmentIterator {
    inner: VecIterator,
}

impl SegmentIterator {
    pub fn new(segment: &Segment) -> Self {
        let ids = AutoEncoder::decode(&segment.data).unwrap_or_default();
        SegmentIterator {
            inner: VecIterator::new(ids),
        }
    }

    pub fn from_encoded(encoded: &EncodedList) -> Self {
        let ids = AutoEncoder::decode(encoded).unwrap_or_default();
        SegmentIterator {
            inner: VecIterator::new(ids),
        }
    }
}

impl Iterator for SegmentIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl PostingIterator for SegmentIterator {
    fn peek(&self) -> Option<EntityId> {
        self.inner.peek()
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        self.inner.seek(target)
    }

    fn size_hint_lower(&self) -> usize {
        self.inner.size_hint_lower()
    }
}

// ============================================================================
// Buffer Iterator (sorted snapshot of write buffer)
// ============================================================================

/// Iterator over write buffer entries (excluding tombstones)
pub struct BufferIterator {
    entries: Vec<BufferEntry>,
    pos: usize,
}

impl BufferIterator {
    pub fn new(buffer: &WriteBuffer) -> Self {
        let mut entries = buffer.snapshot();
        // Filter out tombstones
        entries.retain(|e| !e.tombstone);
        BufferIterator { entries, pos: 0 }
    }

    pub fn with_tombstones(buffer: &WriteBuffer) -> (Self, Vec<EntityId>) {
        let entries = buffer.snapshot();
        
        // Collect all tombstoned IDs
        let mut tombstone_set: std::collections::HashSet<EntityId> = std::collections::HashSet::new();
        for entry in &entries {
            if entry.tombstone {
                tombstone_set.insert(entry.dst);
            }
        }
        
        let tombstones: Vec<EntityId> = tombstone_set.iter().copied().collect();
        
        // Filter live entries to exclude any that have tombstones
        let live: Vec<BufferEntry> = entries
            .into_iter()
            .filter(|e| !e.tombstone && !tombstone_set.contains(&e.dst))
            .collect();
        
        (BufferIterator { entries: live, pos: 0 }, tombstones)
    }
}

impl Iterator for BufferIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.entries.len() {
            let id = self.entries[self.pos].dst;
            self.pos += 1;
            Some(id)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.entries.len() - self.pos;
        (remaining, Some(remaining))
    }
}

impl PostingIterator for BufferIterator {
    fn peek(&self) -> Option<EntityId> {
        if self.pos < self.entries.len() {
            Some(self.entries[self.pos].dst)
        } else {
            None
        }
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        let remaining = &self.entries[self.pos..];
        match remaining.binary_search_by_key(&target, |e| e.dst) {
            Ok(idx) => {
                self.pos += idx;
                Some(self.entries[self.pos].dst)
            }
            Err(idx) => {
                self.pos += idx;
                self.peek()
            }
        }
    }

    fn size_hint_lower(&self) -> usize {
        self.entries.len() - self.pos
    }
}

// ============================================================================
// Filtered Iterator (exclude tombstoned IDs)
// ============================================================================

/// Iterator that filters out tombstoned IDs
pub struct FilteredIterator<I: PostingIterator> {
    inner: I,
    tombstones: Vec<EntityId>,
    tombstone_pos: usize,
}

impl<I: PostingIterator> FilteredIterator<I> {
    pub fn new(inner: I, mut tombstones: Vec<EntityId>) -> Self {
        tombstones.sort();
        tombstones.dedup();
        FilteredIterator {
            inner,
            tombstones,
            tombstone_pos: 0,
        }
    }

    fn is_tombstoned(&mut self, id: EntityId) -> bool {
        // Advance tombstone position
        while self.tombstone_pos < self.tombstones.len() 
            && self.tombstones[self.tombstone_pos] < id 
        {
            self.tombstone_pos += 1;
        }
        
        self.tombstone_pos < self.tombstones.len() 
            && self.tombstones[self.tombstone_pos] == id
    }
}

impl<I: PostingIterator> Iterator for FilteredIterator<I> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(id) => {
                    if !self.is_tombstoned(id) {
                        return Some(id);
                    }
                    // Skip tombstoned, continue loop
                }
                None => return None,
            }
        }
    }
}

impl<I: PostingIterator> PostingIterator for FilteredIterator<I> {
    fn peek(&self) -> Option<EntityId> {
        self.inner.peek()
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        self.inner.seek(target)
    }

    fn size_hint_lower(&self) -> usize {
        self.inner.size_hint_lower().saturating_sub(self.tombstones.len())
    }
}

// ============================================================================
// Merged Iterator (merge multiple sorted streams)
// ============================================================================

/// Wrapper for heap ordering (min-heap via Reverse)
struct HeapEntry<I: PostingIterator> {
    current: EntityId,
    iter: I,
    index: usize, // For stable ordering
}

impl<I: PostingIterator> PartialEq for HeapEntry<I> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current && self.index == other.index
    }
}

impl<I: PostingIterator> Eq for HeapEntry<I> {}

impl<I: PostingIterator> PartialOrd for HeapEntry<I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: PostingIterator> Ord for HeapEntry<I> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.current.cmp(&self.current)
            .then_with(|| other.index.cmp(&self.index))
    }
}

/// Merges multiple sorted iterators into a single sorted stream.
/// Duplicates are emitted only once.
pub struct MergedIterator<I: PostingIterator> {
    heap: BinaryHeap<HeapEntry<I>>,
    last_emitted: Option<EntityId>,
}

impl<I: PostingIterator> MergedIterator<I> {
    pub fn new(iterators: Vec<I>) -> Self {
        let mut heap = BinaryHeap::with_capacity(iterators.len());
        
        for (index, mut iter) in iterators.into_iter().enumerate() {
            if let Some(current) = iter.next() {
                heap.push(HeapEntry { current, iter, index });
            }
        }
        
        MergedIterator {
            heap,
            last_emitted: None,
        }
    }

    pub fn from_two(iter1: I, iter2: I) -> Self {
        Self::new(vec![iter1, iter2])
    }
}

impl<I: PostingIterator> Iterator for MergedIterator<I> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut entry = self.heap.pop()?;
            let current = entry.current;
            
            // Advance the iterator and re-insert if more elements
            if let Some(next) = entry.iter.next() {
                entry.current = next;
                self.heap.push(entry);
            }
            
            // Skip duplicates
            if self.last_emitted == Some(current) {
                continue;
            }
            
            self.last_emitted = Some(current);
            return Some(current);
        }
    }
}

impl<I: PostingIterator> PostingIterator for MergedIterator<I> {
    fn peek(&self) -> Option<EntityId> {
        self.heap.peek().map(|e| e.current)
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        // Seek all iterators and rebuild heap
        let mut new_entries = Vec::with_capacity(self.heap.len());
        
        while let Some(mut entry) = self.heap.pop() {
            if entry.current < target {
                if let Some(id) = entry.iter.seek(target) {
                    entry.current = id;
                    new_entries.push(entry);
                }
            } else {
                new_entries.push(entry);
            }
        }
        
        for entry in new_entries {
            self.heap.push(entry);
        }
        
        self.peek()
    }

    fn size_hint_lower(&self) -> usize {
        self.heap.iter().map(|e| e.iter.size_hint_lower() + 1).sum()
    }
}

// ============================================================================
// Utility: Create iterator from posting list components
// ============================================================================

/// Create a merged iterator from segments and buffer
pub fn posting_list_iterator(
    segments: &[Arc<Segment>],
    buffer: &WriteBuffer,
) -> MergedIterator<Box<dyn PostingIterator + Send>> {
    let mut iterators: Vec<Box<dyn PostingIterator + Send>> = Vec::new();
    
    // Add segment iterators
    for segment in segments {
        iterators.push(Box::new(SegmentIterator::new(segment)));
    }
    
    // Add buffer iterator (with tombstone filtering applied to segments)
    let (buffer_iter, tombstones) = BufferIterator::with_tombstones(buffer);
    
    if !tombstones.is_empty() {
        // Wrap segment iterators with tombstone filter
        let mut filtered_iters: Vec<Box<dyn PostingIterator + Send>> = Vec::new();
        for iter in iterators {
            filtered_iters.push(Box::new(FilteredIterator::new(iter, tombstones.clone())));
        }
        filtered_iters.push(Box::new(buffer_iter));
        MergedIterator::new(filtered_iters)
    } else {
        iterators.push(Box::new(buffer_iter));
        MergedIterator::new(iterators)
    }
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
    fn test_vec_iterator_basic() {
        let ids = make_ids(&[1, 3, 5, 7, 9]);
        let mut iter = VecIterator::new(ids.clone());
        
        assert_eq!(iter.peek(), Some(make_id(1)));
        assert_eq!(iter.next(), Some(make_id(1)));
        assert_eq!(iter.peek(), Some(make_id(3)));
        assert_eq!(iter.next(), Some(make_id(3)));
    }

    #[test]
    fn test_vec_iterator_seek() {
        let ids = make_ids(&[1, 3, 5, 7, 9]);
        let mut iter = VecIterator::new(ids);
        
        // Seek to existing value
        assert_eq!(iter.seek(make_id(5)), Some(make_id(5)));
        assert_eq!(iter.next(), Some(make_id(5)));
        
        // Seek to non-existing value
        let mut iter2 = VecIterator::new(make_ids(&[1, 3, 5, 7, 9]));
        assert_eq!(iter2.seek(make_id(4)), Some(make_id(5)));
    }

    #[test]
    fn test_buffer_iterator() {
        let mut buffer = WriteBuffer::new();
        buffer.push(BufferEntry::add(make_id(5), 100));
        buffer.push(BufferEntry::add(make_id(1), 101));
        buffer.push(BufferEntry::add(make_id(3), 102));
        
        let iter = BufferIterator::new(&buffer);
        let collected: Vec<_> = iter.collect();
        
        assert_eq!(collected, make_ids(&[1, 3, 5]));
    }

    #[test]
    fn test_buffer_iterator_with_tombstones() {
        let mut buffer = WriteBuffer::new();
        buffer.push(BufferEntry::add(make_id(1), 100));
        buffer.push(BufferEntry::add(make_id(3), 101));
        buffer.push(BufferEntry::delete(make_id(3), 102)); // Tombstone
        buffer.push(BufferEntry::add(make_id(5), 103));
        
        let (iter, tombstones) = BufferIterator::with_tombstones(&buffer);
        
        assert_eq!(tombstones, vec![make_id(3)]);
        let collected: Vec<_> = iter.collect();
        assert_eq!(collected, make_ids(&[1, 5]));
    }

    #[test]
    fn test_merged_iterator_two_lists() {
        let iter1 = VecIterator::new(make_ids(&[1, 3, 5, 7]));
        let iter2 = VecIterator::new(make_ids(&[2, 4, 6, 8]));
        
        let merged = MergedIterator::from_two(iter1, iter2);
        let collected: Vec<_> = merged.collect();
        
        assert_eq!(collected, make_ids(&[1, 2, 3, 4, 5, 6, 7, 8]));
    }

    #[test]
    fn test_merged_iterator_overlapping() {
        let iter1 = VecIterator::new(make_ids(&[1, 3, 5]));
        let iter2 = VecIterator::new(make_ids(&[3, 5, 7]));
        
        let merged = MergedIterator::from_two(iter1, iter2);
        let collected: Vec<_> = merged.collect();
        
        // Duplicates should be removed
        assert_eq!(collected, make_ids(&[1, 3, 5, 7]));
    }

    #[test]
    fn test_merged_iterator_empty() {
        let iter1 = VecIterator::empty();
        let iter2 = VecIterator::new(make_ids(&[1, 2, 3]));
        
        let merged = MergedIterator::from_two(iter1, iter2);
        let collected: Vec<_> = merged.collect();
        
        assert_eq!(collected, make_ids(&[1, 2, 3]));
    }

    #[test]
    fn test_merged_iterator_many() {
        let iters = vec![
            VecIterator::new(make_ids(&[1, 10, 100])),
            VecIterator::new(make_ids(&[2, 20, 200])),
            VecIterator::new(make_ids(&[3, 30, 300])),
        ];
        
        let merged = MergedIterator::new(iters);
        let collected: Vec<_> = merged.collect();
        
        assert_eq!(collected, make_ids(&[1, 2, 3, 10, 20, 30, 100, 200, 300]));
    }

    #[test]
    fn test_filtered_iterator() {
        let inner = VecIterator::new(make_ids(&[1, 2, 3, 4, 5]));
        let tombstones = vec![make_id(2), make_id(4)];
        
        let filtered = FilteredIterator::new(inner, tombstones);
        let collected: Vec<_> = filtered.collect();
        
        assert_eq!(collected, make_ids(&[1, 3, 5]));
    }

    #[test]
    fn test_merged_iterator_seek() {
        let iter1 = VecIterator::new(make_ids(&[1, 5, 10]));
        let iter2 = VecIterator::new(make_ids(&[2, 6, 11]));
        
        let mut merged = MergedIterator::from_two(iter1, iter2);
        
        assert_eq!(merged.seek(make_id(5)), Some(make_id(5)));
        assert_eq!(merged.next(), Some(make_id(5)));
        assert_eq!(merged.next(), Some(make_id(6)));
    }
}

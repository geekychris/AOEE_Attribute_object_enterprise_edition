//! Iterators for efficient streaming over posting lists.
//!
//! Provides iterators that decode data on-the-fly to avoid materializing
//! large vectors in memory. Supports merging multiple sorted streams.

use crate::encoding::{
    BlockPackedEncoder, DeltaVarintEncoder, EncodedList, BLOCK_SIZE,
};
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
// DeltaVarint Iterator (lazy decode one varint at a time)
// ============================================================================

/// Iterator that lazily decodes delta-varint encoded data one element at a time.
/// No heap allocation beyond the struct itself.
pub struct DeltaVarintIterator {
    /// Raw encoded bytes (owned via Arc<Segment>)
    segment: Arc<Segment>,
    /// Current byte position in the data
    pos: usize,
    /// Running previous value for delta reconstruction
    prev: u64,
    /// Total number of elements
    count: usize,
    /// Number of values decoded from byte stream (including first value and peeked)
    stream_decoded: usize,
    /// Peeked next value (decoded but not yet consumed)
    peeked: Option<EntityId>,
}

impl DeltaVarintIterator {
    pub fn new(segment: Arc<Segment>) -> Self {
        let data = match &segment.data {
            EncodedList::DeltaVarint(d) => d.as_slice(),
            _ => &[],
        };

        if data.is_empty() {
            return DeltaVarintIterator {
                segment,
                pos: 0,
                prev: 0,
                count: 0,
                stream_decoded: 0,
                peeked: None,
            };
        }

        let mut pos = 0;
        let count = DeltaVarintEncoder::decode_varint(data, &mut pos).unwrap_or(0) as usize;

        if count == 0 {
            return DeltaVarintIterator {
                segment,
                pos,
                prev: 0,
                count: 0,
                stream_decoded: 0,
                peeked: None,
            };
        }

        // Decode the first value (stored as absolute, not delta)
        let first = DeltaVarintEncoder::decode_varint(data, &mut pos).unwrap_or(0);
        let peeked = Some(EntityId::from_raw(first));

        DeltaVarintIterator {
            segment,
            pos,
            prev: first,
            count,
            stream_decoded: 1, // first value is decoded
            peeked,
        }
    }

    fn data(&self) -> &[u8] {
        match &self.segment.data {
            EncodedList::DeltaVarint(d) => d.as_slice(),
            _ => &[],
        }
    }

    /// Decode the next raw value from the stream, returning None if exhausted.
    fn decode_next_raw(&mut self) -> Option<EntityId> {
        if self.stream_decoded >= self.count {
            return None;
        }
        // Extract the data slice by matching on the Arc'd segment directly
        // to avoid holding an immutable borrow on `self` across the mutation.
        let data_slice: &[u8] = match &self.segment.data {
            EncodedList::DeltaVarint(d) => {
                // SAFETY: The Arc<Segment> keeps the data alive for the
                // lifetime of self, and we only read from the slice.
                unsafe { &*(d.as_slice() as *const [u8]) }
            }
            _ => return None,
        };
        if self.pos >= data_slice.len() {
            return None;
        }
        let delta = DeltaVarintEncoder::decode_varint(data_slice, &mut self.pos).ok()?;
        self.prev = self.prev.saturating_add(delta);
        self.stream_decoded += 1;
        Some(EntityId::from_raw(self.prev))
    }
}

impl Iterator for DeltaVarintIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(val) = self.peeked.take() {
            // Pre-decode the next value for peek
            self.peeked = self.decode_next_raw();
            return Some(val);
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }
}

impl DeltaVarintIterator {
    /// Number of items remaining (including peeked value)
    fn remaining(&self) -> usize {
        let undecoded = self.count.saturating_sub(self.stream_decoded);
        undecoded + if self.peeked.is_some() { 1 } else { 0 }
    }

    /// Number of items already returned to the caller
    fn items_returned(&self) -> usize {
        // stream_decoded includes the peeked value, so items returned =
        // stream_decoded - (1 if peeked exists)
        self.stream_decoded.saturating_sub(if self.peeked.is_some() { 1 } else { 0 })
    }
}

impl PostingIterator for DeltaVarintIterator {
    fn peek(&self) -> Option<EntityId> {
        self.peeked
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        // Use skip table to jump ahead if possible
        if !self.segment.skip_table.is_empty() {
            let skip_idx = self.segment.skip_table.partition_point(|&(id, _)| id < target);
            let use_idx = if skip_idx > 0 { skip_idx - 1 } else { 0 };
            let &(_skip_id, skip_offset) = &self.segment.skip_table[use_idx];

            // Only jump if skip point is ahead of current position
            if skip_offset as usize >= self.items_returned() {
                let data = self.data();
                let mut pos = 0;
                let _count = DeltaVarintEncoder::decode_varint(data, &mut pos).unwrap_or(0);
                let first = DeltaVarintEncoder::decode_varint(data, &mut pos).unwrap_or(0);

                if skip_offset == 0 {
                    self.pos = pos;
                    self.prev = first;
                    self.stream_decoded = 1;
                    self.peeked = Some(EntityId::from_raw(first));
                } else {
                    let mut prev = first;
                    for _ in 1..=(skip_offset as usize) {
                        let delta = DeltaVarintEncoder::decode_varint(data, &mut pos).unwrap_or(0);
                        prev = prev.saturating_add(delta);
                    }
                    self.pos = pos;
                    self.prev = prev;
                    self.stream_decoded = skip_offset as usize + 1;
                    self.peeked = Some(EntityId::from_raw(prev));
                }
            }
        }

        // Linear scan forward to find >= target
        while let Some(val) = self.peeked {
            if val >= target {
                return Some(val);
            }
            self.next();
        }
        None
    }

    fn size_hint_lower(&self) -> usize {
        self.remaining()
    }
}

// ============================================================================
// BlockPacked Iterator (decode one block at a time)
// ============================================================================

/// Iterator that lazily decodes block-packed data one block at a time.
/// Only one block (128 values) is materialized at any moment.
pub struct BlockPackedIterator {
    /// Owning reference to the segment
    segment: Arc<Segment>,
    /// Total element count
    count: usize,
    /// First absolute value from the header
    first_value: u64,
    /// Byte position after the 12-byte header
    header_end: usize,
    /// Current byte position in the data (start of next unread block header)
    byte_pos: usize,
    /// Currently decoded block of absolute EntityId values
    block_buf: Vec<EntityId>,
    /// Index within block_buf
    block_idx: usize,
    /// Total elements emitted across all blocks
    total_emitted: usize,
    /// Running previous value (last value of previous block, for delta reconstruction)
    running_prev: u64,
}

impl BlockPackedIterator {
    pub fn new(segment: Arc<Segment>) -> Self {
        let data = match &segment.data {
            EncodedList::BlockPacked(d) => d.as_slice(),
            _ => &[],
        };

        if data.len() < 12 {
            return BlockPackedIterator {
                segment,
                count: 0,
                first_value: 0,
                header_end: 0,
                byte_pos: 0,
                block_buf: Vec::new(),
                block_idx: 0,
                total_emitted: 0,
                running_prev: 0,
            };
        }

        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let first_value = u64::from_le_bytes([
            data[4], data[5], data[6], data[7],
            data[8], data[9], data[10], data[11],
        ]);

        let mut iter = BlockPackedIterator {
            segment,
            count,
            first_value,
            header_end: 12,
            byte_pos: 12,
            block_buf: Vec::with_capacity(BLOCK_SIZE),
            block_idx: 0,
            total_emitted: 0,
            running_prev: first_value,
        };

        // Decode the first block eagerly so peek() works immediately
        if count > 0 {
            iter.decode_next_block();
        }

        iter
    }

    fn data(&self) -> &[u8] {
        match &self.segment.data {
            EncodedList::BlockPacked(d) => d.as_slice(),
            _ => &[],
        }
    }

    /// Decode the next block of values into block_buf.
    fn decode_next_block(&mut self) {
        self.block_buf.clear();
        self.block_idx = 0;

        // Get a raw pointer to the data to avoid holding an immutable borrow
        // on `self` while we mutate `self.byte_pos` etc.
        let data: &[u8] = match &self.segment.data {
            EncodedList::BlockPacked(d) => {
                unsafe { &*(d.as_slice() as *const [u8]) }
            }
            _ => return,
        };

        if self.byte_pos + 2 > data.len() || self.total_emitted >= self.count {
            return;
        }

        let block_count = data[self.byte_pos] as usize;
        let bit_width = data[self.byte_pos + 1];
        self.byte_pos += 2;

        let mut deltas: Vec<u64> = Vec::with_capacity(block_count);
        let bytes_read = BlockPackedEncoder::unpack_block(
            &data[self.byte_pos..], bit_width, block_count, &mut deltas,
        ).unwrap_or(0);
        self.byte_pos += bytes_read;

        // Convert deltas to absolute EntityIds
        let mut prev = self.running_prev;
        for (i, &delta) in deltas.iter().enumerate() {
            if self.total_emitted + i == 0 {
                // First element overall: use first_value directly (delta is 0)
                self.block_buf.push(EntityId::from_raw(self.first_value));
            } else {
                prev = prev.saturating_add(delta);
                self.block_buf.push(EntityId::from_raw(prev));
            }
        }

        // Update running_prev to the last value in this block
        if let Some(&last) = self.block_buf.last() {
            self.running_prev = last.as_raw();
        }
    }
}

impl Iterator for BlockPackedIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.total_emitted >= self.count {
            return None;
        }

        // If current block is exhausted, decode next
        if self.block_idx >= self.block_buf.len() {
            self.decode_next_block();
            if self.block_buf.is_empty() {
                return None;
            }
        }

        let val = self.block_buf[self.block_idx];
        self.block_idx += 1;
        self.total_emitted += 1;

        // Pre-decode next block if current is exhausted, so peek() works
        if self.block_idx >= self.block_buf.len() && self.total_emitted < self.count {
            self.decode_next_block();
        }

        Some(val)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.total_emitted);
        (remaining, Some(remaining))
    }
}

impl PostingIterator for BlockPackedIterator {
    fn peek(&self) -> Option<EntityId> {
        if self.total_emitted >= self.count {
            return None;
        }
        if self.block_idx < self.block_buf.len() {
            Some(self.block_buf[self.block_idx])
        } else {
            None
        }
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        // Scan forward through blocks until we find a value >= target.
        // Each block is decoded lazily one at a time via decode_next_block().
        loop {
            // Check remaining values in the current block
            while self.block_idx < self.block_buf.len() {
                if self.block_buf[self.block_idx] >= target {
                    return Some(self.block_buf[self.block_idx]);
                }
                self.block_idx += 1;
                self.total_emitted += 1;
            }

            if self.total_emitted >= self.count {
                return None;
            }

            // Decode next block and continue scanning
            self.decode_next_block();
            if self.block_buf.is_empty() {
                return None;
            }
        }
    }

    fn size_hint_lower(&self) -> usize {
        self.count.saturating_sub(self.total_emitted)
    }
}

// ============================================================================
// Roaring Iterator (wraps native bitmap iterator)
// ============================================================================

/// Iterator that wraps the roaring crate's native lazy iterator.
pub struct RoaringIterator {
    /// Owning reference to the segment
    _segment: Arc<Segment>,
    /// Collected values (roaring iter doesn't support peek natively)
    /// We store remaining values as a VecIterator.
    /// Note: RoaringBitmap::iter() is already lazy internally;
    /// we collect into a vec only because the roaring iter borrows
    /// the bitmap and we need ownership for the PostingIterator trait.
    inner: VecIterator,
}

impl RoaringIterator {
    pub fn new(segment: Arc<Segment>) -> Self {
        let ids = match &segment.data {
            EncodedList::Roaring(bitmap) => {
                bitmap.iter().map(|v| EntityId::from_raw(v as u64)).collect()
            }
            _ => Vec::new(),
        };
        RoaringIterator {
            _segment: segment,
            inner: VecIterator::new(ids),
        }
    }
}

impl Iterator for RoaringIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl PostingIterator for RoaringIterator {
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
// Segment Iterator (enum dispatch over encoding-specific iterators)
// ============================================================================

/// Iterator that decodes a segment on-the-fly using the appropriate
/// encoding-specific lazy iterator. Only SmallVec and Roaring
/// materialize a full Vec; DeltaVarint and BlockPacked decode lazily.
pub enum SegmentIterator {
    SmallVec(VecIterator),
    DeltaVarint(DeltaVarintIterator),
    BlockPacked(BlockPackedIterator),
    Roaring(RoaringIterator),
}

impl SegmentIterator {
    pub fn new(segment: &Arc<Segment>) -> Self {
        match &segment.data {
            EncodedList::SmallVec(ids) => {
                // SmallVec: data is already a Vec<EntityId>, just clone it
                SegmentIterator::SmallVec(VecIterator::new(ids.clone()))
            }
            EncodedList::DeltaVarint(_) => {
                SegmentIterator::DeltaVarint(DeltaVarintIterator::new(Arc::clone(segment)))
            }
            EncodedList::BlockPacked(_) => {
                SegmentIterator::BlockPacked(BlockPackedIterator::new(Arc::clone(segment)))
            }
            EncodedList::Roaring(_) => {
                SegmentIterator::Roaring(RoaringIterator::new(Arc::clone(segment)))
            }
        }
    }

    pub fn from_encoded(segment: &Arc<Segment>) -> Self {
        Self::new(segment)
    }
}

impl Iterator for SegmentIterator {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SegmentIterator::SmallVec(i) => i.next(),
            SegmentIterator::DeltaVarint(i) => i.next(),
            SegmentIterator::BlockPacked(i) => i.next(),
            SegmentIterator::Roaring(i) => i.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SegmentIterator::SmallVec(i) => i.size_hint(),
            SegmentIterator::DeltaVarint(i) => i.size_hint(),
            SegmentIterator::BlockPacked(i) => i.size_hint(),
            SegmentIterator::Roaring(i) => i.size_hint(),
        }
    }
}

impl PostingIterator for SegmentIterator {
    fn peek(&self) -> Option<EntityId> {
        match self {
            SegmentIterator::SmallVec(i) => i.peek(),
            SegmentIterator::DeltaVarint(i) => i.peek(),
            SegmentIterator::BlockPacked(i) => i.peek(),
            SegmentIterator::Roaring(i) => i.peek(),
        }
    }

    fn seek(&mut self, target: EntityId) -> Option<EntityId> {
        match self {
            SegmentIterator::SmallVec(i) => i.seek(target),
            SegmentIterator::DeltaVarint(i) => i.seek(target),
            SegmentIterator::BlockPacked(i) => i.seek(target),
            SegmentIterator::Roaring(i) => i.seek(target),
        }
    }

    fn size_hint_lower(&self) -> usize {
        match self {
            SegmentIterator::SmallVec(i) => i.size_hint_lower(),
            SegmentIterator::DeltaVarint(i) => i.size_hint_lower(),
            SegmentIterator::BlockPacked(i) => i.size_hint_lower(),
            SegmentIterator::Roaring(i) => i.size_hint_lower(),
        }
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

    // ====================================================================
    // Helpers for building segments with specific encodings
    // ====================================================================

    use crate::encoding::{
        AutoEncoder, PostingEncoder, EncodingStrategy, encode_with_strategy,
    };
    use crate::compaction::{CompactionConfig, Compactor};
    use crate::types::PostingList;

    fn make_sequential_ids(start: u64, count: usize) -> Vec<EntityId> {
        (start..start + count as u64).map(make_id).collect()
    }

    fn make_sparse_ids(start: u64, count: usize, step: u64) -> Vec<EntityId> {
        (0..count as u64).map(|i| make_id(start + i * step)).collect()
    }

    /// Build a Segment with a specific encoding and optional skip table
    fn build_segment_with_strategy(
        ids: &[EntityId],
        strategy: EncodingStrategy,
    ) -> Arc<Segment> {
        let encoded = encode_with_strategy(ids, strategy).unwrap();
        let mut skip_table = Vec::new();
        let interval = 64usize;
        for (i, &id) in ids.iter().enumerate() {
            if i % interval == 0 {
                skip_table.push((id, i as u32));
            }
        }
        Arc::new(Segment {
            first: ids[0],
            last: ids[ids.len() - 1],
            count: ids.len() as u32,
            data: encoded,
            skip_table,
        })
    }

    /// Full-decode reference for comparison
    fn full_decode(segment: &Arc<Segment>) -> Vec<EntityId> {
        AutoEncoder::decode(&segment.data).unwrap()
    }

    // ====================================================================
    // DeltaVarint lazy iterator tests
    // ====================================================================

    #[test]
    fn test_delta_varint_iter_small() {
        let ids = make_ids(&[1, 5, 10, 100, 1000]);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let iter = DeltaVarintIterator::new(Arc::clone(&seg));
        let collected: Vec<_> = iter.collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_delta_varint_iter_sequential_200() {
        let ids = make_sequential_ids(1, 200);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let iter = DeltaVarintIterator::new(Arc::clone(&seg));
        let collected: Vec<_> = iter.collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_delta_varint_iter_sequential_4000() {
        let ids = make_sequential_ids(1, 4000);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let collected: Vec<_> = DeltaVarintIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(collected.len(), 4000);
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_delta_varint_iter_sparse_1000() {
        let ids = make_sparse_ids(100, 1000, 37);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let collected: Vec<_> = DeltaVarintIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_delta_varint_iter_peek_and_next() {
        let ids = make_ids(&[10, 20, 30, 40, 50]);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));

        assert_eq!(iter.peek(), Some(make_id(10)));
        assert_eq!(iter.peek(), Some(make_id(10))); // peek is idempotent
        assert_eq!(iter.next(), Some(make_id(10)));
        assert_eq!(iter.peek(), Some(make_id(20)));
        assert_eq!(iter.next(), Some(make_id(20)));
        assert_eq!(iter.next(), Some(make_id(30)));
        assert_eq!(iter.next(), Some(make_id(40)));
        assert_eq!(iter.next(), Some(make_id(50)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.peek(), None);
    }

    #[test]
    fn test_delta_varint_iter_seek_exact() {
        let ids = make_sequential_ids(1, 500);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));

        assert_eq!(iter.seek(make_id(250)), Some(make_id(250)));
        assert_eq!(iter.next(), Some(make_id(250)));
        assert_eq!(iter.next(), Some(make_id(251)));
    }

    #[test]
    fn test_delta_varint_iter_seek_gap() {
        let ids = make_sparse_ids(10, 100, 10); // 10,20,30,...,1000
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));

        // Seek to a value between 20 and 30 should return 30
        assert_eq!(iter.seek(make_id(25)), Some(make_id(30)));
        assert_eq!(iter.next(), Some(make_id(30)));
    }

    #[test]
    fn test_delta_varint_iter_seek_past_end() {
        let ids = make_ids(&[1, 2, 3]);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));

        assert_eq!(iter.seek(make_id(999)), None);
    }

    #[test]
    fn test_delta_varint_iter_size_hint() {
        let ids = make_sequential_ids(1, 100);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));

        assert_eq!(iter.size_hint_lower(), 100);
        iter.next();
        assert_eq!(iter.size_hint_lower(), 99);
    }

    #[test]
    fn test_delta_varint_iter_matches_full_decode() {
        // Large dataset: verify lazy decode matches full decode exactly
        let ids = make_sparse_ids(42, 3000, 7);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let reference = full_decode(&seg);
        let lazy: Vec<_> = DeltaVarintIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(lazy, reference);
    }

    // ====================================================================
    // BlockPacked lazy iterator tests
    // ====================================================================

    #[test]
    fn test_block_packed_iter_500() {
        let ids = make_sequential_ids(1, 500);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let collected: Vec<_> = BlockPackedIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_block_packed_iter_5000() {
        let ids = make_sequential_ids(1, 5000);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let collected: Vec<_> = BlockPackedIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(collected.len(), 5000);
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_block_packed_iter_sparse() {
        let ids = make_sparse_ids(1, 2000, 1000);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let collected: Vec<_> = BlockPackedIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_block_packed_iter_peek_and_next() {
        let ids = make_sequential_ids(1, 300);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let mut iter = BlockPackedIterator::new(Arc::clone(&seg));

        assert_eq!(iter.peek(), Some(make_id(1)));
        assert_eq!(iter.next(), Some(make_id(1)));
        assert_eq!(iter.peek(), Some(make_id(2)));

        // Consume 127 more to cross the first block boundary (128 total)
        for _ in 0..127 {
            iter.next();
        }
        // Now at element 129 (id=129)
        assert_eq!(iter.peek(), Some(make_id(129)));
        assert_eq!(iter.next(), Some(make_id(129)));
    }

    #[test]
    fn test_block_packed_iter_seek() {
        let ids = make_sequential_ids(1, 1000);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let mut iter = BlockPackedIterator::new(Arc::clone(&seg));

        // Seek to value in second block
        assert_eq!(iter.seek(make_id(200)), Some(make_id(200)));
        assert_eq!(iter.next(), Some(make_id(200)));
        assert_eq!(iter.next(), Some(make_id(201)));
    }

    #[test]
    fn test_block_packed_iter_seek_gap() {
        let ids = make_sparse_ids(10, 500, 10);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let mut iter = BlockPackedIterator::new(Arc::clone(&seg));

        assert_eq!(iter.seek(make_id(55)), Some(make_id(60)));
        assert_eq!(iter.next(), Some(make_id(60)));
    }

    #[test]
    fn test_block_packed_iter_seek_past_end() {
        let ids = make_sequential_ids(1, 500);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let mut iter = BlockPackedIterator::new(Arc::clone(&seg));

        assert_eq!(iter.seek(make_id(99999)), None);
    }

    #[test]
    fn test_block_packed_iter_matches_full_decode() {
        let ids = make_sparse_ids(42, 5000, 13);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let reference = full_decode(&seg);
        let lazy: Vec<_> = BlockPackedIterator::new(Arc::clone(&seg)).collect();
        assert_eq!(lazy, reference);
    }

    #[test]
    fn test_block_packed_iter_crosses_many_blocks() {
        // 10 blocks worth of data (128 * 10 = 1280)
        let ids = make_sequential_ids(1, 1280);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let mut iter = BlockPackedIterator::new(Arc::clone(&seg));

        // Consume all and verify
        let mut count = 0;
        let mut prev = None;
        while let Some(id) = iter.next() {
            if let Some(p) = prev {
                assert!(id > p, "values must be strictly increasing");
            }
            prev = Some(id);
            count += 1;
        }
        assert_eq!(count, 1280);
    }

    // ====================================================================
    // SegmentIterator enum dispatch tests (via auto-selected encoding)
    // ====================================================================

    #[test]
    fn test_segment_iter_smallvec_path() {
        // < 128 elements => SmallVec
        let ids = make_ids(&[1, 5, 10, 50, 100]);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::SmallVec);
        let collected: Vec<_> = SegmentIterator::new(&seg).collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_segment_iter_delta_varint_path() {
        let ids = make_sequential_ids(1, 200);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let collected: Vec<_> = SegmentIterator::new(&seg).collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_segment_iter_block_packed_path() {
        let ids = make_sequential_ids(1, 5000);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);
        let collected: Vec<_> = SegmentIterator::new(&seg).collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_segment_iter_roaring_path() {
        let ids = make_sequential_ids(1, 1000);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::Roaring);
        let collected: Vec<_> = SegmentIterator::new(&seg).collect();
        // Roaring only stores low 32 bits, but for User type these should match
        assert_eq!(collected.len(), ids.len());
    }

    #[test]
    fn test_segment_iter_seek_all_encodings() {
        let ids = make_sequential_ids(1, 500);
        let target = make_id(250);

        for strategy in [
            EncodingStrategy::SmallVec,
            EncodingStrategy::DeltaVarint,
            EncodingStrategy::BlockPacked,
        ] {
            let seg = build_segment_with_strategy(&ids, strategy);
            let mut iter = SegmentIterator::new(&seg);
            let result = iter.seek(target);
            assert_eq!(result, Some(target), "seek failed for {:?}", strategy);
            assert_eq!(iter.next(), Some(target), "next after seek failed for {:?}", strategy);
        }
    }

    // ====================================================================
    // End-to-end: PostingList with compacted segments
    // ====================================================================

    #[test]
    fn test_posting_list_compacted_neighbors_small() {
        let ids = make_ids(&[5, 3, 1, 4, 2]);
        let pl = PostingList::from_ids_compacted(&ids, 100);
        let neighbors = pl.neighbors();
        assert_eq!(neighbors, make_ids(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_posting_list_compacted_neighbors_medium() {
        let ids = make_sequential_ids(1, 500);
        let pl = PostingList::from_ids_compacted(&ids, 100);
        let neighbors = pl.neighbors();
        assert_eq!(neighbors, ids);
    }

    #[test]
    fn test_posting_list_compacted_contains() {
        let ids = make_sequential_ids(1, 500);
        let pl = PostingList::from_ids_compacted(&ids, 100);

        // Should find every element
        for &id in &ids {
            assert!(pl.contains(id), "should contain {:?}", id);
        }
        // Should not find elements outside the range
        assert!(!pl.contains(make_id(0)));
        assert!(!pl.contains(make_id(501)));
        assert!(!pl.contains(make_id(99999)));
    }

    #[test]
    fn test_posting_list_compacted_intersect() {
        let ids_a = make_sequential_ids(1, 500);
        let ids_b = make_sequential_ids(250, 500);
        let pl_a = PostingList::from_ids_compacted(&ids_a, 100);
        let pl_b = PostingList::from_ids_compacted(&ids_b, 100);

        let result = pl_a.intersect(&pl_b);
        let expected = make_sequential_ids(250, 251); // 250..500
        assert_eq!(result, expected);
    }

    #[test]
    fn test_posting_list_compacted_large() {
        // Force DeltaVarint encoding (200-4096 elements)
        let ids = make_sequential_ids(1, 2000);
        let pl = PostingList::from_ids_compacted(&ids, 100);
        let neighbors = pl.neighbors();
        assert_eq!(neighbors.len(), 2000);
        assert_eq!(neighbors, ids);
    }

    #[test]
    fn test_posting_list_compacted_contains_sparse() {
        let ids = make_sparse_ids(1, 300, 100); // 1, 101, 201, ..., 29901
        let pl = PostingList::from_ids_compacted(&ids, 100);

        // Every element should be found
        for &id in &ids {
            assert!(pl.contains(id), "should contain {:?}", id);
        }
        // Gaps should not be found
        assert!(!pl.contains(make_id(2)));
        assert!(!pl.contains(make_id(50)));
        assert!(!pl.contains(make_id(102)));
    }

    // ====================================================================
    // Mixed: segments + buffer through posting_list_iterator
    // ====================================================================

    #[test]
    fn test_posting_list_iterator_segments_and_buffer() {
        let mut pl = PostingList::from_ids_compacted(
            &make_sequential_ids(1, 100), 100,
        );
        // Add some new elements to buffer
        pl.add(make_id(150), 200);
        pl.add(make_id(200), 201);

        let neighbors = pl.neighbors();
        assert_eq!(neighbors.len(), 102);
        assert_eq!(neighbors[0], make_id(1));
        assert_eq!(neighbors[99], make_id(100));
        assert_eq!(neighbors[100], make_id(150));
        assert_eq!(neighbors[101], make_id(200));
    }

    #[test]
    fn test_posting_list_iterator_segments_with_tombstones() {
        let mut pl = PostingList::from_ids_compacted(
            &make_sequential_ids(1, 100), 100,
        );
        // Delete some elements
        pl.delete(make_id(50), 200);
        pl.delete(make_id(75), 201);

        let neighbors = pl.neighbors();
        assert_eq!(neighbors.len(), 98);
        assert!(!neighbors.contains(&make_id(50)));
        assert!(!neighbors.contains(&make_id(75)));
    }

    // ====================================================================
    // Large dataset correctness: lazy vs reference across distributions
    // ====================================================================

    #[test]
    fn test_lazy_vs_reference_large_sequential() {
        for count in [200, 1000, 4000] {
            let ids = make_sequential_ids(1, count);
            let seg = build_segment_with_strategy(
                &ids,
                EncodingStrategy::select(&ids),
            );
            let reference = full_decode(&seg);
            let lazy: Vec<_> = SegmentIterator::new(&seg).collect();
            assert_eq!(lazy, reference, "mismatch for count={}", count);
        }
    }

    #[test]
    fn test_lazy_vs_reference_large_sparse() {
        for (count, step) in [(200, 7), (1000, 31), (4000, 97)] {
            let ids = make_sparse_ids(10, count, step);
            let seg = build_segment_with_strategy(
                &ids,
                EncodingStrategy::select(&ids),
            );
            let reference = full_decode(&seg);
            let lazy: Vec<_> = SegmentIterator::new(&seg).collect();
            assert_eq!(lazy, reference, "mismatch for count={} step={}", count, step);
        }
    }

    #[test]
    fn test_lazy_vs_reference_powers_of_two_gaps() {
        // IDs with exponentially growing gaps: 1, 2, 4, 8, 16, ...
        let ids: Vec<EntityId> = (0..20).map(|i| make_id(1u64 << i)).collect();
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);
        let reference = full_decode(&seg);
        let lazy: Vec<_> = SegmentIterator::new(&seg).collect();
        assert_eq!(lazy, reference);
    }

    #[test]
    fn test_seek_every_element_delta_varint() {
        let ids = make_sequential_ids(1, 500);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);

        // Seek to every single element and verify
        for &expected in &ids {
            let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));
            let found = iter.seek(expected);
            assert_eq!(found, Some(expected), "seek failed for {:?}", expected);
        }
    }

    #[test]
    fn test_seek_every_element_block_packed() {
        let ids = make_sequential_ids(1, 500);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);

        for &expected in &ids {
            let mut iter = BlockPackedIterator::new(Arc::clone(&seg));
            let found = iter.seek(expected);
            assert_eq!(found, Some(expected), "seek failed for {:?}", expected);
        }
    }

    #[test]
    fn test_seek_gaps_delta_varint() {
        let ids = make_sparse_ids(10, 200, 10); // 10,20,...,2000
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::DeltaVarint);

        // Seek to values that fall in gaps
        let mut iter = DeltaVarintIterator::new(Arc::clone(&seg));
        assert_eq!(iter.seek(make_id(15)), Some(make_id(20)));

        let mut iter2 = DeltaVarintIterator::new(Arc::clone(&seg));
        assert_eq!(iter2.seek(make_id(1)), Some(make_id(10)));

        let mut iter3 = DeltaVarintIterator::new(Arc::clone(&seg));
        assert_eq!(iter3.seek(make_id(2001)), None);
    }

    #[test]
    fn test_seek_gaps_block_packed() {
        let ids = make_sparse_ids(10, 500, 10);
        let seg = build_segment_with_strategy(&ids, EncodingStrategy::BlockPacked);

        let mut iter = BlockPackedIterator::new(Arc::clone(&seg));
        assert_eq!(iter.seek(make_id(15)), Some(make_id(20)));

        let mut iter2 = BlockPackedIterator::new(Arc::clone(&seg));
        assert_eq!(iter2.seek(make_id(1)), Some(make_id(10)));
    }

    #[test]
    fn test_empty_segment_all_encodings() {
        for strategy in [
            EncodingStrategy::SmallVec,
            EncodingStrategy::DeltaVarint,
            EncodingStrategy::BlockPacked,
        ] {
            let ids: Vec<EntityId> = vec![];
            let encoded = encode_with_strategy(&ids, strategy).unwrap();
            let seg = Arc::new(Segment {
                first: EntityId::null(),
                last: EntityId::null(),
                count: 0,
                data: encoded,
                skip_table: Vec::new(),
            });
            let collected: Vec<_> = SegmentIterator::new(&seg).collect();
            assert!(collected.is_empty(), "empty segment for {:?}", strategy);
        }
    }

    #[test]
    fn test_single_element_all_encodings() {
        let ids = make_ids(&[42]);
        for strategy in [
            EncodingStrategy::SmallVec,
            EncodingStrategy::DeltaVarint,
            EncodingStrategy::BlockPacked,
        ] {
            let seg = build_segment_with_strategy(&ids, strategy);
            let collected: Vec<_> = SegmentIterator::new(&seg).collect();
            assert_eq!(collected, ids, "single element for {:?}", strategy);

            let mut iter = SegmentIterator::new(&seg);
            assert_eq!(iter.seek(make_id(42)), Some(make_id(42)));
        }
    }
}

//! Compaction: converting write buffer to immutable segments.
//!
//! When the write buffer exceeds a threshold, we compact it into a new segment.
//! This includes sorting, deduplication, tombstone application, and encoding.

use crate::encoding::{AutoEncoder, EncodedList, EncodingStrategy, PostingEncoder, encode_with_strategy};
use crate::id::EntityId;
use crate::types::{BufferEntry, PostingList, Segment, WriteBuffer};
use std::sync::Arc;
use std::collections::HashSet;

/// Configuration for compaction behavior
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompactionConfig {
    /// Trigger compaction when buffer exceeds this many entries
    pub buffer_threshold: usize,
    /// Maximum number of segments before merging
    pub max_segments: usize,
    /// Target size for segments (in entries)
    pub segment_target_size: usize,
    /// Force a specific encoding strategy (None = auto-select)
    pub encoding_strategy: Option<EncodingStrategy>,
    /// Enable skip table generation
    pub generate_skip_table: bool,
    /// Skip table interval (entries between skip points)
    pub skip_interval: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        CompactionConfig {
            buffer_threshold: 256,
            max_segments: 8,
            segment_target_size: 4096,
            encoding_strategy: None,
            generate_skip_table: true,
            skip_interval: 64,
        }
    }
}

impl CompactionConfig {
    pub fn with_buffer_threshold(mut self, threshold: usize) -> Self {
        self.buffer_threshold = threshold;
        self
    }

    pub fn with_max_segments(mut self, max: usize) -> Self {
        self.max_segments = max;
        self
    }

    pub fn with_encoding(mut self, strategy: EncodingStrategy) -> Self {
        self.encoding_strategy = Some(strategy);
        self
    }
}

/// Compaction statistics
#[derive(Debug, Default, Clone)]
pub struct CompactionStats {
    pub entries_processed: usize,
    pub tombstones_applied: usize,
    pub duplicates_removed: usize,
    pub segments_created: usize,
    pub segments_merged: usize,
    pub bytes_written: usize,
}

/// Compactor for a single posting list
pub struct Compactor {
    config: CompactionConfig,
}

impl Compactor {
    pub fn new(config: CompactionConfig) -> Self {
        Compactor { config }
    }

    pub fn with_default_config() -> Self {
        Compactor {
            config: CompactionConfig::default(),
        }
    }

    /// Check if a posting list needs compaction
    pub fn needs_compaction(&self, pl: &PostingList) -> bool {
        pl.buffer.len() > self.config.buffer_threshold
    }

    /// Check if segments need merging
    pub fn needs_merge(&self, pl: &PostingList) -> bool {
        pl.segments.len() > self.config.max_segments
    }

    /// Compact the write buffer into a new segment.
    /// 
    /// Returns the new segment and statistics.
    /// The caller is responsible for swapping the buffer and updating segments.
    pub fn compact_buffer(&self, buffer: &WriteBuffer) -> (Option<Segment>, CompactionStats) {
        let mut stats = CompactionStats::default();
        
        let entries = buffer.entries().to_vec();
        if entries.is_empty() {
            return (None, stats);
        }
        
        stats.entries_processed = entries.len();
        
        // Separate adds and tombstones
        let mut adds: Vec<(EntityId, u64)> = Vec::new();
        let mut tombstones: HashSet<EntityId> = HashSet::new();
        
        for entry in entries {
            if entry.tombstone {
                tombstones.insert(entry.dst);
            } else {
                adds.push((entry.dst, entry.timestamp));
            }
        }
        
        stats.tombstones_applied = tombstones.len();
        
        // Sort by ID, then by timestamp (newer wins)
        adds.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
        
        // Deduplicate and filter tombstones
        let mut deduped: Vec<EntityId> = Vec::new();
        let mut last_id: Option<EntityId> = None;
        
        for (id, _ts) in adds {
            // Skip tombstoned
            if tombstones.contains(&id) {
                continue;
            }
            
            // Skip duplicates
            if last_id == Some(id) {
                stats.duplicates_removed += 1;
                continue;
            }
            
            deduped.push(id);
            last_id = Some(id);
        }
        
        if deduped.is_empty() {
            return (None, stats);
        }
        
        // Encode
        let encoded = match self.config.encoding_strategy {
            Some(strategy) => encode_with_strategy(&deduped, strategy),
            None => AutoEncoder::encode(&deduped),
        };
        
        let encoded = match encoded {
            Ok(e) => e,
            Err(_) => return (None, stats),
        };
        
        stats.bytes_written = encoded.size_bytes();
        stats.segments_created = 1;
        
        // Build skip table
        let skip_table = if self.config.generate_skip_table {
            self.build_skip_table(&deduped)
        } else {
            Vec::new()
        };
        
        let segment = Segment {
            data: encoded,
            first: deduped[0],
            last: deduped[deduped.len() - 1],
            count: deduped.len() as u32,
            skip_table,
        };
        
        (Some(segment), stats)
    }

    /// Merge multiple segments into fewer, larger segments.
    pub fn merge_segments(&self, segments: &[Arc<Segment>]) -> (Vec<Segment>, CompactionStats) {
        let mut stats = CompactionStats::default();
        
        if segments.is_empty() {
            return (Vec::new(), stats);
        }
        
        if segments.len() == 1 {
            // Nothing to merge
            return (vec![(*segments[0]).clone()], stats);
        }
        
        // Decode all segments and merge
        let mut all_ids: Vec<EntityId> = Vec::new();
        
        for segment in segments {
            let decoded = AutoEncoder::decode(&segment.data).unwrap_or_default();
            stats.entries_processed += decoded.len();
            all_ids.extend(decoded);
        }
        
        // Sort and deduplicate
        all_ids.sort();
        let original_len = all_ids.len();
        all_ids.dedup();
        stats.duplicates_removed = original_len - all_ids.len();
        
        if all_ids.is_empty() {
            return (Vec::new(), stats);
        }
        
        // Split into target-sized segments
        let mut new_segments = Vec::new();
        
        for chunk in all_ids.chunks(self.config.segment_target_size) {
            let encoded = match self.config.encoding_strategy {
                Some(strategy) => encode_with_strategy(chunk, strategy),
                None => AutoEncoder::encode(chunk),
            };
            
            let encoded = match encoded {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            stats.bytes_written += encoded.size_bytes();
            
            let skip_table = if self.config.generate_skip_table {
                self.build_skip_table(chunk)
            } else {
                Vec::new()
            };
            
            new_segments.push(Segment {
                data: encoded,
                first: chunk[0],
                last: chunk[chunk.len() - 1],
                count: chunk.len() as u32,
                skip_table,
            });
        }
        
        stats.segments_created = new_segments.len();
        stats.segments_merged = segments.len();
        
        (new_segments, stats)
    }

    /// Apply tombstones from buffer to existing segments.
    /// 
    /// This creates new segments with tombstoned IDs removed.
    pub fn apply_tombstones(
        &self,
        segments: &[Arc<Segment>],
        tombstones: &[EntityId],
    ) -> (Vec<Segment>, CompactionStats) {
        let mut stats = CompactionStats::default();
        
        if tombstones.is_empty() {
            // Nothing to do
            return (segments.iter().map(|s| (**s).clone()).collect(), stats);
        }
        
        let tombstone_set: HashSet<EntityId> = tombstones.iter().copied().collect();
        stats.tombstones_applied = tombstone_set.len();
        
        let mut new_segments = Vec::new();
        
        for segment in segments {
            // Check if any tombstones might be in this segment
            let might_have_tombstones = tombstones.iter().any(|t| segment.might_contain(*t));
            
            if !might_have_tombstones {
                new_segments.push((**segment).clone());
                continue;
            }
            
            // Decode and filter
            let decoded = AutoEncoder::decode(&segment.data).unwrap_or_default();
            stats.entries_processed += decoded.len();
            
            let filtered: Vec<EntityId> = decoded
                .into_iter()
                .filter(|id| !tombstone_set.contains(id))
                .collect();
            
            if filtered.is_empty() {
                continue;
            }
            
            // Re-encode
            let encoded = match self.config.encoding_strategy {
                Some(strategy) => encode_with_strategy(&filtered, strategy),
                None => AutoEncoder::encode(&filtered),
            };
            
            let encoded = match encoded {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            stats.bytes_written += encoded.size_bytes();
            
            let skip_table = if self.config.generate_skip_table {
                self.build_skip_table(&filtered)
            } else {
                Vec::new()
            };
            
            new_segments.push(Segment {
                data: encoded,
                first: filtered[0],
                last: filtered[filtered.len() - 1],
                count: filtered.len() as u32,
                skip_table,
            });
        }
        
        stats.segments_created = new_segments.len();
        (new_segments, stats)
    }

    /// Build a skip table for efficient seeking
    fn build_skip_table(&self, ids: &[EntityId]) -> Vec<(EntityId, u32)> {
        let mut skip_table = Vec::new();
        let interval = self.config.skip_interval as usize;
        
        for (i, &id) in ids.iter().enumerate() {
            if i % interval == 0 {
                skip_table.push((id, i as u32));
            }
        }
        
        skip_table
    }

    /// Full compaction: compact buffer and merge if needed.
    /// 
    /// Returns new segments and the IDs that should be removed from buffer.
    pub fn full_compact(
        &self,
        pl: &PostingList,
    ) -> (Vec<Arc<Segment>>, Vec<BufferEntry>, CompactionStats) {
        let mut total_stats = CompactionStats::default();
        
        // First, compact the buffer
        let (new_segment, buffer_stats) = self.compact_buffer(&pl.buffer);
        total_stats.entries_processed += buffer_stats.entries_processed;
        total_stats.tombstones_applied += buffer_stats.tombstones_applied;
        total_stats.duplicates_removed += buffer_stats.duplicates_removed;
        total_stats.bytes_written += buffer_stats.bytes_written;
        
        // Collect all segments
        let mut all_segments: Vec<Arc<Segment>> = pl.segments.clone();
        if let Some(seg) = new_segment {
            all_segments.push(Arc::new(seg));
            total_stats.segments_created += 1;
        }
        
        // Merge if too many segments
        if all_segments.len() > self.config.max_segments {
            let (merged, merge_stats) = self.merge_segments(&all_segments);
            total_stats.segments_merged = merge_stats.segments_merged;
            total_stats.segments_created = merged.len();
            total_stats.bytes_written += merge_stats.bytes_written;
            
            all_segments = merged.into_iter().map(Arc::new).collect();
        }
        
        // Return the buffer entries to be cleared
        let buffer_entries = pl.buffer.entries().to_vec();
        
        (all_segments, buffer_entries, total_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityType;

    fn make_id(raw: u64) -> EntityId {
        EntityId::new(EntityType::User, raw)
    }

    #[test]
    fn test_compact_empty_buffer() {
        let compactor = Compactor::with_default_config();
        let buffer = WriteBuffer::new();
        
        let (segment, stats) = compactor.compact_buffer(&buffer);
        
        assert!(segment.is_none());
        assert_eq!(stats.entries_processed, 0);
    }

    #[test]
    fn test_compact_basic() {
        let compactor = Compactor::with_default_config();
        let mut buffer = WriteBuffer::new();
        
        buffer.push(BufferEntry::add(make_id(3), 100));
        buffer.push(BufferEntry::add(make_id(1), 101));
        buffer.push(BufferEntry::add(make_id(2), 102));
        
        let (segment, stats) = compactor.compact_buffer(&buffer);
        
        assert!(segment.is_some());
        let seg = segment.unwrap();
        assert_eq!(seg.count, 3);
        assert_eq!(seg.first, make_id(1));
        assert_eq!(seg.last, make_id(3));
        assert_eq!(stats.entries_processed, 3);
    }

    #[test]
    fn test_compact_with_duplicates() {
        let compactor = Compactor::with_default_config();
        let mut buffer = WriteBuffer::new();
        
        buffer.push(BufferEntry::add(make_id(1), 100));
        buffer.push(BufferEntry::add(make_id(1), 101)); // Duplicate
        buffer.push(BufferEntry::add(make_id(2), 102));
        
        let (segment, stats) = compactor.compact_buffer(&buffer);
        
        assert!(segment.is_some());
        let seg = segment.unwrap();
        assert_eq!(seg.count, 2);
        assert_eq!(stats.duplicates_removed, 1);
    }

    #[test]
    fn test_compact_with_tombstones() {
        let compactor = Compactor::with_default_config();
        let mut buffer = WriteBuffer::new();
        
        buffer.push(BufferEntry::add(make_id(1), 100));
        buffer.push(BufferEntry::add(make_id(2), 101));
        buffer.push(BufferEntry::add(make_id(3), 102));
        buffer.push(BufferEntry::delete(make_id(2), 103)); // Tombstone
        
        let (segment, stats) = compactor.compact_buffer(&buffer);
        
        assert!(segment.is_some());
        let seg = segment.unwrap();
        assert_eq!(seg.count, 2); // Only 1 and 3
        assert_eq!(stats.tombstones_applied, 1);
    }

    #[test]
    fn test_compact_all_tombstoned() {
        let compactor = Compactor::with_default_config();
        let mut buffer = WriteBuffer::new();
        
        buffer.push(BufferEntry::add(make_id(1), 100));
        buffer.push(BufferEntry::delete(make_id(1), 101));
        
        let (segment, _stats) = compactor.compact_buffer(&buffer);
        
        assert!(segment.is_none());
    }

    #[test]
    fn test_merge_segments() {
        let compactor = Compactor::new(CompactionConfig {
            segment_target_size: 100,
            ..Default::default()
        });
        
        // Create two segments
        let ids1: Vec<EntityId> = (1..50).map(make_id).collect();
        let ids2: Vec<EntityId> = (50..100).map(make_id).collect();
        
        let seg1 = Arc::new(Segment {
            data: AutoEncoder::encode(&ids1).unwrap(),
            first: ids1[0],
            last: ids1[ids1.len() - 1],
            count: ids1.len() as u32,
            skip_table: Vec::new(),
        });
        
        let seg2 = Arc::new(Segment {
            data: AutoEncoder::encode(&ids2).unwrap(),
            first: ids2[0],
            last: ids2[ids2.len() - 1],
            count: ids2.len() as u32,
            skip_table: Vec::new(),
        });
        
        let (merged, stats) = compactor.merge_segments(&[seg1, seg2]);
        
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].count, 99);
        assert_eq!(stats.segments_merged, 2);
    }

    #[test]
    fn test_needs_compaction() {
        let config = CompactionConfig {
            buffer_threshold: 5,
            ..Default::default()
        };
        let compactor = Compactor::new(config);
        
        let mut pl = PostingList::new();
        assert!(!compactor.needs_compaction(&pl));
        
        for i in 0..6 {
            pl.buffer.push(BufferEntry::add(make_id(i), 100));
        }
        
        assert!(compactor.needs_compaction(&pl));
    }

    #[test]
    fn test_skip_table_generation() {
        let config = CompactionConfig {
            skip_interval: 4,
            generate_skip_table: true,
            ..Default::default()
        };
        let compactor = Compactor::new(config);
        
        let mut buffer = WriteBuffer::new();
        for i in 0..10 {
            buffer.push(BufferEntry::add(make_id(i), 100));
        }
        
        let (segment, _) = compactor.compact_buffer(&buffer);
        let seg = segment.unwrap();
        
        // Skip table should have entries at 0, 4, 8
        assert_eq!(seg.skip_table.len(), 3);
        assert_eq!(seg.skip_table[0], (make_id(0), 0));
        assert_eq!(seg.skip_table[1], (make_id(4), 4));
        assert_eq!(seg.skip_table[2], (make_id(8), 8));
    }
}

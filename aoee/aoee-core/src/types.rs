//! Core types for AOEE: edges, buffers, segments, and posting lists.

use crate::encoding::EncodedList;
use crate::id::EntityId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Edge types representing relationships between entities.
///
/// These define the semantic meaning of connections in the graph.
/// Extensible to support various relationship types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum EdgeType {
    // Social relationships
    /// User follows another user
    Follows = 0,
    /// User is followed by another user (reverse of Follows)
    FollowedBy = 1,
    /// User is friends with another user (bidirectional)
    FriendOf = 2,
    /// User has blocked another user
    Blocks = 3,
    /// User is blocked by another user (reverse)
    BlockedBy = 4,

    // Content interactions
    /// User/entity likes something
    Likes = 10,
    /// Entity is liked by users (reverse)
    LikedBy = 11,
    /// User/entity comments on something
    CommentsOn = 12,
    /// Entity has comments (reverse)
    HasComment = 13,
    /// User shares/reposts content
    Shares = 14,
    /// Content is shared by users (reverse)
    SharedBy = 15,

    // Authorship
    /// User authored/created content
    Authored = 20,
    /// Content is authored by user (reverse)
    AuthoredBy = 21,

    // Group/Page membership
    /// User is member of group/page
    MemberOf = 30,
    /// Group/page has member (reverse)
    HasMember = 31,
    /// User administers group/page
    Administers = 32,
    /// Group/page is administered by user (reverse)
    AdministeredBy = 33,

    // Content containment
    /// Entity contains another entity (e.g., album contains photos)
    Contains = 40,
    /// Entity is contained in another (reverse)
    ContainedIn = 41,

    // Tagging
    /// Entity is tagged in content
    TaggedIn = 50,
    /// Content has tag (reverse)
    HasTag = 51,

    // Mentions
    /// Entity mentions another
    Mentions = 60,
    /// Entity is mentioned by (reverse)
    MentionedBy = 61,

    // Custom edge types for application-specific use
    Custom1 = 100,
    Custom2 = 101,
    Custom3 = 102,
    Custom4 = 103,
    Custom5 = 104,
}

impl EdgeType {
    /// Get the reverse edge type if one exists.
    ///
    /// Returns None for edge types that don't have a defined reverse.
    pub fn reverse(self) -> Option<EdgeType> {
        match self {
            EdgeType::Follows => Some(EdgeType::FollowedBy),
            EdgeType::FollowedBy => Some(EdgeType::Follows),
            EdgeType::FriendOf => Some(EdgeType::FriendOf), // Symmetric
            EdgeType::Blocks => Some(EdgeType::BlockedBy),
            EdgeType::BlockedBy => Some(EdgeType::Blocks),
            EdgeType::Likes => Some(EdgeType::LikedBy),
            EdgeType::LikedBy => Some(EdgeType::Likes),
            EdgeType::CommentsOn => Some(EdgeType::HasComment),
            EdgeType::HasComment => Some(EdgeType::CommentsOn),
            EdgeType::Shares => Some(EdgeType::SharedBy),
            EdgeType::SharedBy => Some(EdgeType::Shares),
            EdgeType::Authored => Some(EdgeType::AuthoredBy),
            EdgeType::AuthoredBy => Some(EdgeType::Authored),
            EdgeType::MemberOf => Some(EdgeType::HasMember),
            EdgeType::HasMember => Some(EdgeType::MemberOf),
            EdgeType::Administers => Some(EdgeType::AdministeredBy),
            EdgeType::AdministeredBy => Some(EdgeType::Administers),
            EdgeType::Contains => Some(EdgeType::ContainedIn),
            EdgeType::ContainedIn => Some(EdgeType::Contains),
            EdgeType::TaggedIn => Some(EdgeType::HasTag),
            EdgeType::HasTag => Some(EdgeType::TaggedIn),
            EdgeType::Mentions => Some(EdgeType::MentionedBy),
            EdgeType::MentionedBy => Some(EdgeType::Mentions),
            _ => None,
        }
    }

    /// Check if this edge type is symmetric (same as its reverse).
    pub fn is_symmetric(self) -> bool {
        matches!(self, EdgeType::FriendOf)
    }

    /// Get the raw u16 value.
    pub fn as_raw(self) -> u16 {
        self as u16
    }

    /// Create from raw u16 value.
    pub fn from_raw(value: u16) -> Option<Self> {
        match value {
            0 => Some(EdgeType::Follows),
            1 => Some(EdgeType::FollowedBy),
            2 => Some(EdgeType::FriendOf),
            3 => Some(EdgeType::Blocks),
            4 => Some(EdgeType::BlockedBy),
            10 => Some(EdgeType::Likes),
            11 => Some(EdgeType::LikedBy),
            12 => Some(EdgeType::CommentsOn),
            13 => Some(EdgeType::HasComment),
            14 => Some(EdgeType::Shares),
            15 => Some(EdgeType::SharedBy),
            20 => Some(EdgeType::Authored),
            21 => Some(EdgeType::AuthoredBy),
            30 => Some(EdgeType::MemberOf),
            31 => Some(EdgeType::HasMember),
            32 => Some(EdgeType::Administers),
            33 => Some(EdgeType::AdministeredBy),
            40 => Some(EdgeType::Contains),
            41 => Some(EdgeType::ContainedIn),
            50 => Some(EdgeType::TaggedIn),
            51 => Some(EdgeType::HasTag),
            60 => Some(EdgeType::Mentions),
            61 => Some(EdgeType::MentionedBy),
            100 => Some(EdgeType::Custom1),
            101 => Some(EdgeType::Custom2),
            102 => Some(EdgeType::Custom3),
            103 => Some(EdgeType::Custom4),
            104 => Some(EdgeType::Custom5),
            _ => None,
        }
    }
}

/// Key for looking up a posting list: (source entity, edge type).
///
/// This uniquely identifies an adjacency list in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeKey {
    /// Source entity ID
    pub src: EntityId,
    /// Type of edge/relationship
    pub edge_type: EdgeType,
}

impl EdgeKey {
    /// Create a new EdgeKey.
    #[inline]
    pub fn new(src: EntityId, edge_type: EdgeType) -> Self {
        EdgeKey { src, edge_type }
    }

    /// Get the reverse key (for reverse edge lookups).
    ///
    /// Note: This only changes the edge type, not the source.
    /// For the actual reverse edge, you need the destination entity as source.
    pub fn with_reverse_type(&self) -> Option<EdgeKey> {
        self.edge_type.reverse().map(|rev| EdgeKey {
            src: self.src,
            edge_type: rev,
        })
    }
}

/// Edge types that support metadata (1 byte per edge).
/// For LIKES: 0=like, 1=love, 2=haha, 3=wow, 4=sad, 5=angry
pub const METADATA_EDGE_TYPES: &[EdgeType] = &[EdgeType::Likes];

/// Check if an edge type supports metadata.
#[inline]
pub fn edge_type_supports_metadata(edge_type: EdgeType) -> bool {
    METADATA_EDGE_TYPES.contains(&edge_type)
}

/// Reaction types for LIKES edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ReactionType {
    #[default]
    Like = 0,
    Love = 1,
    Haha = 2,
    Wow = 3,
    Sad = 4,
    Angry = 5,
}

impl ReactionType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => ReactionType::Love,
            2 => ReactionType::Haha,
            3 => ReactionType::Wow,
            4 => ReactionType::Sad,
            5 => ReactionType::Angry,
            _ => ReactionType::Like,
        }
    }
}

/// An entry in the write buffer, representing a pending edge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferEntry {
    /// Destination entity ID
    pub dst: EntityId,
    /// Whether this is a deletion (tombstone)
    pub tombstone: bool,
    /// Timestamp of the operation (for ordering and TTL)
    pub timestamp: u64,
    /// Optional metadata byte (meaning depends on edge type)
    pub metadata: u8,
}

impl BufferEntry {
    /// Create a new add entry.
    #[inline]
    pub fn add(dst: EntityId, timestamp: u64) -> Self {
        BufferEntry {
            dst,
            tombstone: false,
            timestamp,
            metadata: 0,
        }
    }

    /// Create a new add entry with metadata.
    #[inline]
    pub fn add_with_metadata(dst: EntityId, timestamp: u64, metadata: u8) -> Self {
        BufferEntry {
            dst,
            tombstone: false,
            timestamp,
            metadata,
        }
    }

    /// Create a new delete (tombstone) entry.
    #[inline]
    pub fn delete(dst: EntityId, timestamp: u64) -> Self {
        BufferEntry {
            dst,
            tombstone: true,
            timestamp,
            metadata: 0,
        }
    }
}

/// Append-only write buffer for pending edge operations.
///
/// New edges are appended here before being compacted into immutable segments.
#[derive(Debug, Clone, Default)]
pub struct WriteBuffer {
    entries: Vec<BufferEntry>,
}

impl WriteBuffer {
    /// Create a new empty write buffer.
    pub fn new() -> Self {
        WriteBuffer {
            entries: Vec::new(),
        }
    }

    /// Create a write buffer with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        WriteBuffer {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Append an entry to the buffer.
    #[inline]
    pub fn push(&mut self, entry: BufferEntry) {
        self.entries.push(entry);
    }

    /// Get the number of entries in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get a snapshot of entries for reading.
    ///
    /// The returned entries are sorted by destination ID and deduplicated.
    pub fn snapshot(&self) -> Vec<BufferEntry> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let mut entries = self.entries.clone();
        // Sort by dst, then by timestamp (newer wins)
        entries.sort_by(|a, b| {
            a.dst
                .cmp(&b.dst)
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });

        // Deduplicate: keep the newest entry for each dst
        entries.dedup_by(|a, b| a.dst == b.dst);
        entries
    }

    /// Take all entries and clear the buffer.
    ///
    /// Used during compaction to atomically swap out the buffer.
    pub fn take(&mut self) -> Vec<BufferEntry> {
        std::mem::take(&mut self.entries)
    }

    /// Get raw entries (unsorted) for iteration.
    pub fn entries(&self) -> &[BufferEntry] {
        &self.entries
    }
}

/// An immutable compressed segment containing sorted destination IDs.
///
/// Segments are created during compaction and never modified afterward.
#[derive(Debug, Clone)]
pub struct Segment {
    /// Compressed payload (encoding depends on the strategy used)
    pub data: EncodedList,
    /// First (minimum) ID in this segment
    pub first: EntityId,
    /// Last (maximum) ID in this segment
    pub last: EntityId,
    /// Number of IDs in this segment
    pub count: u32,
    /// Skip table for fast seeking: (value, offset in decoded sequence)
    /// Every N elements, we store the value for binary search
    pub skip_table: Vec<(EntityId, u32)>,
}

impl Segment {
    /// Check if this segment might contain the given ID.
    #[inline]
    pub fn might_contain(&self, id: EntityId) -> bool {
        id >= self.first && id <= self.last
    }

    /// Get the skip interval used for the skip table.
    pub const SKIP_INTERVAL: u32 = 64;
}

/// A posting list representing all outgoing edges of a given type from a source entity.
///
/// Structure:
/// - `buffer`: Mutable write buffer for recent changes (append-only)
/// - `segments`: Immutable compressed segments (read-optimized)
///
/// This is analogous to LSM-tree's memtable + SSTables structure.
#[derive(Debug)]
pub struct PostingList {
    /// Write buffer for pending operations
    pub buffer: WriteBuffer,
    /// Immutable segments (sorted by first ID)
    pub segments: Vec<Arc<Segment>>,
    /// Total count (approximate, updated during compaction)
    pub total_count: u64,
    /// Timestamp of last modification
    pub last_modified: u64,
}

impl PostingList {
    /// Create a new empty posting list.
    pub fn new() -> Self {
        PostingList {
            buffer: WriteBuffer::new(),
            segments: Vec::new(),
            total_count: 0,
            last_modified: 0,
        }
    }

    /// Check if the posting list is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty() && self.segments.is_empty()
    }

    /// Get the buffer length (for compaction triggering).
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Get the number of segments.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

impl Default for PostingList {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe posting list wrapped in a read-write lock.
pub type SharedPostingList = Arc<RwLock<PostingList>>;

/// Create a new shared posting list.
pub fn new_shared_posting_list() -> SharedPostingList {
    Arc::new(RwLock::new(PostingList::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityType;

    #[test]
    fn test_edge_type_reverse() {
        assert_eq!(EdgeType::Follows.reverse(), Some(EdgeType::FollowedBy));
        assert_eq!(EdgeType::FollowedBy.reverse(), Some(EdgeType::Follows));
        assert_eq!(EdgeType::FriendOf.reverse(), Some(EdgeType::FriendOf));
        assert_eq!(EdgeType::Likes.reverse(), Some(EdgeType::LikedBy));
    }

    #[test]
    fn test_edge_key() {
        let user = EntityId::new(EntityType::User, 42);
        let key = EdgeKey::new(user, EdgeType::Follows);

        assert_eq!(key.src, user);
        assert_eq!(key.edge_type, EdgeType::Follows);

        let rev = key.with_reverse_type().unwrap();
        assert_eq!(rev.edge_type, EdgeType::FollowedBy);
    }

    #[test]
    fn test_buffer_entry() {
        let dst = EntityId::new(EntityType::User, 100);
        let add = BufferEntry::add(dst, 1000);
        let del = BufferEntry::delete(dst, 1001);

        assert!(!add.tombstone);
        assert!(del.tombstone);
        assert_eq!(add.dst, dst);
    }

    #[test]
    fn test_write_buffer_snapshot() {
        let mut buffer = WriteBuffer::new();

        let id1 = EntityId::new(EntityType::User, 3);
        let id2 = EntityId::new(EntityType::User, 1);
        let id3 = EntityId::new(EntityType::User, 2);

        buffer.push(BufferEntry::add(id1, 100));
        buffer.push(BufferEntry::add(id2, 101));
        buffer.push(BufferEntry::add(id3, 102));
        // Add duplicate with newer timestamp
        buffer.push(BufferEntry::delete(id1, 103));

        let snapshot = buffer.snapshot();

        // Should be sorted by dst
        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot[0].dst, id2); // id=1
        assert_eq!(snapshot[1].dst, id3); // id=2
        assert_eq!(snapshot[2].dst, id1); // id=3

        // The duplicate should be the newer one (tombstone)
        assert!(snapshot[2].tombstone);
    }

    #[test]
    fn test_write_buffer_take() {
        let mut buffer = WriteBuffer::new();
        let id = EntityId::new(EntityType::User, 1);
        buffer.push(BufferEntry::add(id, 100));
        buffer.push(BufferEntry::add(id, 101));

        assert_eq!(buffer.len(), 2);

        let taken = buffer.take();
        assert_eq!(taken.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_posting_list() {
        let mut pl = PostingList::new();
        assert!(pl.is_empty());

        let id = EntityId::new(EntityType::User, 1);
        pl.buffer.push(BufferEntry::add(id, 100));

        assert!(!pl.is_empty());
        assert_eq!(pl.buffer_len(), 1);
        assert_eq!(pl.segment_count(), 0);
    }
}

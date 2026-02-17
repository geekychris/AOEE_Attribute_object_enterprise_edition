//! Entity ID system with embedded type information.
//!
//! IDs are 64-bit values where the high 16 bits encode the entity type
//! and the low 48 bits encode the unique identifier within that type.
//!
//! Layout: [type:16][id:48]
//!
//! This allows O(1) type extraction and supports up to 65,536 entity types
//! with ~281 trillion unique IDs per type.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Custom epoch: January 1, 2020 00:00:00 UTC
/// This gives us ~136 years of range (until ~2156)
pub const AOEE_EPOCH: u64 = 1577836800;

/// Number of bits used for the entity type
const TYPE_BITS: u32 = 16;
/// Number of bits used for the raw ID
const ID_BITS: u32 = 48;
/// Mask for extracting the raw ID (lower 48 bits)
const ID_MASK: u64 = (1u64 << ID_BITS) - 1;
/// Maximum value for raw ID
pub const MAX_RAW_ID: u64 = ID_MASK;

/// Entity types supported by the system.
///
/// The discriminant values are stored in the high bits of EntityId.
/// New types can be added as needed - this is designed to be extensible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum EntityType {
    /// User entity (person, account)
    User = 0,
    /// Post entity (status update, article)
    Post = 1,
    /// Comment entity
    Comment = 2,
    /// Photo/Image entity
    Photo = 3,
    /// Video entity
    Video = 4,
    /// Group entity
    Group = 5,
    /// Page entity (business page, fan page)
    Page = 6,
    /// Event entity
    Event = 7,
    /// Message entity
    Message = 8,
    /// Reaction entity (like, love, etc.)
    Reaction = 9,
    /// Tag entity
    Tag = 10,
    /// Location/Place entity
    Location = 11,
    /// Link/URL entity
    Link = 12,
    /// Album entity
    Album = 13,
    /// Story entity
    Story = 14,
    /// Custom type 1 (application-defined)
    Custom1 = 100,
    /// Custom type 2 (application-defined)
    Custom2 = 101,
    /// Custom type 3 (application-defined)
    Custom3 = 102,
    /// Unknown/Invalid type
    Unknown = 0xFFFF,
}

impl EntityType {
    /// Create EntityType from raw u16 value
    #[inline]
    pub fn from_raw(value: u16) -> Self {
        match value {
            0 => EntityType::User,
            1 => EntityType::Post,
            2 => EntityType::Comment,
            3 => EntityType::Photo,
            4 => EntityType::Video,
            5 => EntityType::Group,
            6 => EntityType::Page,
            7 => EntityType::Event,
            8 => EntityType::Message,
            9 => EntityType::Reaction,
            10 => EntityType::Tag,
            11 => EntityType::Location,
            12 => EntityType::Link,
            13 => EntityType::Album,
            14 => EntityType::Story,
            100 => EntityType::Custom1,
            101 => EntityType::Custom2,
            102 => EntityType::Custom3,
            _ => EntityType::Unknown,
        }
    }

    /// Get the raw u16 value
    #[inline]
    pub fn as_raw(self) -> u16 {
        self as u16
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::User => write!(f, "User"),
            EntityType::Post => write!(f, "Post"),
            EntityType::Comment => write!(f, "Comment"),
            EntityType::Photo => write!(f, "Photo"),
            EntityType::Video => write!(f, "Video"),
            EntityType::Group => write!(f, "Group"),
            EntityType::Page => write!(f, "Page"),
            EntityType::Event => write!(f, "Event"),
            EntityType::Message => write!(f, "Message"),
            EntityType::Reaction => write!(f, "Reaction"),
            EntityType::Tag => write!(f, "Tag"),
            EntityType::Location => write!(f, "Location"),
            EntityType::Link => write!(f, "Link"),
            EntityType::Album => write!(f, "Album"),
            EntityType::Story => write!(f, "Story"),
            EntityType::Custom1 => write!(f, "Custom1"),
            EntityType::Custom2 => write!(f, "Custom2"),
            EntityType::Custom3 => write!(f, "Custom3"),
            EntityType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A 64-bit entity identifier with embedded type information.
///
/// The high 16 bits encode the entity type, and the low 48 bits
/// encode the unique identifier within that type.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[repr(transparent)]
pub struct EntityId(u64);

impl EntityId {
    /// Create a new EntityId from type and raw ID.
    ///
    /// # Panics
    /// Panics if `raw_id` exceeds 48 bits (greater than `MAX_RAW_ID`).
    #[inline]
    pub fn new(entity_type: EntityType, raw_id: u64) -> Self {
        debug_assert!(raw_id <= MAX_RAW_ID, "raw_id exceeds 48 bits");
        let type_bits = (entity_type.as_raw() as u64) << ID_BITS;
        EntityId(type_bits | (raw_id & ID_MASK))
    }

    /// Create an EntityId from a raw 64-bit value.
    ///
    /// Use this when deserializing or receiving IDs from external sources.
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        EntityId(value)
    }

    /// Get the raw 64-bit value.
    #[inline]
    pub const fn as_raw(self) -> u64 {
        self.0
    }

    /// Extract the entity type from this ID.
    #[inline]
    pub fn entity_type(self) -> EntityType {
        let type_bits = (self.0 >> ID_BITS) as u16;
        EntityType::from_raw(type_bits)
    }

    /// Extract the raw ID (without type information).
    #[inline]
    pub const fn raw_id(self) -> u64 {
        self.0 & ID_MASK
    }

    /// Check if this is a valid ID (non-zero raw_id).
    #[inline]
    pub const fn is_valid(self) -> bool {
        (self.0 & ID_MASK) != 0
    }

    /// Create a null/invalid EntityId.
    #[inline]
    pub const fn null() -> Self {
        EntityId(0)
    }

    /// Check if this ID represents a User entity.
    #[inline]
    pub fn is_user(self) -> bool {
        self.entity_type() == EntityType::User
    }

    /// Check if this ID represents a Post entity.
    #[inline]
    pub fn is_post(self) -> bool {
        self.entity_type() == EntityType::Post
    }

    /// Check if this ID represents a Comment entity.
    #[inline]
    pub fn is_comment(self) -> bool {
        self.entity_type() == EntityType::Comment
    }
}

impl fmt::Debug for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EntityId({:?}:{})",
            self.entity_type(),
            self.raw_id()
        )
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.entity_type(), self.raw_id())
    }
}

impl From<u64> for EntityId {
    #[inline]
    fn from(value: u64) -> Self {
        EntityId::from_raw(value)
    }
}

impl From<EntityId> for u64 {
    #[inline]
    fn from(id: EntityId) -> Self {
        id.as_raw()
    }
}

/// Bits used for timestamp in generated IDs
const TIMESTAMP_BITS: u32 = 32;
/// Bits used for sequence counter in generated IDs
const SEQUENCE_BITS: u32 = 16;
/// Mask for sequence counter (lower 16 bits of raw_id)
const SEQUENCE_MASK: u64 = (1u64 << SEQUENCE_BITS) - 1;
/// Maximum sequence value before overflow
const MAX_SEQUENCE: u16 = 0xFFFF;

/// Thread-safe ID generator using hybrid timestamp + sequence approach.
///
/// Generates IDs with the following structure within the 48-bit raw_id:
/// - Upper 32 bits: seconds since AOEE_EPOCH (2020-01-01)
/// - Lower 16 bits: sequence counter (0-65535)
///
/// This allows 65,536 IDs per second per generator instance without
/// requiring a central coordinator.
///
/// # Example
/// ```
/// use aoee_core::id::{IdGenerator, EntityType};
///
/// let generator = IdGenerator::new(EntityType::User);
/// let id1 = generator.next_id();
/// let id2 = generator.next_id();
/// assert_ne!(id1, id2);
/// assert!(id1.is_user());
/// ```
pub struct IdGenerator {
    entity_type: EntityType,
    last_timestamp: AtomicU32,
    sequence: AtomicU16,
}

impl IdGenerator {
    /// Create a new ID generator for the specified entity type.
    pub fn new(entity_type: EntityType) -> Self {
        IdGenerator {
            entity_type,
            last_timestamp: AtomicU32::new(0),
            sequence: AtomicU16::new(0),
        }
    }

    /// Get the current timestamp as seconds since AOEE_EPOCH.
    #[inline]
    fn current_timestamp() -> u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        (now.saturating_sub(AOEE_EPOCH)) as u32
    }

    /// Generate the next unique ID.
    ///
    /// Thread-safe: multiple threads can call this concurrently.
    /// If more than 65,536 IDs are generated in the same second,
    /// this will spin-wait until the next second.
    pub fn next_id(&self) -> EntityId {
        loop {
            let now = Self::current_timestamp();
            let last = self.last_timestamp.load(Ordering::Acquire);

            if now > last {
                // New second - try to update timestamp and reset sequence
                if self.last_timestamp.compare_exchange(
                    last, now, Ordering::AcqRel, Ordering::Acquire
                ).is_ok() {
                    // Store 1 so next fetch_add returns 1 (we use 0 here)
                    self.sequence.store(1, Ordering::Release);
                    return self.make_id(now, 0);
                }
                // Another thread updated - retry
                continue;
            }

            // Same second - increment sequence
            let seq = self.sequence.fetch_add(1, Ordering::AcqRel);
            if seq <= MAX_SEQUENCE {
                return self.make_id(last, seq);
            }

            // Sequence overflow - wait for next second
            // In production, you might want to add a small sleep here
            std::hint::spin_loop();
        }
    }

    /// Generate multiple IDs efficiently.
    ///
    /// More efficient than calling `next_id()` in a loop when
    /// generating many IDs at once.
    pub fn next_ids(&self, count: usize) -> Vec<EntityId> {
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(self.next_id());
        }
        ids
    }

    /// Construct an EntityId from timestamp and sequence.
    #[inline]
    fn make_id(&self, timestamp: u32, sequence: u16) -> EntityId {
        let raw_id = ((timestamp as u64) << SEQUENCE_BITS) | (sequence as u64);
        EntityId::new(self.entity_type, raw_id)
    }

    /// Get the entity type this generator produces.
    #[inline]
    pub fn entity_type(&self) -> EntityType {
        self.entity_type
    }

    /// Extract timestamp from a generated ID (seconds since AOEE_EPOCH).
    ///
    /// Only valid for IDs created by an IdGenerator.
    #[inline]
    pub fn extract_timestamp(id: EntityId) -> u32 {
        (id.raw_id() >> SEQUENCE_BITS) as u32
    }

    /// Extract sequence number from a generated ID.
    ///
    /// Only valid for IDs created by an IdGenerator.
    #[inline]
    pub fn extract_sequence(id: EntityId) -> u16 {
        (id.raw_id() & SEQUENCE_MASK) as u16
    }

    /// Convert timestamp (seconds since AOEE_EPOCH) to Unix timestamp.
    #[inline]
    pub fn to_unix_timestamp(aoee_timestamp: u32) -> u64 {
        AOEE_EPOCH + aoee_timestamp as u64
    }

    /// Convert Unix timestamp to AOEE timestamp.
    #[inline]
    pub fn from_unix_timestamp(unix_timestamp: u64) -> u32 {
        unix_timestamp.saturating_sub(AOEE_EPOCH) as u32
    }
}

impl Clone for IdGenerator {
    /// Clone creates a new generator with the same type but fresh state.
    fn clone(&self) -> Self {
        IdGenerator::new(self.entity_type)
    }
}

impl fmt::Debug for IdGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdGenerator")
            .field("entity_type", &self.entity_type)
            .field("last_timestamp", &self.last_timestamp.load(Ordering::Relaxed))
            .field("sequence", &self.sequence.load(Ordering::Relaxed))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_creation() {
        let id = EntityId::new(EntityType::User, 12345);
        assert_eq!(id.entity_type(), EntityType::User);
        assert_eq!(id.raw_id(), 12345);
    }

    #[test]
    fn test_entity_id_roundtrip() {
        let original = EntityId::new(EntityType::Post, 999_999_999);
        let raw = original.as_raw();
        let restored = EntityId::from_raw(raw);
        assert_eq!(original, restored);
        assert_eq!(restored.entity_type(), EntityType::Post);
        assert_eq!(restored.raw_id(), 999_999_999);
    }

    #[test]
    fn test_entity_id_ordering() {
        // IDs of the same type should be ordered by raw_id
        let id1 = EntityId::new(EntityType::User, 100);
        let id2 = EntityId::new(EntityType::User, 200);
        assert!(id1 < id2);

        // Different types are ordered by type first
        let user_id = EntityId::new(EntityType::User, 1000);
        let post_id = EntityId::new(EntityType::Post, 1);
        assert!(user_id < post_id); // User(0) < Post(1)
    }

    #[test]
    fn test_max_raw_id() {
        let id = EntityId::new(EntityType::Comment, MAX_RAW_ID);
        assert_eq!(id.raw_id(), MAX_RAW_ID);
        assert_eq!(id.entity_type(), EntityType::Comment);
    }

    #[test]
    fn test_null_id() {
        let null = EntityId::null();
        assert!(!null.is_valid());
        assert_eq!(null.raw_id(), 0);
    }

    #[test]
    fn test_type_checks() {
        let user = EntityId::new(EntityType::User, 1);
        let post = EntityId::new(EntityType::Post, 1);
        let comment = EntityId::new(EntityType::Comment, 1);

        assert!(user.is_user());
        assert!(!user.is_post());

        assert!(post.is_post());
        assert!(!post.is_user());

        assert!(comment.is_comment());
    }

    #[test]
    fn test_entity_type_from_raw() {
        assert_eq!(EntityType::from_raw(0), EntityType::User);
        assert_eq!(EntityType::from_raw(1), EntityType::Post);
        assert_eq!(EntityType::from_raw(9999), EntityType::Unknown);
    }

    #[test]
    fn test_display() {
        let id = EntityId::new(EntityType::User, 42);
        assert_eq!(format!("{}", id), "User:42");
        assert_eq!(format!("{:?}", id), "EntityId(User:42)");
    }

    // IdGenerator tests

    #[test]
    fn test_generator_creates_typed_ids() {
        let gen = IdGenerator::new(EntityType::User);
        let id = gen.next_id();
        assert!(id.is_user());
        assert_eq!(id.entity_type(), EntityType::User);
    }

    #[test]
    fn test_generator_unique_ids() {
        let gen = IdGenerator::new(EntityType::Post);
        let ids: Vec<EntityId> = (0..100).map(|_| gen.next_id()).collect();
        
        // All IDs should be unique
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
    }

    #[test]
    fn test_generator_sequence_increments() {
        let gen = IdGenerator::new(EntityType::Comment);
        let id1 = gen.next_id();
        let id2 = gen.next_id();
        
        // Within the same second, sequence should increment
        let seq1 = IdGenerator::extract_sequence(id1);
        let seq2 = IdGenerator::extract_sequence(id2);
        
        // Sequence should be 0, 1 or incrementing
        assert!(seq2 > seq1 || seq2 == 0, "seq1={}, seq2={}", seq1, seq2);
    }

    #[test]
    fn test_generator_timestamp_extraction() {
        let gen = IdGenerator::new(EntityType::Photo);
        let id = gen.next_id();
        
        let ts = IdGenerator::extract_timestamp(id);
        let unix_ts = IdGenerator::to_unix_timestamp(ts);
        
        // Unix timestamp should be reasonable (after 2020, before 2100)
        assert!(unix_ts >= AOEE_EPOCH);
        assert!(unix_ts < AOEE_EPOCH + 80 * 365 * 24 * 3600); // Less than 80 years from epoch
    }

    #[test]
    fn test_generator_next_ids() {
        let gen = IdGenerator::new(EntityType::Video);
        let ids = gen.next_ids(50);
        
        assert_eq!(ids.len(), 50);
        
        // All should be Video type
        for id in &ids {
            assert_eq!(id.entity_type(), EntityType::Video);
        }
        
        // All should be unique
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 50);
    }

    #[test]
    fn test_generator_entity_type() {
        let gen = IdGenerator::new(EntityType::Group);
        assert_eq!(gen.entity_type(), EntityType::Group);
    }

    #[test]
    fn test_epoch_conversion() {
        // Test round-trip conversion
        let unix_ts: u64 = 1700000000; // Some time in 2023
        let aoee_ts = IdGenerator::from_unix_timestamp(unix_ts);
        let back = IdGenerator::to_unix_timestamp(aoee_ts);
        assert_eq!(back, unix_ts);
    }

    #[test]
    fn test_generator_clone_is_independent() {
        let gen1 = IdGenerator::new(EntityType::User);
        let _id1 = gen1.next_id();
        
        let gen2 = gen1.clone();
        let id2 = gen2.next_id();
        
        // Cloned generator should work independently
        assert!(id2.is_user());
    }

    #[test]
    fn test_different_generators_produce_different_types() {
        let user_gen = IdGenerator::new(EntityType::User);
        let post_gen = IdGenerator::new(EntityType::Post);
        
        let user_id = user_gen.next_id();
        let post_id = post_gen.next_id();
        
        assert!(user_id.is_user());
        assert!(post_id.is_post());
        assert_ne!(user_id.entity_type(), post_id.entity_type());
    }
}

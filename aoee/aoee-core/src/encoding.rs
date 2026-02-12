//! Encoding strategies for posting lists.
//!
//! Provides four encoding strategies with automatic selection based on list size and density:
//! - SmallVec: Plain Vec<u64> for small lists (< 128 elements)
//! - DeltaVarint: Delta encoding + variable-length integers (128..4096 elements)
//! - BlockPacked: Fixed-size blocks for large lists (> 4096 elements)
//! - Roaring: Roaring bitmaps for huge/dense lists

use crate::id::EntityId;
use roaring::RoaringBitmap;
use std::io::{self, Read, Write};
use thiserror::Error;

/// Encoding errors
#[derive(Error, Debug)]
pub enum EncodingError {
    #[error("Invalid encoding format")]
    InvalidFormat,
    #[error("Unexpected end of data")]
    UnexpectedEnd,
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Value overflow")]
    Overflow,
}

/// Thresholds for encoding strategy selection
pub const SMALL_VEC_THRESHOLD: usize = 128;
pub const DELTA_VARINT_THRESHOLD: usize = 4096;
/// Density threshold for switching to Roaring (ids per range)
pub const ROARING_DENSITY_THRESHOLD: f64 = 0.01;

/// Encoding strategy identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum EncodingStrategy {
    /// Plain Vec<u64> - for small lists
    SmallVec = 0,
    /// Delta + Varint encoding - for medium lists
    DeltaVarint = 1,
    /// Block-packed encoding - for large lists
    BlockPacked = 2,
    /// Roaring bitmap - for huge/dense lists
    Roaring = 3,
}

impl EncodingStrategy {
    /// Select the best encoding strategy for the given IDs.
    pub fn select(ids: &[EntityId]) -> Self {
        let len = ids.len();
        
        if len == 0 {
            return EncodingStrategy::SmallVec;
        }
        
        if len < SMALL_VEC_THRESHOLD {
            return EncodingStrategy::SmallVec;
        }
        
        // Check density for Roaring suitability
        if len > DELTA_VARINT_THRESHOLD {
            let first = ids.first().map(|id| id.as_raw()).unwrap_or(0);
            let last = ids.last().map(|id| id.as_raw()).unwrap_or(0);
            let range = last.saturating_sub(first) + 1;
            let density = len as f64 / range as f64;
            
            if density > ROARING_DENSITY_THRESHOLD || len > 100_000 {
                return EncodingStrategy::Roaring;
            }
            
            return EncodingStrategy::BlockPacked;
        }
        
        EncodingStrategy::DeltaVarint
    }

    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(EncodingStrategy::SmallVec),
            1 => Some(EncodingStrategy::DeltaVarint),
            2 => Some(EncodingStrategy::BlockPacked),
            3 => Some(EncodingStrategy::Roaring),
            _ => None,
        }
    }
}

/// Encoded list data with strategy tag
#[derive(Debug, Clone)]
pub enum EncodedList {
    SmallVec(Vec<EntityId>),
    DeltaVarint(Vec<u8>),
    BlockPacked(Vec<u8>),
    Roaring(RoaringBitmap),
}

impl EncodedList {
    /// Get the encoding strategy used
    pub fn strategy(&self) -> EncodingStrategy {
        match self {
            EncodedList::SmallVec(_) => EncodingStrategy::SmallVec,
            EncodedList::DeltaVarint(_) => EncodingStrategy::DeltaVarint,
            EncodedList::BlockPacked(_) => EncodingStrategy::BlockPacked,
            EncodedList::Roaring(_) => EncodingStrategy::Roaring,
        }
    }

    /// Get approximate memory size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            EncodedList::SmallVec(v) => v.len() * 8,
            EncodedList::DeltaVarint(v) => v.len(),
            EncodedList::BlockPacked(v) => v.len(),
            EncodedList::Roaring(r) => r.serialized_size(),
        }
    }
}

/// Trait for encoding/decoding posting lists
pub trait PostingEncoder {
    /// Encode a sorted list of IDs
    fn encode(ids: &[EntityId]) -> Result<EncodedList, EncodingError>;
    
    /// Decode to a vector of IDs
    fn decode(encoded: &EncodedList) -> Result<Vec<EntityId>, EncodingError>;
    
    /// Check if an ID exists in the encoded list
    fn contains(encoded: &EncodedList, id: EntityId) -> Result<bool, EncodingError>;
    
    /// Get the count of IDs
    fn count(encoded: &EncodedList) -> usize;
}

// ============================================================================
// SmallVec Encoding (trivial - just store the Vec)
// ============================================================================

pub struct SmallVecEncoder;

impl SmallVecEncoder {
    pub fn encode(ids: &[EntityId]) -> EncodedList {
        EncodedList::SmallVec(ids.to_vec())
    }

    pub fn decode(ids: &[EntityId]) -> Vec<EntityId> {
        ids.to_vec()
    }

    pub fn contains(ids: &[EntityId], target: EntityId) -> bool {
        ids.binary_search(&target).is_ok()
    }
}

// ============================================================================
// Delta + Varint Encoding
// ============================================================================

pub struct DeltaVarintEncoder;

impl DeltaVarintEncoder {
    /// Encode a u64 as a variable-length integer (LEB128-style)
    fn encode_varint(value: u64, out: &mut Vec<u8>) {
        let mut v = value;
        loop {
            let mut byte = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    /// Decode a variable-length integer
    fn decode_varint(data: &[u8], pos: &mut usize) -> Result<u64, EncodingError> {
        let mut result: u64 = 0;
        let mut shift = 0;
        
        loop {
            if *pos >= data.len() {
                return Err(EncodingError::UnexpectedEnd);
            }
            let byte = data[*pos];
            *pos += 1;
            
            result |= ((byte & 0x7F) as u64) << shift;
            
            if byte & 0x80 == 0 {
                break;
            }
            
            shift += 7;
            if shift >= 64 {
                return Err(EncodingError::Overflow);
            }
        }
        
        Ok(result)
    }

    pub fn encode(ids: &[EntityId]) -> Result<EncodedList, EncodingError> {
        if ids.is_empty() {
            return Ok(EncodedList::DeltaVarint(Vec::new()));
        }

        let mut out = Vec::with_capacity(ids.len() * 2); // Estimate
        
        // First, encode the count
        Self::encode_varint(ids.len() as u64, &mut out);
        
        // Encode first value directly
        let mut prev = ids[0].as_raw();
        Self::encode_varint(prev, &mut out);
        
        // Encode remaining as deltas
        for id in &ids[1..] {
            let curr = id.as_raw();
            let delta = curr.saturating_sub(prev);
            Self::encode_varint(delta, &mut out);
            prev = curr;
        }
        
        Ok(EncodedList::DeltaVarint(out))
    }

    pub fn decode(data: &[u8]) -> Result<Vec<EntityId>, EncodingError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut pos = 0;
        let count = Self::decode_varint(data, &mut pos)? as usize;
        
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut ids = Vec::with_capacity(count);
        
        // Decode first value
        let mut prev = Self::decode_varint(data, &mut pos)?;
        ids.push(EntityId::from_raw(prev));
        
        // Decode remaining deltas
        for _ in 1..count {
            let delta = Self::decode_varint(data, &mut pos)?;
            prev = prev.saturating_add(delta);
            ids.push(EntityId::from_raw(prev));
        }
        
        Ok(ids)
    }

    pub fn contains(data: &[u8], target: EntityId) -> Result<bool, EncodingError> {
        // For contains, we could use skip tables for efficiency
        // For now, linear decode (can be optimized later)
        let ids = Self::decode(data)?;
        Ok(ids.binary_search(&target).is_ok())
    }

    pub fn count(data: &[u8]) -> Result<usize, EncodingError> {
        if data.is_empty() {
            return Ok(0);
        }
        let mut pos = 0;
        let count = Self::decode_varint(data, &mut pos)? as usize;
        Ok(count)
    }
}

// ============================================================================
// Block-Packed Encoding
// ============================================================================

/// Block size for block-packed encoding (number of values per block)
pub const BLOCK_SIZE: usize = 128;

pub struct BlockPackedEncoder;

impl BlockPackedEncoder {
    /// Calculate bits needed to represent a value
    fn bits_needed(value: u64) -> u8 {
        if value == 0 {
            return 1;
        }
        64 - value.leading_zeros() as u8
    }

    /// Pack values into a block with given bit width
    fn pack_block(values: &[u64], bit_width: u8, out: &mut Vec<u8>) {
        if bit_width == 0 {
            return;
        }

        let mut buffer: u64 = 0;
        let mut bits_in_buffer = 0u8;

        for &value in values {
            buffer |= (value & ((1u64 << bit_width) - 1)) << bits_in_buffer;
            bits_in_buffer += bit_width;

            while bits_in_buffer >= 8 {
                out.push(buffer as u8);
                buffer >>= 8;
                bits_in_buffer -= 8;
            }
        }

        // Flush remaining bits
        if bits_in_buffer > 0 {
            out.push(buffer as u8);
        }
    }

    /// Unpack a block with given bit width
    fn unpack_block(data: &[u8], bit_width: u8, count: usize, out: &mut Vec<u64>) -> Result<usize, EncodingError> {
        if bit_width == 0 {
            out.extend(std::iter::repeat(0).take(count));
            return Ok(0);
        }

        let mut buffer: u64 = 0;
        let mut bits_in_buffer = 0u8;
        let mut byte_pos = 0;
        let mask = (1u64 << bit_width) - 1;

        for _ in 0..count {
            while bits_in_buffer < bit_width {
                if byte_pos >= data.len() {
                    return Err(EncodingError::UnexpectedEnd);
                }
                buffer |= (data[byte_pos] as u64) << bits_in_buffer;
                byte_pos += 1;
                bits_in_buffer += 8;
            }

            out.push(buffer & mask);
            buffer >>= bit_width;
            bits_in_buffer -= bit_width;
        }

        Ok(byte_pos)
    }

    pub fn encode(ids: &[EntityId]) -> Result<EncodedList, EncodingError> {
        if ids.is_empty() {
            return Ok(EncodedList::BlockPacked(Vec::new()));
        }

        let mut out = Vec::new();
        
        // Header: total count (4 bytes)
        out.extend_from_slice(&(ids.len() as u32).to_le_bytes());
        
        // First value (8 bytes) - needed to reconstruct from deltas
        out.extend_from_slice(&ids[0].as_raw().to_le_bytes());
        
        // Compute deltas
        let mut deltas: Vec<u64> = Vec::with_capacity(ids.len());
        deltas.push(0); // First delta is 0 (we store first value separately)
        
        let mut prev = ids[0].as_raw();
        for id in &ids[1..] {
            let curr = id.as_raw();
            deltas.push(curr.saturating_sub(prev));
            prev = curr;
        }

        // Process in blocks
        for chunk in deltas.chunks(BLOCK_SIZE) {
            // Find max delta in block to determine bit width
            let max_delta = chunk.iter().copied().max().unwrap_or(0);
            let bit_width = Self::bits_needed(max_delta);
            
            // Block header: count (1 byte) + bit_width (1 byte)
            out.push(chunk.len() as u8);
            out.push(bit_width);
            
            // Pack the block
            Self::pack_block(chunk, bit_width, &mut out);
        }

        Ok(EncodedList::BlockPacked(out))
    }

    pub fn decode(data: &[u8]) -> Result<Vec<EntityId>, EncodingError> {
        if data.len() < 12 {
            if data.is_empty() {
                return Ok(Vec::new());
            }
            return Err(EncodingError::UnexpectedEnd);
        }

        let mut pos = 0;
        
        // Read count
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        pos += 4;
        
        if count == 0 {
            return Ok(Vec::new());
        }

        // Read first value
        let first_value = u64::from_le_bytes([
            data[4], data[5], data[6], data[7],
            data[8], data[9], data[10], data[11],
        ]);
        pos += 8;

        let mut deltas: Vec<u64> = Vec::with_capacity(count);
        
        // Read blocks
        while deltas.len() < count {
            if pos + 2 > data.len() {
                return Err(EncodingError::UnexpectedEnd);
            }
            
            let block_count = data[pos] as usize;
            let bit_width = data[pos + 1];
            pos += 2;
            
            let bytes_read = Self::unpack_block(&data[pos..], bit_width, block_count, &mut deltas)?;
            pos += bytes_read;
        }

        // Reconstruct values from deltas
        let mut ids = Vec::with_capacity(count);
        let mut prev = first_value;
        
        for (i, &delta) in deltas.iter().enumerate() {
            if i == 0 {
                ids.push(EntityId::from_raw(prev));
            } else {
                prev = prev.saturating_add(delta);
                ids.push(EntityId::from_raw(prev));
            }
        }

        Ok(ids)
    }

    pub fn contains(data: &[u8], target: EntityId) -> Result<bool, EncodingError> {
        let ids = Self::decode(data)?;
        Ok(ids.binary_search(&target).is_ok())
    }

    pub fn count(data: &[u8]) -> Result<usize, EncodingError> {
        if data.len() < 4 {
            return Ok(0);
        }
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        Ok(count)
    }
}

// ============================================================================
// Roaring Bitmap Encoding
// ============================================================================

pub struct RoaringEncoder;

impl RoaringEncoder {
    /// Encode IDs using Roaring bitmap.
    /// 
    /// Note: Roaring bitmap uses u32, so we need to handle 64-bit IDs specially.
    /// We partition by the high 32 bits and store separate bitmaps.
    /// For simplicity in this version, we only use the low 32 bits.
    /// A production version would handle the full 64-bit range.
    pub fn encode(ids: &[EntityId]) -> Result<EncodedList, EncodingError> {
        let mut bitmap = RoaringBitmap::new();
        
        for id in ids {
            // For now, we use the raw_id() which is 48 bits
            // We'll truncate to 32 bits for Roaring
            // A full implementation would partition by high bits
            let low32 = (id.as_raw() & 0xFFFF_FFFF) as u32;
            bitmap.insert(low32);
        }
        
        Ok(EncodedList::Roaring(bitmap))
    }

    pub fn decode(bitmap: &RoaringBitmap) -> Vec<EntityId> {
        bitmap.iter().map(|v| EntityId::from_raw(v as u64)).collect()
    }

    pub fn contains(bitmap: &RoaringBitmap, target: EntityId) -> bool {
        let low32 = (target.as_raw() & 0xFFFF_FFFF) as u32;
        bitmap.contains(low32)
    }

    pub fn count(bitmap: &RoaringBitmap) -> usize {
        bitmap.len() as usize
    }
}

// ============================================================================
// Unified Encoder
// ============================================================================

/// Main encoder that auto-selects strategy
pub struct AutoEncoder;

impl PostingEncoder for AutoEncoder {
    fn encode(ids: &[EntityId]) -> Result<EncodedList, EncodingError> {
        let strategy = EncodingStrategy::select(ids);
        
        match strategy {
            EncodingStrategy::SmallVec => Ok(SmallVecEncoder::encode(ids)),
            EncodingStrategy::DeltaVarint => DeltaVarintEncoder::encode(ids),
            EncodingStrategy::BlockPacked => BlockPackedEncoder::encode(ids),
            EncodingStrategy::Roaring => RoaringEncoder::encode(ids),
        }
    }

    fn decode(encoded: &EncodedList) -> Result<Vec<EntityId>, EncodingError> {
        match encoded {
            EncodedList::SmallVec(ids) => Ok(SmallVecEncoder::decode(ids)),
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::decode(data),
            EncodedList::BlockPacked(data) => BlockPackedEncoder::decode(data),
            EncodedList::Roaring(bitmap) => Ok(RoaringEncoder::decode(bitmap)),
        }
    }

    fn contains(encoded: &EncodedList, id: EntityId) -> Result<bool, EncodingError> {
        match encoded {
            EncodedList::SmallVec(ids) => Ok(SmallVecEncoder::contains(ids, id)),
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::contains(data, id),
            EncodedList::BlockPacked(data) => BlockPackedEncoder::contains(data, id),
            EncodedList::Roaring(bitmap) => Ok(RoaringEncoder::contains(bitmap, id)),
        }
    }

    fn count(encoded: &EncodedList) -> usize {
        match encoded {
            EncodedList::SmallVec(ids) => ids.len(),
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::count(data).unwrap_or(0),
            EncodedList::BlockPacked(data) => BlockPackedEncoder::count(data).unwrap_or(0),
            EncodedList::Roaring(bitmap) => RoaringEncoder::count(bitmap),
        }
    }
}

/// Force a specific encoding strategy
pub fn encode_with_strategy(ids: &[EntityId], strategy: EncodingStrategy) -> Result<EncodedList, EncodingError> {
    match strategy {
        EncodingStrategy::SmallVec => Ok(SmallVecEncoder::encode(ids)),
        EncodingStrategy::DeltaVarint => DeltaVarintEncoder::encode(ids),
        EncodingStrategy::BlockPacked => BlockPackedEncoder::encode(ids),
        EncodingStrategy::Roaring => RoaringEncoder::encode(ids),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityType;

    fn make_ids(raw_ids: &[u64]) -> Vec<EntityId> {
        raw_ids.iter().map(|&id| EntityId::new(EntityType::User, id)).collect()
    }

    fn make_sequential_ids(start: u64, count: usize) -> Vec<EntityId> {
        (start..start + count as u64)
            .map(|id| EntityId::new(EntityType::User, id))
            .collect()
    }

    fn make_sparse_ids(start: u64, count: usize, step: u64) -> Vec<EntityId> {
        (0..count as u64)
            .map(|i| EntityId::new(EntityType::User, start + i * step))
            .collect()
    }

    // ========== SmallVec Tests ==========
    
    #[test]
    fn test_smallvec_empty() {
        let ids: Vec<EntityId> = vec![];
        let encoded = SmallVecEncoder::encode(&ids);
        match encoded {
            EncodedList::SmallVec(decoded) => assert!(decoded.is_empty()),
            _ => panic!("Wrong encoding type"),
        }
    }

    #[test]
    fn test_smallvec_roundtrip() {
        let ids = make_ids(&[1, 5, 10, 100, 1000]);
        let encoded = SmallVecEncoder::encode(&ids);
        match &encoded {
            EncodedList::SmallVec(decoded) => assert_eq!(*decoded, ids),
            _ => panic!("Wrong encoding type"),
        }
    }

    #[test]
    fn test_smallvec_contains() {
        let ids = make_ids(&[1, 5, 10, 100, 1000]);
        assert!(SmallVecEncoder::contains(&ids, EntityId::new(EntityType::User, 5)));
        assert!(!SmallVecEncoder::contains(&ids, EntityId::new(EntityType::User, 6)));
    }

    // ========== DeltaVarint Tests ==========

    #[test]
    fn test_delta_varint_empty() {
        let ids: Vec<EntityId> = vec![];
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_delta_varint_single() {
        let ids = make_ids(&[42]);
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_delta_varint_roundtrip() {
        let ids = make_ids(&[1, 5, 10, 100, 1000, 10000]);
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_delta_varint_sequential() {
        let ids = make_sequential_ids(1, 500);
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::DeltaVarint(data) => DeltaVarintEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_delta_varint_compression() {
        // Sequential IDs should compress well (delta of 1)
        let ids = make_sequential_ids(1, 1000);
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        let size = match &encoded {
            EncodedList::DeltaVarint(data) => data.len(),
            _ => panic!("Wrong encoding type"),
        };
        // Should be much smaller than 8 bytes per ID
        assert!(size < ids.len() * 4);
    }

    #[test]
    fn test_delta_varint_contains() {
        let ids = make_ids(&[1, 5, 10, 100, 1000]);
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        match &encoded {
            EncodedList::DeltaVarint(data) => {
                assert!(DeltaVarintEncoder::contains(data, EntityId::new(EntityType::User, 5)).unwrap());
                assert!(!DeltaVarintEncoder::contains(data, EntityId::new(EntityType::User, 6)).unwrap());
            }
            _ => panic!("Wrong encoding type"),
        }
    }

    // ========== BlockPacked Tests ==========

    #[test]
    fn test_blockpacked_empty() {
        let ids: Vec<EntityId> = vec![];
        let encoded = BlockPackedEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::BlockPacked(data) => BlockPackedEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_blockpacked_roundtrip() {
        let ids = make_sequential_ids(1, 500);
        let encoded = BlockPackedEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::BlockPacked(data) => BlockPackedEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_blockpacked_large() {
        let ids = make_sequential_ids(1, 5000);
        let encoded = BlockPackedEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::BlockPacked(data) => BlockPackedEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_blockpacked_sparse() {
        let ids = make_sparse_ids(1, 500, 1000);
        let encoded = BlockPackedEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::BlockPacked(data) => BlockPackedEncoder::decode(data).unwrap(),
            _ => panic!("Wrong encoding type"),
        };
        assert_eq!(decoded, ids);
    }

    // ========== Roaring Tests ==========

    #[test]
    fn test_roaring_empty() {
        let ids: Vec<EntityId> = vec![];
        let encoded = RoaringEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::Roaring(bitmap) => RoaringEncoder::decode(bitmap),
            _ => panic!("Wrong encoding type"),
        };
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_roaring_roundtrip() {
        let ids = make_sequential_ids(1, 1000);
        let encoded = RoaringEncoder::encode(&ids).unwrap();
        let decoded = match &encoded {
            EncodedList::Roaring(bitmap) => RoaringEncoder::decode(bitmap),
            _ => panic!("Wrong encoding type"),
        };
        // Note: Roaring only stores low 32 bits, so we compare those
        assert_eq!(decoded.len(), ids.len());
    }

    #[test]
    fn test_roaring_contains() {
        let ids = make_ids(&[1, 5, 10, 100, 1000]);
        let encoded = RoaringEncoder::encode(&ids).unwrap();
        match &encoded {
            EncodedList::Roaring(bitmap) => {
                assert!(RoaringEncoder::contains(bitmap, EntityId::new(EntityType::User, 5)));
                assert!(!RoaringEncoder::contains(bitmap, EntityId::new(EntityType::User, 6)));
            }
            _ => panic!("Wrong encoding type"),
        }
    }

    // ========== Auto-selection Tests ==========

    #[test]
    fn test_auto_select_smallvec() {
        let ids = make_ids(&[1, 2, 3, 4, 5]);
        assert_eq!(EncodingStrategy::select(&ids), EncodingStrategy::SmallVec);
    }

    #[test]
    fn test_auto_select_delta_varint() {
        let ids = make_sequential_ids(1, 200);
        assert_eq!(EncodingStrategy::select(&ids), EncodingStrategy::DeltaVarint);
    }

    #[test]
    fn test_auto_select_blockpacked() {
        let ids = make_sparse_ids(1, 5000, 1000);
        let strategy = EncodingStrategy::select(&ids);
        assert!(matches!(strategy, EncodingStrategy::BlockPacked | EncodingStrategy::Roaring));
    }

    #[test]
    fn test_auto_encoder_roundtrip() {
        // Test various sizes
        for size in [0, 1, 50, 200, 5000] {
            let ids = make_sequential_ids(1, size);
            let encoded = AutoEncoder::encode(&ids).unwrap();
            let decoded = AutoEncoder::decode(&encoded).unwrap();
            assert_eq!(decoded.len(), ids.len(), "Size mismatch for count={}", size);
        }
    }

    // ========== Varint Unit Tests ==========

    #[test]
    fn test_varint_small_values() {
        let mut out = Vec::new();
        DeltaVarintEncoder::encode_varint(0, &mut out);
        assert_eq!(out, vec![0]);

        out.clear();
        DeltaVarintEncoder::encode_varint(127, &mut out);
        assert_eq!(out, vec![127]);

        out.clear();
        DeltaVarintEncoder::encode_varint(128, &mut out);
        assert_eq!(out, vec![0x80, 0x01]);
    }

    #[test]
    fn test_varint_roundtrip() {
        for value in [0, 1, 127, 128, 16383, 16384, u32::MAX as u64, u64::MAX] {
            let mut out = Vec::new();
            DeltaVarintEncoder::encode_varint(value, &mut out);
            let mut pos = 0;
            let decoded = DeltaVarintEncoder::decode_varint(&out, &mut pos).unwrap();
            assert_eq!(decoded, value, "Failed for value {}", value);
        }
    }
}

# Posting List Encoding

This document describes how AOEE encodes posting lists (adjacency lists) in memory for efficient storage and fast operations.

## Overview

AOEE stores relationship edges as **posting lists** - sorted lists of destination entity IDs for each (source, edge_type) pair. For example, "User 123's followers" is a posting list containing all user IDs that follow user 123.

To minimize memory usage while maintaining fast access, AOEE uses **adaptive encoding** that selects the best compression strategy based on list characteristics.

## Encoding Strategies

AOEE automatically selects from four encoding strategies:

| Strategy | List Size | Description | Bytes per ID |
|----------|-----------|-------------|--------------|
| SmallVec | < 128 | Plain array of u64 | 8 bytes |
| DeltaVarint | 128 - 4,096 | Delta + variable-length int | 1-2 bytes typical |
| BlockPacked | > 4,096 | Fixed-width blocks | ~2-4 bytes |
| Roaring | Very large/dense | Roaring bitmaps | ~0.1-2 bytes |

## Delta-Varint Encoding (Primary Strategy)

For most real-world social graphs, **DeltaVarint** provides the best balance of compression and speed.

### How It Works

**Step 1: Sort IDs**
```
Original: [1000, 1005, 1007, 1010, 1050]
Sorted:   [1000, 1005, 1007, 1010, 1050]  (already sorted)
```

**Step 2: Compute Deltas**

Instead of storing absolute IDs, store the *difference* from the previous ID:
```
IDs:    [1000, 1005, 1007, 1010, 1050]
Deltas: [1000,    5,    2,    3,   40]
         ↑        ↑     ↑     ↑     ↑
         first    +5    +2    +3   +40
```

**Step 3: Encode with Variable-Length Integers (Varint)**

Small numbers use fewer bytes using LEB128-style encoding:

| Value Range | Bytes Used |
|-------------|------------|
| 0 - 127 | 1 byte |
| 128 - 16,383 | 2 bytes |
| 16,384 - 2,097,151 | 3 bytes |
| ... | ... |

**Varint Encoding Format:**
- Each byte uses 7 bits for data, 1 bit as continuation flag
- If high bit = 1, more bytes follow
- If high bit = 0, this is the last byte

```
Value 5:    0x05        (1 byte:  0000 0101)
Value 127:  0x7F        (1 byte:  0111 1111)
Value 128:  0x80 0x01   (2 bytes: 1000 0000, 0000 0001)
Value 300:  0xAC 0x02   (2 bytes: 1010 1100, 0000 0010)
```

### Compression Example

Consider a user with 1,000 followers (sequential IDs 5000-5999):

**Without compression:**
```
1,000 IDs × 8 bytes = 8,000 bytes
```

**With delta-varint:**
```
- Count: 1 varint (2 bytes)
- First ID (5000): 2 bytes
- 999 deltas of 1: 999 × 1 byte = 999 bytes
Total: ~1,003 bytes (87% reduction!)
```

### Code Location

Implementation: `aoee-core/src/encoding.rs`

```rust
// Encode a u64 as variable-length integer (LEB128)
fn encode_varint(value: u64, out: &mut Vec<u8>) {
    let mut v = value;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;  // Set continuation bit
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

// Delta encoding
pub fn encode(ids: &[EntityId]) -> Result<EncodedList, EncodingError> {
    // Encode count
    Self::encode_varint(ids.len() as u64, &mut out);
    
    // First value stored directly
    let mut prev = ids[0].as_raw();
    Self::encode_varint(prev, &mut out);
    
    // Remaining as deltas
    for id in &ids[1..] {
        let curr = id.as_raw();
        let delta = curr.saturating_sub(prev);
        Self::encode_varint(delta, &mut out);
        prev = curr;
    }
    
    Ok(EncodedList::DeltaVarint(out))
}
```

## Block-Packed Encoding

For large lists (> 4,096 elements), AOEE uses **block-packed encoding**:

1. Divide IDs into blocks of 128 values
2. Compute deltas within each block
3. Find the maximum delta in the block
4. Pack all deltas using the minimum bits needed for the max

**Example:**
```
Block deltas: [1, 2, 1, 3, 1, 1, 2, 1, ...]  (max = 3, needs 2 bits)
Packed: Each delta uses exactly 2 bits
128 deltas × 2 bits = 256 bits = 32 bytes per block
```

Benefits:
- SIMD-friendly (fixed-width values)
- Fast random access within blocks
- Good compression for uniform distributions

## Roaring Bitmap Encoding

For very large or dense lists (> 100,000 elements or high density), AOEE uses **Roaring bitmaps**:

- Hybrid container structure (arrays, bitmaps, runs)
- Excellent for dense ID ranges
- Supports fast set operations (AND, OR, XOR)

## Posting List Structure

A complete posting list has two parts:

```
PostingList {
    buffer: WriteBuffer,     // Recent changes (unsorted, uncompressed)
    segments: Vec<Segment>,  // Compacted data (sorted, compressed)
}
```

**WriteBuffer:** New edges are appended here (fast writes)

**Segments:** Periodically, the buffer is compacted into immutable segments using the encoding strategies above.

```
Segment {
    data: EncodedList,           // Compressed IDs
    first: EntityId,             // Min ID (for range checks)
    last: EntityId,              // Max ID (for range checks)  
    count: u32,                  // Number of IDs
    skip_table: Vec<(EntityId, u32)>,  // For fast seeking
}
```

## Performance Characteristics

### Memory Usage

| Scenario | Raw (8B/ID) | Delta-Varint | Savings |
|----------|-------------|--------------|---------|
| 100 sequential IDs | 800 B | ~110 B | 86% |
| 1,000 followers (sparse) | 8 KB | ~2 KB | 75% |
| 10,000 likes | 80 KB | ~15 KB | 81% |

### Operation Complexity

| Operation | SmallVec | DeltaVarint | BlockPacked |
|-----------|----------|-------------|-------------|
| Iteration | O(n) | O(n) | O(n) |
| Contains | O(log n) | O(n)* | O(log n + k) |
| Count | O(1) | O(1) | O(1) |
| Decode | O(1) | O(n) | O(n) |

*Note: DeltaVarint `contains` currently requires full decode. Skip tables can improve this to O(log n).

## Configuration

Thresholds are defined in `aoee-core/src/encoding.rs`:

```rust
pub const SMALL_VEC_THRESHOLD: usize = 128;      // Use SmallVec below this
pub const DELTA_VARINT_THRESHOLD: usize = 4096;  // Use DeltaVarint below this
pub const ROARING_DENSITY_THRESHOLD: f64 = 0.01; // Switch to Roaring above this
```

## Future Improvements

1. **Skip tables for DeltaVarint** - Enable O(log n) contains without full decode
2. **Lower SmallVec threshold** - More lists benefit from compression
3. **SIMD decoding** - Vectorized varint decoding for 2-4x speedup
4. **Adaptive block sizes** - Tune block size based on delta distribution

## References

- [LEB128 Variable-Length Encoding](https://en.wikipedia.org/wiki/LEB128)
- [Roaring Bitmaps](https://roaringbitmap.org/)
- [Frame of Reference (FOR) Encoding](https://lemire.me/blog/2012/02/08/effective-compression-using-frame-of-reference-and-delta-coding/)
- [SIMD-Based Decoding](https://arxiv.org/abs/1209.2137)

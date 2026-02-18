# Entity ID System

This document describes the 64-bit Entity ID system used throughout AOEE for identifying entities (users, posts, comments, etc.) with embedded type information.

## Overview

Every entity in AOEE is identified by a **64-bit integer** that encodes both the entity type and a unique identifier. This design enables:

- **O(1) type extraction** - No database lookup needed to determine what kind of entity an ID represents
- **Efficient compression** - IDs remain integers, enabling delta+varint encoding
- **Cross-language compatibility** - Any language that supports 64-bit integers can work with these IDs
- **Distributed generation** - No central coordinator required

## ID Layout

```
64-bit EntityId:
┌─────────────────┬──────────────────────────────────────────────┐
│   Type (16)     │              Raw ID (48)                     │
├─────────────────┼──────────────────────────────────────────────┤
│ Bits 63-48      │              Bits 47-0                       │
└─────────────────┴──────────────────────────────────────────────┘

Capacity:
- Types:  65,536 different entity types (2^16)
- IDs:    281,474,976,710,656 unique IDs per type (2^48 ≈ 281 trillion)
```

### Bit Manipulation

```
Extract type:    type   = id >> 48
Extract raw_id:  raw_id = id & 0x0000_FFFF_FFFF_FFFF
Construct ID:    id     = (type << 48) | raw_id
```

## Example IDs

| Entity | Type Code | Raw ID | Full 64-bit ID (hex) | Full 64-bit ID (decimal) |
|--------|-----------|--------|----------------------|--------------------------|
| User 1 | 0 | 1 | `0x0000000000000001` | 1 |
| User 1000 | 0 | 1000 | `0x00000000000003E8` | 1000 |
| Post 1 | 1 | 1 | `0x0001000000000001` | 281474976710657 |
| Post 1000 | 1 | 1000 | `0x00010000000003E8` | 281474976711656 |
| Comment 42 | 2 | 42 | `0x000200000000002A` | 562949953421354 |
| Photo 999 | 3 | 999 | `0x00030000000003E7` | 844424930132967 |

**Key insight:** User 1000 and Post 1 have very different 64-bit values despite similar "raw" numbers. This is by design - the type is encoded in the high bits.

## Entity Types

### Predefined Types

| Type | Code | Description |
|------|------|-------------|
| User | 0 | Person, account, profile |
| Post | 1 | Status update, article, content |
| Comment | 2 | Comment on content |
| Photo | 3 | Image, picture |
| Video | 4 | Video content |
| Group | 5 | User group, community |
| Page | 6 | Business page, fan page |
| Event | 7 | Calendar event |
| Message | 8 | Direct message |
| Reaction | 9 | Like, love, etc. |
| Tag | 10 | Hashtag, label |
| Location | 11 | Place, venue |
| Link | 12 | URL, external link |
| Album | 13 | Photo/video collection |
| Story | 14 | Ephemeral content |
| Custom1-3 | 100-102 | Application-defined |
| Unknown | 65535 | Invalid/unknown type |

### Type Hierarchy (Logical, Not Encoded)

While the bit representation is flat, the type system can track logical hierarchies:

```
Content
├── Post
├── Comment
├── Photo
├── Video
└── Story

Social
├── User
├── Group
└── Page

Media
├── Photo
├── Video
└── Album
```

This hierarchy is maintained in application logic, not in the ID bits.

## ID Generation

### Recommended: Hybrid Timestamp + Sequence

For distributed systems without a central coordinator:

```
Raw ID (48 bits):
┌────────────────────────────────────┬──────────────────┐
│        Timestamp (32)              │   Sequence (16)  │
├────────────────────────────────────┼──────────────────┤
│        Seconds since epoch         │   Counter 0-65535│
└────────────────────────────────────┴──────────────────┘
```

**Custom Epoch:** January 1, 2020 00:00:00 UTC
- Provides ~136 years of range (until ~2156)
- More efficient than Unix epoch (1970)

**Generation Algorithm:**

```rust
const EPOCH: u64 = 1577836800; // 2020-01-01 00:00:00 UTC

struct IdGenerator {
    entity_type: u16,
    last_timestamp: u32,
    sequence: u16,
}

impl IdGenerator {
    fn next_id(&mut self) -> u64 {
        let now = (current_unix_timestamp() - EPOCH) as u32;
        
        if now == self.last_timestamp {
            self.sequence += 1;
            if self.sequence == 0 {
                // Overflow - wait for next second
                wait_for_next_second();
                self.sequence = 0;
            }
        } else {
            self.last_timestamp = now;
            self.sequence = 0;
        }
        
        let raw_id = ((self.last_timestamp as u64) << 16) | (self.sequence as u64);
        ((self.entity_type as u64) << 48) | raw_id
    }
}
```

**Capacity:** 65,536 IDs per second per type per generator instance.

### Alternative: Database Sequences

For centralized systems:

```sql
-- PostgreSQL example
CREATE SEQUENCE user_id_seq;
CREATE SEQUENCE post_id_seq;

-- Generate typed ID
SELECT (0::bigint << 48) | nextval('user_id_seq') AS user_id;
SELECT (1::bigint << 48) | nextval('post_id_seq') AS post_id;
```

### Alternative: UUID to 64-bit

If you have existing UUIDs:

```python
import uuid
import hashlib

def uuid_to_typed_id(u: uuid.UUID, entity_type: int) -> int:
    # Use lower 48 bits of UUID's hash
    hash_bytes = hashlib.sha256(u.bytes).digest()
    raw_id = int.from_bytes(hash_bytes[:6], 'big')  # 48 bits
    return (entity_type << 48) | raw_id
```

**Warning:** This loses UUID uniqueness guarantees. Only use if collision probability is acceptable.

## Cross-Language Examples

### Rust (Native)

```rust
use aoee_core::{EntityId, EntityType};

// Create typed ID
let user_id = EntityId::new(EntityType::User, 12345);
let post_id = EntityId::new(EntityType::Post, 67890);

// Extract components
let entity_type = user_id.entity_type();  // EntityType::User
let raw_id = user_id.raw_id();            // 12345
let full_id = user_id.as_raw();           // 12345 (type 0 in high bits)

// Type checking
if user_id.is_user() {
    println!("This is a user!");
}

// From raw 64-bit value
let restored = EntityId::from_raw(0x0001000000000001);
assert_eq!(restored.entity_type(), EntityType::Post);
assert_eq!(restored.raw_id(), 1);
```

### Java

```java
public class EntityId {
    private static final long TYPE_MASK = 0xFFFF_0000_0000_0000L;
    private static final long ID_MASK = 0x0000_FFFF_FFFF_FFFFL;
    
    private final long value;
    
    public EntityId(int type, long rawId) {
        this.value = ((long) type << 48) | (rawId & ID_MASK);
    }
    
    public static EntityId fromRaw(long value) {
        return new EntityId(value);
    }
    
    public int getType() {
        return (int) (value >>> 48);
    }
    
    public long getRawId() {
        return value & ID_MASK;
    }
    
    public long asRaw() {
        return value;
    }
    
    public boolean isUser() {
        return getType() == 0;
    }
    
    public boolean isPost() {
        return getType() == 1;
    }
    
    @Override
    public String toString() {
        return EntityType.fromCode(getType()).name() + ":" + getRawId();
    }
}

// Usage
EntityId userId = new EntityId(0, 12345);      // User 12345
EntityId postId = new EntityId(1, 67890);      // Post 67890

long raw = postId.asRaw();                      // 281474976778570
EntityId restored = EntityId.fromRaw(raw);
System.out.println(restored.getType());         // 1 (Post)
```

### Python

```python
class EntityId:
    TYPE_SHIFT = 48
    ID_MASK = 0x0000_FFFF_FFFF_FFFF
    
    # Type codes
    USER = 0
    POST = 1
    COMMENT = 2
    PHOTO = 3
    
    def __init__(self, entity_type: int, raw_id: int):
        self.value = (entity_type << self.TYPE_SHIFT) | (raw_id & self.ID_MASK)
    
    @classmethod
    def from_raw(cls, value: int) -> 'EntityId':
        obj = cls.__new__(cls)
        obj.value = value
        return obj
    
    @property
    def entity_type(self) -> int:
        return self.value >> self.TYPE_SHIFT
    
    @property
    def raw_id(self) -> int:
        return self.value & self.ID_MASK
    
    def is_user(self) -> bool:
        return self.entity_type == self.USER
    
    def is_post(self) -> bool:
        return self.entity_type == self.POST
    
    def __repr__(self):
        type_names = {0: 'User', 1: 'Post', 2: 'Comment', 3: 'Photo'}
        type_name = type_names.get(self.entity_type, f'Type{self.entity_type}')
        return f'{type_name}:{self.raw_id}'

# Usage
user_id = EntityId(EntityId.USER, 12345)
post_id = EntityId(EntityId.POST, 67890)

raw = post_id.value                    # 281474976778570
restored = EntityId.from_raw(raw)
print(restored.entity_type)            # 1
print(restored)                        # Post:67890
```

### JavaScript/TypeScript

```typescript
// Note: JavaScript numbers are 53-bit. Use BigInt for full 64-bit support.

class EntityId {
    private static readonly TYPE_SHIFT = 48n;
    private static readonly ID_MASK = 0x0000_FFFF_FFFF_FFFFn;
    
    static readonly USER = 0;
    static readonly POST = 1;
    static readonly COMMENT = 2;
    
    readonly value: bigint;
    
    constructor(entityType: number, rawId: bigint) {
        this.value = (BigInt(entityType) << EntityId.TYPE_SHIFT) | (rawId & EntityId.ID_MASK);
    }
    
    static fromRaw(value: bigint): EntityId {
        const id = Object.create(EntityId.prototype);
        id.value = value;
        return id;
    }
    
    get entityType(): number {
        return Number(this.value >> EntityId.TYPE_SHIFT);
    }
    
    get rawId(): bigint {
        return this.value & EntityId.ID_MASK;
    }
    
    isUser(): boolean {
        return this.entityType === EntityId.USER;
    }
    
    // For JSON serialization (as string to preserve precision)
    toJSON(): string {
        return this.value.toString();
    }
    
    static fromJSON(s: string): EntityId {
        return EntityId.fromRaw(BigInt(s));
    }
}

// Usage
const userId = new EntityId(EntityId.USER, 12345n);
const postId = new EntityId(EntityId.POST, 67890n);

const raw = postId.value;              // 281474976778570n
const restored = EntityId.fromRaw(raw);
console.log(restored.entityType);      // 1
```

## API Enforcement

### Recommended: Enforce Typed IDs

The gRPC/REST API should enforce that IDs include proper type information:

```protobuf
// Proto definition
message EntityId {
    uint64 value = 1;  // Full 64-bit typed ID
}

message AddEdgeRequest {
    EntityId src = 1;   // Must be properly typed
    uint32 edge_type = 2;
    EntityId dst = 3;   // Must be properly typed
}
```

**Server-side validation:**

```rust
fn validate_entity_id(id: u64, expected_type: Option<EntityType>) -> Result<EntityId, Error> {
    let entity_id = EntityId::from_raw(id);
    
    // Check for untyped IDs (raw values without type bits)
    if id != 0 && id < (1u64 << 48) && entity_id.entity_type() == EntityType::User {
        // Warning: This might be an untyped ID being treated as User
        // Consider: return Err(Error::UntypedId)
    }
    
    if let Some(expected) = expected_type {
        if entity_id.entity_type() != expected {
            return Err(Error::TypeMismatch {
                expected,
                actual: entity_id.entity_type(),
            });
        }
    }
    
    Ok(entity_id)
}
```

### Migration from Untyped IDs

If migrating from a system with plain integer IDs:

```rust
// Convert legacy ID to typed ID
fn migrate_legacy_id(legacy_id: u64, entity_type: EntityType) -> EntityId {
    EntityId::new(entity_type, legacy_id)
}

// Batch migration
fn migrate_user_ids(legacy_ids: &[u64]) -> Vec<EntityId> {
    legacy_ids.iter()
        .map(|&id| EntityId::new(EntityType::User, id))
        .collect()
}
```

## Compression Implications

The typed ID system works well with delta+varint compression:

### Same-Type Lists (Optimal)

```
User followers: [User:1000, User:1001, User:1002, User:1005]
Raw values:     [1000, 1001, 1002, 1005]  (type bits are 0)
Deltas:         [1000, 1, 1, 3]           (small deltas = good compression)
```

### Mixed-Type Lists (Less Optimal)

```
Liked entities: [Post:100, Photo:200, Video:50]
Raw values:     [281474976710756, 844424930132168, 1125899906842674]
Deltas:         [281474976710756, 562949953421412, 281474976710506]
                 (large deltas = poor compression)
```

**Recommendation:** Store edges by type when possible for better compression.

## Client ID Generation Patterns

There are two approaches to generating IDs, depending on your architecture:

### 1. Local Generation (Recommended for Most Cases)

Use local `IdGenerator` instances when:
- You need high throughput without network latency
- Your services can tolerate non-sequential IDs across nodes
- You're building microservices that need to generate IDs independently

**Rust:**
```rust
use aoee_core::{IdGenerator, EntityType};

let gen = IdGenerator::new(EntityType::User);
let id = gen.next_id();  // No network call
```

**Java:**
```java
IdGenerator gen = new IdGenerator(EntityType.USER);
EntityId id = gen.nextId();  // No network call
```

### 2. Server-side Generation (gRPC)

Use server-side generation when:
- You need a central source of truth for IDs
- You want all IDs to be generated by a single authority
- Network latency is acceptable for your use case

**Java Client:**
```java
try (AoeeClient client = new AoeeClient("localhost", 50051)) {
    // Generate a single ID
    EntityId userId = client.generateId(EntityType.USER);
    
    // Generate multiple IDs in batch
    List<EntityId> postIds = client.generateIds(EntityType.POST, 100);
}
```

**gRPC Direct:**
```protobuf
rpc GenerateIds(GenerateIdsRequest) returns (GenerateIdsResponse);

message GenerateIdsRequest {
    uint32 entity_type = 1;  // 0=User, 1=Post, etc.
    uint32 count = 2;        // 1-10000
}
```

### Persistence Layer Integration

When building a persistence layer (like a social media backend), you'll typically:

1. **On entity creation:** Generate a typed ID before persisting
```java
// Creating a new user
IdGenerator userGen = new IdGenerator(EntityType.USER);

public User createUser(String name, String email) {
    EntityId id = userGen.nextId();
    User user = new User(id.getValue(), name, email);
    userRepository.save(user);
    return user;
}
```

2. **On entity retrieval:** Decode the ID to verify type
```java
public User getUser(long rawId) {
    EntityId id = EntityId.fromRaw(rawId);
    if (!id.isUser()) {
        throw new IllegalArgumentException("Not a user ID: " + id);
    }
    return userRepository.findById(rawId);
}
```

3. **On edge creation:** Use typed IDs for both src and dst
```java
public void followUser(long srcUserId, long dstUserId) {
    // Validate both are user IDs
    EntityId src = EntityId.fromRaw(srcUserId);
    EntityId dst = EntityId.fromRaw(dstUserId);
    
    if (!src.isUser() || !dst.isUser()) {
        throw new IllegalArgumentException("Both IDs must be users");
    }
    
    aoeeClient.addEdge(srcUserId, EdgeType.FOLLOWS, dstUserId);
}
```

### ID Generation Best Practices

1. **One generator per type per service instance** - Share generators within a service
2. **Don't share generators across services** - Each service should have its own
3. **Validate types at API boundaries** - Check ID types when receiving from external sources
4. **Store full 64-bit values** - Always persist the complete typed ID, not just raw_id

## Implementation Reference

- **Rust:** `aoee/aoee-core/src/id.rs` - Core EntityId and IdGenerator implementation
- **Java:** `aoee-java-client/src/main/java/com/aoee/client/` - EntityId, EntityType, IdGenerator
- **gRPC:** `aoee/aoee-server/proto/aoee.proto` - GenerateIds RPC
- **Constants:**
  - `TYPE_BITS = 16`
  - `ID_BITS = 48`
  - `ID_MASK = 0x0000_FFFF_FFFF_FFFF`
  - `MAX_RAW_ID = 281,474,976,710,655`

## Summary

| Aspect | Value |
|--------|-------|
| Total bits | 64 |
| Type bits | 16 (high) |
| ID bits | 48 (low) |
| Max types | 65,536 |
| Max IDs per type | ~281 trillion |
| Type extraction | O(1) bit shift |
| Generation | Timestamp + sequence (no coordinator) |
| Epoch | 2020-01-01 00:00:00 UTC |
| Compression | Delta+varint on raw 64-bit values |

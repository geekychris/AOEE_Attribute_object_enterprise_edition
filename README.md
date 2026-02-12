# AOEE - Attribute Object Enterprise Edition
![img.png](doc_images/aoee_logo.png)


A high-performance, TAO-inspired in-memory relationship cache for social graph queries. AOEE provides compressed posting lists with efficient set operations optimized for friend-of-friend queries, news feeds, and social graph traversals.

## Table of Contents

1. [Purpose](#purpose)
2. [Architecture Overview](#architecture-overview)
3. [Data Representation](#data-representation)
4. [Encoding Strategies](#encoding-strategies)
5. [Core Components](#core-components)
6. [Java Client](#java-client)
7. [Spring Boot Proxy](#spring-boot-proxy)
8. [React UI](#react-ui)
9. [Building and Installation](#building-and-installation)
10. [Extending AOEE](#extending-aoee)
11. [Future Features](#future-features)
12. [User Manual](#user-manual)

---

## Purpose

AOEE solves the problem of efficiently querying social graph relationships at scale. Inspired by Facebook's TAO (The Associations and Objects), it provides:

- **Sub-millisecond edge lookups**: O(1) cache access for relationship queries
- **Efficient set operations**: Optimized intersection/union for "mutual friends" queries
- **Friend-of-Friend (2-hop) queries**: Find connection suggestions with scoring
- **Compressed storage**: 4 encoding strategies minimize memory footprint
- **Sharded architecture**: Horizontal scalability via consistent hashing

### Use Cases

- Social networks: followers, friends, likes, shares
- Content platforms: authorship, comments, reactions
- Group/community management: memberships, roles
- Recommendation systems: friend suggestions, content recommendations

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           React UI (:5173)                          │
│                    Graph Explorer, Edge Manager                      │
└─────────────────────────────────────────────────────────────────────┘
                                    │ HTTP/REST
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Spring Boot Proxy (:8080)                       │
│              REST API, Dataset Loader, CORS handling                 │
└─────────────────────────────────────────────────────────────────────┘
                                    │ gRPC
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Rust AOEE Server (:50051)                      │
│                         gRPC Service Layer                           │
├─────────────────────────────────────────────────────────────────────┤
│                          Shard Manager                               │
│                   Consistent Hashing, Routing                        │
├──────────────────┬──────────────────┬──────────────────┬────────────┤
│     Shard 0      │     Shard 1      │     Shard 2      │   ...      │
│   DashMap Cache  │   DashMap Cache  │   DashMap Cache  │            │
├──────────────────┴──────────────────┴──────────────────┴────────────┤
│                         Storage Layer                                │
│                   InMemoryStore / LSM (future)                       │
└─────────────────────────────────────────────────────────────────────┘
```

### Crate Structure (Rust)

| Crate | Description |
|-------|-------------|
| `aoee-core` | Core data structures: EntityId, EdgeKey, PostingList, encoding, set operations, FOF queries |
| `aoee-storage` | Storage abstractions: InMemoryStore, EdgeStore trait |
| `aoee-shard` | Sharding: Shard, ShardManager, consistent hashing |
| `aoee-server` | gRPC service implementation |
| `aoee-client` | Rust gRPC client library |
| `aoee-bench` | Benchmarking utilities |

---

## Data Representation

### Entity IDs

64-bit identifiers with embedded type information:

```
┌────────────────────────────────────────────────────────────────────┐
│  Entity Type (8 bits)  │            Entity Value (56 bits)         │
└────────────────────────────────────────────────────────────────────┘
```

Entity types: User, Post, Comment, Photo, Video, Group, Page, Event, Tag, Custom

### Edge Types

Relationships are typed with forward/reverse pairs:

| Category | Forward | Reverse | Code |
|----------|---------|---------|------|
| Social | FOLLOWS | FOLLOWED_BY | 0, 1 |
| Social | FRIEND_OF | FRIEND_OF | 2 |
| Social | BLOCKS | BLOCKED_BY | 3, 4 |
| Content | LIKES | LIKED_BY | 10, 11 |
| Content | COMMENTS_ON | HAS_COMMENT | 12, 13 |
| Content | SHARES | SHARED_BY | 14, 15 |
| Authorship | AUTHORED | AUTHORED_BY | 20, 21 |
| Groups | MEMBER_OF | HAS_MEMBER | 30, 31 |
| Groups | ADMINISTERS | ADMINISTERED_BY | 32, 33 |
| Containment | CONTAINS | CONTAINED_IN | 40, 41 |
| Tagging | TAGGED_IN | HAS_TAG | 50, 51 |
| Mentions | MENTIONS | MENTIONED_BY | 60, 61 |
| Custom | CUSTOM_1-5 | - | 100-104 |

### Edge Keys

An edge key uniquely identifies a posting list:

```rust
EdgeKey {
    src: EntityId,      // Source entity
    edge_type: EdgeType // Relationship type
}
```

Example: `EdgeKey(User:123, FOLLOWS)` → all users that user 123 follows

### Metadata Support

Certain edge types support a metadata byte per edge:

- **LIKES (10)**: Reaction type
  - 0 = 👍 Like
  - 1 = ❤️ Love
  - 2 = 😂 Haha
  - 3 = 😮 Wow
  - 4 = 😢 Sad
  - 5 = 😠 Angry

---

## Encoding Strategies

AOEE automatically selects the optimal encoding based on list characteristics:

### 1. SmallVec (< 128 elements)
Plain `Vec<u64>` - simple, fast for small lists.

### 2. DeltaVarint (128 - 4,096 elements)
- Delta encoding: store differences between consecutive sorted IDs
- LEB128 variable-length integers: small deltas use fewer bytes
- Typical compression: 2-4 bytes per ID

### 3. BlockPacked (> 4,096 elements)
- Fixed-size blocks (128 IDs)
- Bit-packed within blocks
- Good balance of compression and random access

### 4. Roaring Bitmaps (> 100K or dense)
- For huge or dense lists
- Excellent compression for clustered IDs
- Fast set operations

### Strategy Selection

```rust
fn select_encoding(ids: &[EntityId]) -> EncodingStrategy {
    let len = ids.len();
    if len < 128 { SmallVec }
    else if len < 4096 { DeltaVarint }
    else if is_dense(ids) || len > 100_000 { Roaring }
    else { BlockPacked }
}
```

---

## Core Components

### Posting List

The fundamental data structure storing edges for a key:

```rust
PostingList {
    buffer: WriteBuffer,      // Recent writes (unsorted)
    segments: Vec<Segment>,   // Compacted, sorted, encoded
    last_modified: u64,       // Timestamp
    total_count: u64,         // Approximate count
}
```

### Write Buffer

LSM-style buffer for recent writes:

```rust
BufferEntry {
    dst: EntityId,      // Destination
    timestamp: u64,     // Nanoseconds since epoch
    tombstone: bool,    // Delete marker
    metadata: u8,       // Optional metadata (reactions)
}
```

### Segments

Immutable, compressed, sorted data:

```rust
Segment {
    data: EncodedList,  // Compressed IDs
    min_id: EntityId,   // Range for bloom filter
    max_id: EntityId,
    count: u32,
    level: u8,          // Compaction level
}
```

### Compaction

Background process merging buffers into segments:

1. Buffer fills to threshold (default: 1000 entries)
2. Entries are sorted and deduplicated
3. Tombstones cancel out additions
4. New segment created with optimal encoding
5. Small segments merge into larger ones (leveled compaction)

---

## Java Client

### Installation

```bash
cd aoee-java-client
mvn install
```

### Usage

```java
// Connect to AOEE server
AoeeClient client = new AoeeClient("localhost", 50051);

// Add edges
long timestamp = client.addEdge(userId, EdgeType.FOLLOWS, targetId);

// Add edge with metadata (reactions)
client.addEdge(userId, EdgeType.LIKES, postId, 0, ReactionType.LOVE.getValue());

// Query neighbors
List<Long> following = client.getNeighbors(userId, EdgeType.FOLLOWS);

// Query with metadata
NeighborsResult result = client.getNeighborsWithMetadata(
    userId, EdgeType.LIKES, 100, true
);
for (int i = 0; i < result.neighbors().size(); i++) {
    long postId = result.neighbors().get(i);
    int reaction = result.metadata().get(i);
    System.out.println("User liked post " + postId + " with " + 
        ReactionType.fromValue(reaction));
}

// Check existence
boolean follows = client.contains(userId, EdgeType.FOLLOWS, targetId);

// Count edges
long followerCount = client.count(userId, EdgeType.FOLLOWED_BY);

// Set operations
List<Long> mutualFriends = client.intersect(
    new EdgeKey(user1, EdgeType.FRIEND_OF),
    new EdgeKey(user2, EdgeType.FRIEND_OF)
);

// Friend-of-Friend suggestions
FofResponse fof = client.friendOfFriend(userId, EdgeType.FRIEND_OF);
for (FofCandidate candidate : fof.getCandidatesList()) {
    System.out.println("Suggestion: " + candidate.getId() + 
        " (score: " + candidate.getScore() + ")");
}

// Clean up
client.close();
```

### Edge Type Constants

```java
public class EdgeType {
    public static final int FOLLOWS = 0;
    public static final int FOLLOWED_BY = 1;
    public static final int FRIEND_OF = 2;
    public static final int LIKES = 10;
    public static final int AUTHORED = 20;
    public static final int MEMBER_OF = 30;
    public static final int TAGGED_IN = 50;
    // ... etc
}
```

---

## Spring Boot Proxy

REST proxy providing HTTP access to the gRPC server with CORS support.

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/edges` | Add an edge |
| DELETE | `/api/edges` | Delete an edge |
| GET | `/api/edges/{src}/{edgeType}` | Get neighbors |
| GET | `/api/edges/{src}/{edgeType}/contains/{dst}` | Check edge exists |
| GET | `/api/edges/{src}/{edgeType}/count` | Count edges |
| POST | `/api/set/intersect` | Intersect two edge lists |
| POST | `/api/set/union` | Union two edge lists |
| POST | `/api/fof/{src}/{edgeType}` | Friend-of-Friend query |
| GET | `/api/stats` | Server statistics |
| POST | `/api/dataset/load` | Load dataset from text |
| POST | `/api/dataset/preview` | Preview dataset (validate) |
| GET | `/api/dataset/sample` | Get sample dataset |
| GET | `/api/health` | Health check |

### Example Requests

```bash
# Add an edge with reaction
curl -X POST http://localhost:8080/api/edges \
  -H "Content-Type: application/json" \
  -d '{"src": 1, "edgeType": "LIKES", "dst": 1001, "metadata": 1}'

# Get neighbors with metadata
curl "http://localhost:8080/api/edges/1/LIKES?includeMetadata=true"

# Friend-of-Friend suggestions
curl -X POST http://localhost:8080/api/fof/1/FOLLOWS \
  -H "Content-Type: application/json" \
  -d '{"maxResults": 10, "minScore": 2}'
```

---

## React UI

Web interface for exploring and managing the graph.

### Components

1. **Dashboard**: Server health, statistics, quick actions
2. **Graph Explorer**: Visual network graph with force-directed layout
3. **Edge Manager**: Add/delete edges, query neighbors
4. **Query Builder**: Set operations, FOF queries
5. **Dataset Loader**: Load sample or custom datasets

### Features

- Interactive force-directed graph visualization
- Multi-edge-type selection with color coding
- Auto-zoom to fit graph on load
- Reaction emoji picker for LIKES edges
- Real-time edge type filtering
- Resizable graph panel

---

## Building and Installation

### Prerequisites

- Rust 1.75+ (for AOEE server)
- Java 21+ (for Java client and Spring Boot)
- Node.js 18+ (for React UI)
- Maven 3.8+ (for Java projects)

### Build Steps

```bash
# 1. Build Rust server
cd aoee
cargo build --release

# 2. Build Java client
cd ../aoee-java-client
mvn clean install

# 3. Build Spring Boot proxy
cd ../aoee-spring
mvn clean package

# 4. Build React UI
cd ../aoee-ui
npm install
npm run build
```

### Running Services

**Option 1: Use the start script**

```bash
./start-services.sh
```

**Option 2: Start manually**

```bash
# Terminal 1: Rust server
cd aoee && ./target/release/aoee-server

# Terminal 2: Spring Boot
cd aoee-spring && mvn spring-boot:run

# Terminal 3: React UI
cd aoee-ui && python3 -m http.server 5173 --directory dist
```

### Accessing the UI

Open http://localhost:5173 in your browser.

---

## Extending AOEE

### Adding New Edge Types

1. Add to `aoee-core/src/types.rs`:

```rust
pub enum EdgeType {
    // ... existing types
    MyNewType = 70,
    MyNewTypeReverse = 71,
}

impl EdgeType {
    pub fn reverse(self) -> Option<EdgeType> {
        match self {
            // ... existing mappings
            EdgeType::MyNewType => Some(EdgeType::MyNewTypeReverse),
            EdgeType::MyNewTypeReverse => Some(EdgeType::MyNewType),
        }
    }
    
    pub fn from_raw(value: u16) -> Option<Self> {
        match value {
            // ... existing mappings
            70 => Some(EdgeType::MyNewType),
            71 => Some(EdgeType::MyNewTypeReverse),
        }
    }
}
```

2. Add to Java client `EdgeType.java`:

```java
public static final int MY_NEW_TYPE = 70;
public static final int MY_NEW_TYPE_REVERSE = 71;
```

3. Add to React UI `types/index.ts`:

```typescript
export const EDGE_TYPES = [
  // ... existing types
  'MY_NEW_TYPE',
];
```

### Adding Metadata to Edge Types

1. Add to `METADATA_EDGE_TYPES` in `aoee-core/src/types.rs`:

```rust
pub const METADATA_EDGE_TYPES: &[EdgeType] = &[
    EdgeType::Likes,
    EdgeType::MyNewType,  // Add here
];
```

2. Update `DatasetService.java`:

```java
private static final Set<String> METADATA_EDGE_TYPES = Set.of(
    "LIKES",
    "MY_NEW_TYPE"  // Add here
);
```

### Adding Custom Storage Backend

Implement the `EdgeStore` trait:

```rust
pub trait EdgeStore: Send + Sync {
    fn persist_edge(&self, key: EdgeKey, dst: EntityId, ts: u64) 
        -> impl Future<Output = Result<(), StorageError>>;
    fn persist_delete(&self, key: EdgeKey, dst: EntityId, ts: u64) 
        -> impl Future<Output = Result<(), StorageError>>;
    fn load_posting_list(&self, key: EdgeKey) 
        -> impl Future<Output = Result<Option<PostingList>, StorageError>>;
}
```

---

## Future Features

### Planned Enhancements

1. **Persistent Storage**: RocksDB-backed LSM tree for durability
2. **Replication**: Leader-follower replication for high availability
3. **Distributed Queries**: Cross-node set operations
4. **Streaming Updates**: WebSocket/SSE for real-time graph updates
5. **Query Caching**: LRU cache for frequent FOF queries
6. **Batch Operations**: Bulk insert/delete APIs
7. **Graph Analytics**: PageRank, community detection
8. **Time-Travel Queries**: Query graph state at historical timestamps

### Performance Optimizations

1. **SIMD Acceleration**: Vectorized set operations
2. **Memory-Mapped Files**: Reduce GC pressure for large datasets
3. **Connection Pooling**: gRPC channel management
4. **Adaptive Encoding**: Runtime strategy switching based on access patterns

---

## User Manual

### Getting Started with the React UI

#### 1. Load the Sample Dataset

1. Navigate to **Dataset Loader** in the sidebar
2. Click **"Load Sample Dataset"** to populate the text area
3. Click **"Load Dataset"** to import into AOEE
4. You should see: "24 Entities, 85 Edges Loaded"

#### 2. Explore the Graph

1. Navigate to **Graph Explorer**
2. Set **Root Entity ID** to `1` (Alice)
3. Select edge types: `FOLLOWS`, `FRIEND_OF`, `LIKES`
4. Set **Depth** to `2`
5. Enable **"Show Edge Labels"**
6. Click **"Load Graph"**
7. The graph will render and auto-zoom to fit

**Tips:**
- Click any node to expand its connections
- Toggle edge types to filter the view
- Drag the bottom edge to resize the graph panel

#### 3. Manage Edges

1. Navigate to **Edge Manager**
2. **Add an edge:**
   - Source ID: `1`, Edge Type: `LIKES`, Destination: `1003`
   - For LIKES, select a reaction emoji (e.g., ❤️)
   - Click **"Add Edge"**
3. **Query neighbors:**
   - Source ID: `1`, Edge Type: `LIKES`
   - Click **"Get Neighbors"**
   - You'll see post IDs with reaction emojis

#### 4. Friend-of-Friend Queries

1. Navigate to **Query Builder**
2. Select **"Friend of Friend"** query type
3. Enter Source ID: `1` (Alice)
4. Edge Type: `FOLLOWS`
5. Click **"Execute"**

**Interpreting Results:**
- Candidates are ranked by **score** (mutual connections)
- Higher score = more friends in common
- Direct friends are excluded by default

### Example Queries with the Sample Dataset

#### Find Alice's Friend Suggestions

```
Source: 1, Edge Type: FOLLOWS
```

Results show users followed by people Alice follows, ranked by how many mutual follows they have.

#### Find Who Liked Alice's First Post

```
Source ID: 1001, Edge Type: LIKED_BY
```

Note: This requires the reverse edge type to be indexed.

#### Find Mutual Friends Between Alice and Bob

```
Intersect:
  Key 1: src=1, type=FRIEND_OF
  Key 2: src=2, type=FRIEND_OF
```

Results: IDs of users who are friends with both Alice and Bob.

#### Find All Developers Who Are Also Hikers

```
Intersect:
  Key 1: src=2001, type=HAS_MEMBER (developers group)
  Key 2: src=2003, type=HAS_MEMBER (hikers group)
```

Results: User IDs who are in both groups.

### Dataset Format Reference

```
# Comments start with #

# Define entities (optional, for documentation)
ENTITY <type> <id> <name>

# Define edges
<EDGE_TYPE> <src_id> <dst_id> [metadata]

# Edge types (case-insensitive):
FOLLOWS, FRIEND, LIKES, BLOCKS, MEMBER, AUTHORED, TAGGED

# Metadata (for LIKES only):
LIKES <src> <dst> like|love|haha|wow|sad|angry
```

### Troubleshooting

**UI shows "Failed to connect":**
- Ensure all services are running: `./start-services.sh`
- Check logs in `/logs/` directory

**Graph is empty after loading:**
- Verify dataset loaded successfully (check load result message)
- Ensure you selected at least one edge type

**FOF returns no results:**
- The source user needs connections first
- Try with user `1` (Alice) who has many connections

---

## License

MIT OR Apache-2.0

## Contributors

AOEE Contributors

Co-Authored-By: Warp <agent@warp.dev>

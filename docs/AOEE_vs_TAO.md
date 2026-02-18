# AOEE vs Facebook TAO: Architecture Comparison

This document compares AOEE (Attribute Object Enterprise Edition) with Facebook's TAO (The Associations and Objects), the distributed graph data store that inspired AOEE's design.

## Overview

### What is TAO?

TAO is Facebook's distributed data store for the social graph, serving billions of reads per second across Facebook's products. It was designed to solve the impedance mismatch between the social graph and the underlying MySQL storage, providing:

- Low-latency access to graph edges (associations)
- Efficient fan-out for social features (news feed, friend lists, likes)
- Strong consistency for writes with eventual consistency for reads
- Massive horizontal scalability

TAO was first described in the 2013 USENIX paper "TAO: Facebook's Distributed Data Store for the Social Graph."

### What is AOEE?

AOEE is an in-memory relationship cache inspired by TAO's data model and access patterns. It provides:

- Sub-millisecond latency for graph queries
- Memory-efficient edge storage using advanced encoding
- Friend-of-friend (2-hop) queries with scoring
- Timestamp and metadata support for edges

AOEE is designed as a caching layer that can sit in front of persistent storage, similar to how TAO caches MySQL data.

## Core Concepts Comparison

### Data Model

| Concept | TAO | AOEE |
|---------|-----|------|
| **Nodes** | Objects with id, type, and data | EntityId (type + numeric id) |
| **Edges** | Associations with id1, atype, id2, time, data | Edges with subject, edge_type, object, timestamp, metadata |
| **Edge ordering** | By timestamp (most recent first) | By object id (sorted) |
| **Edge data** | Arbitrary serialized data | 1-byte metadata (optional) |

**TAO Objects:**
```
Object {
  id: 64-bit integer
  otype: object type
  data: serialized blob
}
```

**AOEE Entities:**
```
EntityId {
  entity_type: u8 (0-255 types)
  id: u64
}
```

**TAO Associations:**
```
Association {
  id1: source object
  atype: association type
  id2: destination object
  time: 32-bit timestamp
  data: serialized blob
}
```

**AOEE Edges:**
```
Edge {
  subject: EntityId
  edge_type: EdgeType enum
  object: EntityId
  timestamp: u64 (nanoseconds)
  metadata: Option<u8>
}
```

### Access Patterns

Both systems optimize for similar read patterns:

| Operation | TAO | AOEE |
|-----------|-----|------|
| **Get neighbors** | `assoc_get(id1, atype)` | `neighbors(subject, edge_type)` |
| **Check edge exists** | `assoc_get(id1, atype, id2s)` | `contains(subject, edge_type, object)` |
| **Count edges** | `assoc_count(id1, atype)` | `count(subject, edge_type)` |
| **Range query** | `assoc_range(id1, atype, pos, limit)` | `neighbors` with limit parameter |
| **Time range** | `assoc_time_range(id1, atype, high, low, limit)` | Not directly supported |

### Unique to AOEE

AOEE adds operations not present in TAO:

- **Set intersection**: `intersection(id1, id2, edge_type)` - find mutual friends/common interests
- **Friend-of-friend**: `friend_of_friend(id, edge_type, limit, score_threshold)` - 2-hop traversal with scoring
- **Metadata filtering**: Query edges by metadata value (e.g., specific reaction types)

## Architecture Comparison

### TAO Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Web Servers                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    TAO Leader Cache                          │
│              (one per shard, handles writes)                 │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  TAO Follower   │ │  TAO Follower   │ │  TAO Follower   │
│     Cache       │ │     Cache       │ │     Cache       │
└─────────────────┘ └─────────────────┘ └─────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    MySQL Database                            │
│                  (persistent storage)                        │
└─────────────────────────────────────────────────────────────┘
```

**Key characteristics:**
- Multi-tier caching with leader/follower topology
- Leader handles writes, followers handle reads
- Eventual consistency between leader and followers
- MySQL as the source of truth
- Sharded by object id

### AOEE Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                          │
│              (React UI / REST clients)                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Spring Boot Proxy                           │
│              (REST API, dataset loading)                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Java gRPC Client                           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  AOEE Rust Server                            │
│    ┌─────────────────────────────────────────────────┐      │
│    │              Shard Manager                       │      │
│    │  ┌─────────┐ ┌─────────┐ ┌─────────┐           │      │
│    │  │ Shard 0 │ │ Shard 1 │ │ Shard N │           │      │
│    │  └─────────┘ └─────────┘ └─────────┘           │      │
│    └─────────────────────────────────────────────────┘      │
│    ┌─────────────────────────────────────────────────┐      │
│    │           Posting Lists (per edge type)          │      │
│    │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │      │
│    │  │ Buffer   │ │ Segments │ │ Roaring  │        │      │
│    │  │ (recent) │ │ (frozen) │ │ (dense)  │        │      │
│    │  └──────────┘ └──────────┘ └──────────┘        │      │
│    └─────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

**Key characteristics:**
- Single-tier in-memory cache
- All operations handled by single server process
- Strong consistency (single writer)
- Optional persistence via snapshots
- Sharded by subject entity id

## Key Similarities

### 1. Edge-Centric Data Model

Both systems model the social graph as directed, typed edges between entities:
- Edges have a source, type, and destination
- Edges are grouped by (source, type) for efficient retrieval
- Edge lists are the primary unit of caching

### 2. Optimized for Read-Heavy Workloads

Both assume reads vastly outnumber writes:
- TAO: 99.8% reads in production
- AOEE: Designed for query-heavy social features

### 3. Sorted Edge Lists

Both maintain edges in sorted order:
- TAO: Sorted by timestamp (most recent first)
- AOEE: Sorted by object id (enables binary search, intersection)

### 4. Association Types

Both use typed edges to represent different relationship semantics:
- TAO: Numeric association types (atypes)
- AOEE: Enum-based edge types (FOLLOWS, FRIEND_OF, LIKES, MEMBER_OF)

### 5. Timestamp Support

Both track when edges were created:
- TAO: 32-bit Unix timestamp
- AOEE: 64-bit nanosecond timestamp

## Key Differences

### 1. Persistence Model

| Aspect | TAO | AOEE |
|--------|-----|------|
| Primary storage | MySQL | In-memory |
| Persistence | Database is source of truth | Optional snapshots |
| Recovery | Reload from MySQL | Reload from snapshot/dataset |
| Durability | Strong (MySQL replication) | Configurable |

**TAO** treats the cache as ephemeral—if cache is lost, data is reconstructed from MySQL.

**AOEE** is primarily an in-memory system with optional persistence. It's designed for scenarios where:
- Data can be reconstructed from an upstream system
- Dataset fits in memory
- Ultra-low latency is more important than durability

### 2. Consistency Model

| Aspect | TAO | AOEE |
|--------|-----|------|
| Write consistency | Strong (via leader) | Strong (single process) |
| Read consistency | Eventual (follower lag) | Strong |
| Cross-shard | Eventually consistent | N/A (single process) |

**TAO** uses a leader/follower model where:
- Writes go to the leader cache
- Followers asynchronously receive updates
- Reads from followers may see stale data

**AOEE** provides strong consistency because:
- Single process handles all reads and writes
- No replication lag
- Trade-off: Limited to single-machine scale

### 3. Distribution and Scale

| Aspect | TAO | AOEE |
|--------|-----|------|
| Deployment | Geo-distributed data centers | Single machine |
| Sharding | Distributed across machines | In-process sharding |
| Scale | Billions of objects | Millions of objects |
| Replication | Multi-region | None (currently) |

**TAO** is designed for Facebook-scale:
- Thousands of cache servers
- Multiple geographic regions
- Billions of reads per second

**AOEE** is designed for smaller deployments:
- Single server (multi-core)
- In-memory dataset
- Millions of edges

### 4. Edge Ordering

| Aspect | TAO | AOEE |
|--------|-----|------|
| Primary order | By timestamp (descending) | By object id (ascending) |
| Use case | "Most recent N" queries | Set operations, binary search |
| Trade-off | Temporal locality | Intersection efficiency |

**TAO's timestamp ordering** optimizes for:
- "Show latest 10 comments"
- "Recent activity feed"
- Time-range queries

**AOEE's id ordering** optimizes for:
- `contains()` via binary search: O(log n)
- `intersection()` via merge: O(n + m)
- Friend-of-friend scoring

### 5. Edge Metadata

| Aspect | TAO | AOEE |
|--------|-----|------|
| Data per edge | Arbitrary blob | 1 byte (optional) |
| Flexibility | High (any serialized data) | Low (256 values max) |
| Memory efficiency | Lower | Higher |

**TAO** allows arbitrary data on edges, useful for:
- Edge-specific attributes
- Denormalized data
- Custom metadata

**AOEE** uses a single byte for metadata:
- Reaction types (like, love, haha, etc.)
- Edge categories
- Extremely memory-efficient

### 6. Encoding Strategies

| Aspect | TAO | AOEE |
|--------|-----|------|
| Storage format | Serialized objects | Compressed posting lists |
| Compression | Application-level | Delta-varint, Roaring bitmaps |
| Memory efficiency | Moderate | High |

**AOEE** uses multiple encoding strategies based on list characteristics:
- **SmallVec**: For lists < 8 elements (inline storage)
- **Delta-Varint**: For sparse lists (variable-length encoding)
- **Block-Packed**: For medium-density lists
- **Roaring Bitmaps**: For dense lists (bitmap compression)

### 7. Query Capabilities

| Operation | TAO | AOEE |
|-----------|-----|------|
| Get neighbors | ✓ | ✓ |
| Count | ✓ | ✓ |
| Contains | ✓ | ✓ |
| Range by time | ✓ | ✗ |
| Range by position | ✓ | ✓ |
| Set intersection | ✗ | ✓ |
| Friend-of-friend | ✗ | ✓ |
| Scored recommendations | ✗ | ✓ |

**AOEE adds** graph-aware operations that TAO delegates to application code:
- Mutual friend computation
- 2-hop friend recommendations with scoring
- Efficient set operations on edge lists

## Design Philosophy

### TAO: "Cache as a Service"

TAO's philosophy is to be a transparent caching layer:
- Clients don't know if data came from cache or database
- Cache misses automatically fill from MySQL
- Focus on consistency and availability
- Scale through distributed caching

### AOEE: "In-Memory Graph Engine"

AOEE's philosophy is to be a specialized query engine:
- All data lives in memory
- Focus on query expressiveness
- Graph-specific operations (intersection, FoF)
- Scale through memory efficiency

## When to Use Each

### Use TAO (or TAO-like systems) when:

- You need persistent, durable storage
- Dataset exceeds single-machine memory
- Geographic distribution is required
- You have existing MySQL infrastructure
- Read consistency can be eventual

### Use AOEE when:

- Ultra-low latency is critical (< 100µs)
- Dataset fits in memory (millions of edges)
- You need graph-specific operations (FoF, intersection)
- Strong read consistency is required
- You want a simpler operational model

## Summary

AOEE takes inspiration from TAO's core insight—that social graphs are best served by edge-centric data structures with typed associations—but makes different trade-offs:

| Trade-off | TAO | AOEE |
|-----------|-----|------|
| Scale vs. Latency | Scale | Latency |
| Persistence vs. Simplicity | Persistence | Simplicity |
| Flexibility vs. Efficiency | Flexibility | Efficiency |
| Distribution vs. Consistency | Distribution | Consistency |

AOEE can be thought of as "TAO for a single machine"—capturing the essence of TAO's data model while optimizing for scenarios where the entire working set fits in memory and sub-millisecond latency is paramount.

## References

1. "TAO: Facebook's Distributed Data Store for the Social Graph" - USENIX ATC 2013
2. Facebook Engineering Blog: "TAO: The power of the graph"
3. "Scaling Memcache at Facebook" - NSDI 2013 (predecessor to TAO)

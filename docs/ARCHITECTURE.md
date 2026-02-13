# AOEE Architecture

AOEE (Attribute Object Enterprise Edition) is a TAO-inspired distributed graph cache system with pluggable persistence backends.

## System Overview

```
                                    ┌─────────────────────────────────────────────────────┐
                                    │                   AOEE System                        │
                                    │                                                     │
┌─────────────┐                     │  ┌─────────────────┐     ┌──────────────────────┐  │
│  React UI   │─────────────────────┼─▶│  aoee-spring    │────▶│   AOEE Rust Server   │  │
│  (port 5173)│                     │  │  (port 8080)    │gRPC │    (port 50051)      │  │
└─────────────┘                     │  │  REST/GraphQL   │     │                      │  │
                                    │  └─────────────────┘     │  ┌──────────────────┐│  │
┌─────────────┐                     │                          │  │  Shard Manager   ││  │
│   Clients   │────gRPC─────────────┼──────────────────────────┼─▶│  (4 shards)      ││  │
│  (Java/.NET)│                     │                          │  └────────┬─────────┘│  │
└─────────────┘                     │                          │           │          │  │
                                    │                          │  ┌────────▼─────────┐│  │
                                    │                          │  │    LRU Cache     ││  │
                                    │                          │  │  (per shard)     ││  │
                                    │                          │  └────────┬─────────┘│  │
                                    │                          │           │          │  │
                                    │                          │  ┌────────▼─────────┐│  │
                                    │                          │  │  Storage Backend ││  │
                                    │                          │  │ (write-through)  ││  │
                                    │                          │  └────────┬─────────┘│  │
                                    │                          └───────────┼──────────┘  │
                                    └──────────────────────────────────────┼──────────────┘
                                                                           │
                           ┌───────────────────────────────────────────────┼──────────────┐
                           │                                               ▼              │
                           │  ┌────────────────┐     ┌─────────────────────────────────┐ │
                           │  │    RocksDB     │     │      aoee-persistence           │ │
                           │  │  (local disk)  │     │       (port 9081)               │ │
                           │  │                │     │                                 │ │
                           │  │ Option A:      │     │  Option B:                      │ │
                           │  │ Embedded       │     │  HTTP REST API                  │ │
                           │  │ key-value      │     │  ┌─────────────────────────┐    │ │
                           │  │ store          │     │  │   H2 / PostgreSQL       │    │ │
                           │  └────────────────┘     │  └─────────────────────────┘    │ │
                           │                         └─────────────────────────────────┘ │
                           │                         Persistence Backends                 │
                           └──────────────────────────────────────────────────────────────┘
```

## Components

### 1. AOEE Rust Server (`aoee-server`)

The core graph cache engine written in Rust, providing:

- **gRPC API** - High-performance edge operations
- **Sharded Architecture** - 4 shards with consistent hashing
- **LRU Cache** - Per-shard caching with configurable limits
- **Write-through Persistence** - Automatic sync to backend storage
- **Cache Management** - Flush, clear, and eviction controls

**Configuration (Environment Variables):**
```bash
AOEE_STORAGE_TYPE=memory|rocksdb|http  # Storage backend (default: memory)
AOEE_WRITE_THROUGH=true|false          # Enable write-through (default: true)
AOEE_ROCKSDB_PATH=./data/aoee-rocksdb  # RocksDB path
AOEE_HTTP_URL=http://localhost:9081    # HTTP backend URL
AOEE_LISTEN_ADDR=[::1]:50051           # gRPC listen address
```

### 2. Persistence Service (`aoee-persistence`)

Java Spring Boot service providing durable storage:

- **REST API** - CRUD operations for edges and entities
- **GraphQL API** - Flexible querying with GraphiQL IDE
- **Database Support** - H2 (dev) or PostgreSQL (production)
- **Import/Export** - Dataset format support

### 3. Spring Boot Proxy (`aoee-spring`)

Optional REST/WebSocket gateway:

- **REST API** - HTTP interface to AOEE
- **WebSocket** - Real-time updates
- **Benchmarking** - Built-in load testing tools

### 4. React UI (`aoee-ui`)

Web-based visualization and administration:

- **Graph Visualization** - Interactive network view
- **Statistics Dashboard** - Cache metrics and performance
- **Query Interface** - Ad-hoc edge queries

## Storage Backends

### In-Memory (Default)
```bash
AOEE_STORAGE_TYPE=memory
```
- Fastest option, no durability
- Good for development and testing

### RocksDB
```bash
AOEE_STORAGE_TYPE=rocksdb
AOEE_ROCKSDB_PATH=./data/aoee-rocksdb
```
- Embedded key-value store
- Local persistence without external service
- Good for single-node deployments

### HTTP Backend
```bash
AOEE_STORAGE_TYPE=http
AOEE_HTTP_URL=http://localhost:9081
```
- Delegates to aoee-persistence service
- SQL database backing (H2/PostgreSQL)
- Supports multiple AOEE instances

## Data Flow

### Write Path (with write-through)
```
1. Client sends AddEdge(src=1, type=FOLLOWS, dst=2)
2. AOEE routes to shard based on hash(src, type)
3. Shard updates in-memory posting list
4. If write-through enabled:
   - HTTP backend: POST /api/v1/edges
   - RocksDB: Direct put to DB
5. Response returned to client
```

### Read Path (cache miss)
```
1. Client sends Neighbors(src=1, type=FOLLOWS)
2. AOEE routes to appropriate shard
3. Check LRU cache for posting list
4. If cache miss:
   - Load from storage backend
   - Populate cache
   - Update LRU access time
5. Return neighbors to client
```

## Cache Management

### LRU Eviction
- Automatic eviction when cache exceeds `max_entries`
- Time-based eviction with configurable TTL
- Memory-based eviction limits

### Cache Configuration
```rust
cache:
  max_entries: 100000        # Maximum cached posting lists
  max_memory_bytes: 0        # Memory limit (0 = unlimited)
  eviction_target_ratio: 0.9 # Evict to 90% capacity
  min_entries: 1000          # Minimum entries to keep
```

### gRPC Cache Operations
```bash
# Clear all caches
grpcurl -plaintext -d '{}' localhost:50051 aoee.Aoee/ClearCache

# Flush and optionally clear
grpcurl -plaintext -d '{"clear_after_flush": true}' localhost:50051 aoee.Aoee/FlushCache

# Get cache statistics
grpcurl -plaintext -d '{"per_shard": true}' localhost:50051 aoee.Aoee/Stats
```

## Edge Types

| Type | Value | Description |
|------|-------|-------------|
| FOLLOWS | 0 | User follows another user |
| FOLLOWED_BY | 1 | Reverse of follows |
| FRIEND_OF | 2 | Bidirectional friendship |
| BLOCKS | 3 | User blocks another |
| LIKES | 10 | User likes content |
| LIKED_BY | 11 | Content is liked by users |
| COMMENTS_ON | 12 | User comments on content |
| AUTHORED | 20 | User created content |
| MEMBER_OF | 30 | User is member of group |

### Reaction Metadata (for LIKES)
| Value | Reaction |
|-------|----------|
| 0 | Like |
| 1 | Love |
| 2 | Haha |
| 3 | Wow |
| 4 | Sad |
| 5 | Angry |

## Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| Cache hit | ~70µs | In-memory lookup |
| Cache miss (RocksDB) | ~500µs | Local disk read |
| Cache miss (HTTP) | ~1-5ms | Network + DB query |
| Add edge (write-through) | ~200µs | Async persistence |
| Intersect | ~100µs | Merge sorted lists |

## Deployment Modes

### Development (In-Memory)
```bash
./target/release/aoee-server
# or
AOEE_STORAGE_TYPE=memory ./target/release/aoee-server
```

### Single Node (RocksDB)
```bash
AOEE_STORAGE_TYPE=rocksdb \
AOEE_WRITE_THROUGH=true \
./target/release/aoee-server
```

### Distributed (HTTP Backend)
```bash
# Start persistence service
cd aoee-persistence
mvn spring-boot:run -Dspring-boot.run.arguments="--server.port=9081"

# Start AOEE with HTTP backend
AOEE_STORAGE_TYPE=http \
AOEE_HTTP_URL=http://localhost:9081 \
./target/release/aoee-server
```

## Monitoring

### Metrics Available
- `cached_lists` - Number of posting lists in cache
- `cache_hits` / `cache_misses` - Cache efficiency
- `cache_evictions` - LRU evictions count
- `cache_memory_bytes` - Estimated memory usage
- `reads` / `writes` - Operation counts

### Health Checks
```bash
# AOEE gRPC (via grpcurl)
grpcurl -plaintext localhost:50051 aoee.Aoee/Stats

# Persistence REST
curl http://localhost:9081/actuator/health
```

## See Also

- [PERSISTENCE.md](PERSISTENCE.md) - Detailed persistence API docs
- [BENCHMARKING.md](BENCHMARKING.md) - Performance testing guide
- [AOEE_vs_TAO.md](AOEE_vs_TAO.md) - Comparison with Facebook TAO

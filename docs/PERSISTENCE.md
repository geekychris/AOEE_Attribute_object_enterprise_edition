# AOEE Persistence Service

The persistence service provides durable storage for AOEE edges, enabling write-through caching and cache warming from a database.

## Overview

```
┌─────────────┐     ┌─────────────────┐     ┌──────────────┐
│  React UI   │────▶│  aoee-spring    │────▶│ AOEE Rust    │
│  (port 5173)│     │  (port 8080)    │     │ (port 50051) │
└─────────────┘     └────────┬────────┘     └──────────────┘
                             │ write-through
                             ▼
                    ┌─────────────────┐
                    │aoee-persistence │
                    │  (port 8081)    │
                    │  REST + GraphQL │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  H2 / PostgreSQL│
                    └─────────────────┘
```

## Quick Start

### Start with Persistence Enabled

```bash
./start-services.sh --persist
```

This starts:
- Rust AOEE server (port 50051)
- Persistence service (port 8081)
- Spring Boot proxy with write-through enabled (port 8080)
- React UI (port 5173)

### Access Points

| Service | URL |
|---------|-----|
| REST API | http://localhost:8081/api/v1/ |
| GraphQL | http://localhost:8081/graphql |
| GraphiQL IDE | http://localhost:8081/graphiql |
| H2 Console | http://localhost:8081/h2-console |

### H2 Console Login

- JDBC URL: `jdbc:h2:file:./data/aoee`
- Username: `sa`
- Password: (empty)

## REST API

### Entities

```bash
# Create entity
curl -X POST http://localhost:8081/api/v1/entities \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "entityType": "USER", "name": "Alice"}'

# Get entity
curl http://localhost:8081/api/v1/entities/1

# List entities by type
curl "http://localhost:8081/api/v1/entities?type=USER"

# Delete entity
curl -X DELETE http://localhost:8081/api/v1/entities/1
```

### Edges

```bash
# Create edge
curl -X POST http://localhost:8081/api/v1/edges \
  -H "Content-Type: application/json" \
  -d '{"src": 1, "edgeType": "FOLLOWS", "dst": 2}'

# Create edge with metadata (reaction)
curl -X POST http://localhost:8081/api/v1/edges \
  -H "Content-Type: application/json" \
  -d '{"src": 1, "edgeType": "LIKES", "dst": 1001, "metadata": 1}'

# Get edges by source
curl "http://localhost:8081/api/v1/edges?src=1&type=FOLLOWS"

# Get neighbors
curl "http://localhost:8081/api/v1/edges/neighbors?src=1&type=FOLLOWS"

# Check edge exists
curl http://localhost:8081/api/v1/edges/1/FOLLOWS/2/exists

# Delete edge
curl -X DELETE http://localhost:8081/api/v1/edges/1/FOLLOWS/2

# Get mutual connections
curl "http://localhost:8081/api/v1/edges/mutual?id1=1&id2=2&type=FRIEND_OF"
```

### Export/Import

```bash
# Export all edges in AOEE dataset format
curl http://localhost:8081/api/v1/export/edges

# Import edges from dataset
curl -X POST http://localhost:8081/api/v1/export/edges \
  -H "Content-Type: text/plain" \
  -d "FOLLOWS 1 2
FOLLOWS 1 3
LIKES 1 1001 1 love"

# Get stats
curl http://localhost:8081/api/v1/export/stats
```

## GraphQL API

Access the GraphiQL IDE at http://localhost:8081/graphiql

### Example Queries

```graphql
# Get an entity with its edges
query {
  entity(id: "1") {
    id
    entityType
    name
    outgoingEdges(edgeType: "FOLLOWS") {
      dstId
      dst {
        name
      }
    }
    neighborCount(edgeType: "FOLLOWS")
  }
}

# Get neighbors
query {
  neighbors(src: "1", edgeType: "FOLLOWS") {
    neighbors
    count
  }
}

# Find mutual friends
query {
  mutualConnections(id1: "1", id2: "2", edgeType: "FRIEND_OF") {
    mutual
    count
  }
}

# Get stats
query {
  stats {
    totalEntities
    totalEdges
    entityTypes
    edgeTypes
  }
}
```

### Example Mutations

```graphql
# Create entity
mutation {
  createEntity(id: "100", entityType: "USER", name: "Bob") {
    id
    name
    createdAt
  }
}

# Create edge
mutation {
  createEdge(src: "1", edgeType: "FOLLOWS", dst: "100") {
    id
    srcId
    dstId
  }
}

# Delete edge
mutation {
  deleteEdge(src: "1", edgeType: "FOLLOWS", dst: "100")
}

# Import dataset
mutation {
  importDataset(content: "FOLLOWS 1 2\nFOLLOWS 1 3")
}
```

## Write-Through Configuration

The Spring Boot proxy can be configured to write-through to persistence:

```yaml
# aoee-spring/src/main/resources/application.yml
aoee:
  persistence:
    enabled: true                    # Enable persistence integration
    url: http://localhost:8081       # Persistence service URL
    write-through: true              # Write to persistence on add/delete
    warm-on-startup: false           # Load from persistence on startup
```

### Cache Warming

To warm the AOEE cache from persisted data:

```bash
# Warm cache from persistence
curl -X POST http://localhost:8080/api/cache/warm

# Check persistence status
curl http://localhost:8080/api/cache/persistence/status
```

## Database Schema

### Entities Table

```sql
CREATE TABLE entities (
    id BIGINT PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

### Edges Table

```sql
CREATE TABLE edges (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    src_id BIGINT NOT NULL,
    edge_type VARCHAR(50) NOT NULL,
    dst_id BIGINT NOT NULL,
    timestamp_ns BIGINT NOT NULL,
    metadata SMALLINT DEFAULT 0 NOT NULL,
    created_at TIMESTAMP NOT NULL,
    UNIQUE (src_id, edge_type, dst_id)
);
```

### Indexes

- `idx_edges_src_type` - Forward traversal (src_id, edge_type)
- `idx_edges_dst_type` - Reverse traversal (dst_id, edge_type)
- `idx_edges_type` - Edge type queries

## PostgreSQL Configuration

To use PostgreSQL instead of H2:

1. Create database:
```sql
CREATE DATABASE aoee;
CREATE USER aoee WITH PASSWORD 'aoee';
GRANT ALL PRIVILEGES ON DATABASE aoee TO aoee;
```

2. Run with postgres profile:
```bash
cd aoee-persistence
mvn spring-boot:run -Dspring-boot.run.profiles=postgres
```

Or set in application.yml:
```yaml
spring:
  profiles:
    active: postgres
```

## Building

```bash
cd aoee-persistence
mvn clean package

# Run standalone
java -jar target/aoee-persistence-0.1.0.jar
```

## Comparison: AOEE vs Persistence

| Aspect | AOEE (Rust) | Persistence |
|--------|-------------|-------------|
| Storage | In-memory | Database (H2/PostgreSQL) |
| Latency | ~70µs | ~1-10ms |
| Scale | Millions of edges | Limited by disk |
| Durability | Volatile | Persistent |
| Queries | Optimized graph ops | Standard SQL/GraphQL |

The persistence service is designed as a backing store, not a replacement for AOEE's in-memory performance.

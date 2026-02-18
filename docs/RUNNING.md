# Running AOEE

This guide covers starting all AOEE services and running tests.

## Prerequisites

- **Rust** - Latest stable (for aoee-server)
- **Java 21+** - For Spring Boot services
- **Maven** - For building Java projects
- **Node.js** - For React UI (optional)
- **grpcurl** - For testing gRPC endpoints

### Install grpcurl (macOS)
```bash
brew install grpcurl
```

## Quick Start

### Option 1: All Services Script
```bash
# Start all services (in-memory mode)
./start-services.sh

# Start with persistence
./start-services.sh --persist
```

### Option 2: Manual Startup

#### 1. Build Rust Server
```bash
cd aoee
cargo build --release --features all-backends
```

#### 2. Start Services Individually

**In-Memory Mode (Development):**
```bash
./aoee/target/release/aoee-server
```

**With RocksDB Persistence:**
```bash
AOEE_STORAGE_TYPE=rocksdb \
AOEE_WRITE_THROUGH=true \
AOEE_ROCKSDB_PATH=./data/aoee-rocksdb \
./aoee/target/release/aoee-server
```

**With HTTP Backend:**
```bash
# Terminal 1: Start persistence service
cd aoee-persistence
mvn spring-boot:run -Dspring-boot.run.arguments="--server.port=9081"

# Terminal 2: Start AOEE server
AOEE_STORAGE_TYPE=http \
AOEE_WRITE_THROUGH=true \
AOEE_HTTP_URL=http://localhost:9081 \
./aoee/target/release/aoee-server
```

**With Spring Boot Proxy:**
```bash
# Terminal 3: Start Spring proxy (optional)
cd aoee-spring
mvn spring-boot:run
```

**With React UI:**
```bash
# Terminal 4: Start UI (optional)
cd aoee-ui
npm run dev
# or serve built version:
python3 -m http.server 5173 --directory dist
```

## Service Ports

| Service | Port | Protocol |
|---------|------|----------|
| AOEE Rust Server | 50051 | gRPC |
| Persistence Service | 9081 | REST/GraphQL |
| Spring Boot Proxy | 8080 | REST/WebSocket |
| React UI | 5173 | HTTP |

## Testing

### Run Integration Tests
```bash
# Full persistence integration test
./test-persistence.sh
```

This test:
1. Starts persistence service on port 9081
2. Starts AOEE with HTTP backend
3. Creates a social graph (25 follow edges)
4. Creates likes with reaction metadata (11 edges)
5. Tests intersect, union, count, contains operations
6. Clears cache and verifies reload from persistence
7. Verifies metadata preservation

### Run Rust Unit Tests
```bash
cd aoee

# All tests
cargo test

# Specific crate tests
cargo test -p aoee-core
cargo test -p aoee-storage
cargo test -p aoee-shard
cargo test -p aoee-server
```

### Run with Specific Features
```bash
# With all backends
cargo test --features all-backends

# With only RocksDB
cargo test --features rocksdb-backend

# With only HTTP backend
cargo test --features http-backend
```

### Manual gRPC Testing

**Add an edge:**
```bash
grpcurl -plaintext -d '{
  "key": {"src": 1, "edge_type": 0},
  "dst": 100
}' localhost:50051 aoee.Aoee/AddEdge
```

**Add edge with reaction:**
```bash
grpcurl -plaintext -d '{
  "key": {"src": 1, "edge_type": 10},
  "dst": 1000,
  "metadata": 1
}' localhost:50051 aoee.Aoee/AddEdge
```

**Get neighbors:**
```bash
grpcurl -plaintext -d '{
  "key": {"src": 1, "edge_type": 0}
}' localhost:50051 aoee.Aoee/Neighbors
```

**Get neighbors with metadata:**
```bash
grpcurl -plaintext -d '{
  "key": {"src": 1, "edge_type": 10},
  "include_metadata": true
}' localhost:50051 aoee.Aoee/Neighbors
```

**Check if edge exists:**
```bash
grpcurl -plaintext -d '{
  "key": {"src": 1, "edge_type": 0},
  "dst": 100
}' localhost:50051 aoee.Aoee/Contains
```

**Count edges:**
```bash
grpcurl -plaintext -d '{
  "key": {"src": 1, "edge_type": 0}
}' localhost:50051 aoee.Aoee/Count
```

**Find common follows (intersect):**
```bash
grpcurl -plaintext -d '{
  "key1": {"src": 1, "edge_type": 0},
  "key2": {"src": 2, "edge_type": 0}
}' localhost:50051 aoee.Aoee/Intersect
```

**Find all follows (union):**
```bash
grpcurl -plaintext -d '{
  "key1": {"src": 1, "edge_type": 0},
  "key2": {"src": 2, "edge_type": 0}
}' localhost:50051 aoee.Aoee/Union
```

**Get statistics:**
```bash
grpcurl -plaintext -d '{"per_shard": true}' localhost:50051 aoee.Aoee/Stats
```

**Clear cache:**
```bash
grpcurl -plaintext -d '{}' localhost:50051 aoee.Aoee/ClearCache
```

**List available gRPC methods:**
```bash
grpcurl -plaintext localhost:50051 list aoee.Aoee
```

### Persistence API Testing

**Create edge via REST:**
```bash
curl -X POST http://localhost:9081/api/v1/edges \
  -H "Content-Type: application/json" \
  -d '{"src": 1, "edgeType": "FOLLOWS", "dst": 100}'
```

**Get edges:**
```bash
curl "http://localhost:9081/api/v1/edges?src=1&type=FOLLOWS"
```

**Get neighbors:**
```bash
curl "http://localhost:9081/api/v1/edges/neighbors?src=1&type=FOLLOWS"
```

**Get count:**
```bash
curl "http://localhost:9081/api/v1/edges/count?src=1&type=FOLLOWS"
```

**Check health:**
```bash
curl http://localhost:9081/actuator/health
```

**Delete all edges (testing only):**
```bash
curl -X DELETE http://localhost:9081/api/v1/edges
```

## Benchmarking

### Built-in Benchmark
```bash
./run-benchmark.sh
```

### Custom Benchmark Parameters
```bash
curl -X POST "http://localhost:8080/api/benchmark/generate?users=10000&avgFollows=100"
curl -X POST "http://localhost:8080/api/benchmark/run?iterations=10000&parallel=true"
```

## Stopping Services

```bash
# Stop all AOEE processes
pkill -f "aoee-server"
pkill -f "aoee-persistence.*spring-boot:run"
pkill -f "aoee-spring.*spring-boot:run"
pkill -f "http.server 5173"
```

## Logs

Log files are written to `./logs/`:
- `aoee-server.log` - Rust server logs
- `persistence.log` - Persistence service logs
- `spring-boot.log` - Spring proxy logs
- `react-ui.log` - UI server logs

## Troubleshooting

### Port Already in Use
```bash
# Check what's using a port
lsof -i :50051
lsof -i :9081

# Kill specific process
kill -9 <PID>
```

### gRPC Connection Refused
```bash
# Verify server is running
ps aux | grep aoee-server

# Check if listening
lsof -i :50051
```

### Persistence Not Responding
```bash
# Check Java process
ps aux | grep aoee-persistence

# Check logs
tail -f logs/persistence.log

# Verify health endpoint
curl http://localhost:9081/actuator/health
```

### Cache Not Loading from Persistence
1. Verify write-through is enabled: `AOEE_WRITE_THROUGH=true`
2. Check HTTP URL: `AOEE_HTTP_URL=http://localhost:9081`
3. Verify persistence service is healthy
4. Check for 404/405 errors in aoee-server logs

## Environment Variables Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `AOEE_STORAGE_TYPE` | `memory` | Storage backend: memory, rocksdb, http |
| `AOEE_WRITE_THROUGH` | `true` | Enable write-through to storage |
| `AOEE_ROCKSDB_PATH` | `./data/aoee-rocksdb` | RocksDB data directory |
| `AOEE_HTTP_URL` | `http://localhost:8081` | HTTP backend URL |
| `AOEE_LISTEN_ADDR` | `[::1]:50051` | gRPC listen address |
| `AOEE_NUM_SHARDS` | `4` | Number of shards |

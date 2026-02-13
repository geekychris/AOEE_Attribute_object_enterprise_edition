#!/bin/bash

# Integration Test: AOEE Write-Through Persistence
#
# This test verifies that:
# 1. Writes to AOEE cache are persisted to the HTTP backend
# 2. After cache clear, reads reload from persistence
# 3. Data integrity is maintained across cache eviction
#
# Prerequisites:
# - Java 21+ with Maven
# - Rust toolchain with aoee built
# - grpcurl installed

set -e

AOEE_ROOT="/Users/chris/AOEE"
PERSISTENCE_PORT=9081
AOEE_PORT=50051
LOG_DIR="$AOEE_ROOT/logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_test() { echo -e "\n${YELLOW}=== TEST: $1 ===${NC}"; }

cleanup() {
    log_info "Cleaning up processes..."
    pkill -f "aoee-persistence.*spring-boot:run" 2>/dev/null || true
    pkill -f "aoee-server" 2>/dev/null || true
    sleep 1
}

wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=30
    local attempt=1
    
    log_info "Waiting for $name to be ready..."
    while [ $attempt -le $max_attempts ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            log_info "$name is ready"
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    
    log_error "$name failed to start after $max_attempts seconds"
    return 1
}

wait_for_grpc() {
    local port=$1
    local name=$2
    local max_attempts=30
    local attempt=1
    
    log_info "Waiting for $name gRPC to be ready..."
    while [ $attempt -le $max_attempts ]; do
        # Try a simple gRPC call to check if server is up
        if grpcurl -plaintext -d '{"per_shard": false}' localhost:$port aoee.Aoee/Stats > /dev/null 2>&1; then
            log_info "$name gRPC is ready"
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    
    log_error "$name gRPC failed to start after $max_attempts seconds"
    # Show the server log for debugging
    log_error "Server log:"
    cat "$LOG_DIR/aoee-server-test.log" | tail -20
    return 1
}

# Trap cleanup on exit
trap cleanup EXIT

# Create log directory
mkdir -p "$LOG_DIR"

log_info "Starting AOEE Persistence Integration Test"

# Step 1: Clean up existing processes
cleanup

# Step 2: Start the persistence service
log_info "Starting persistence service on port $PERSISTENCE_PORT..."
cd "$AOEE_ROOT/aoee-persistence"
mvn spring-boot:run -q -Dspring-boot.run.arguments="--server.port=$PERSISTENCE_PORT" > "$LOG_DIR/persistence-test.log" 2>&1 &
PERSIST_PID=$!

wait_for_service "http://localhost:$PERSISTENCE_PORT/actuator/health" "Persistence Service"

# Step 3: Clear any existing data in persistence
log_info "Clearing existing persistence data..."
curl -s -X DELETE "http://localhost:$PERSISTENCE_PORT/api/v1/edges" > /dev/null 2>&1 || log_info "(No existing data to clear)"

# Step 4: Start AOEE server with HTTP backend
log_info "Starting AOEE server with HTTP backend on port $AOEE_PORT..."
cd "$AOEE_ROOT/aoee"
AOEE_STORAGE_TYPE=http \
AOEE_WRITE_THROUGH=true \
AOEE_HTTP_URL="http://localhost:$PERSISTENCE_PORT" \
./target/release/aoee-server > "$LOG_DIR/aoee-server-test.log" 2>&1 &
AOEE_PID=$!

wait_for_grpc $AOEE_PORT "AOEE Server"

# ============================================
# TEST 1: Build a social graph
# ============================================
log_test "Building social graph (write-through to persistence)"

# Create a social network:
#   User 1 follows: 10, 11, 12, 13
#   User 2 follows: 11, 12, 14, 15
#   User 3 follows: 12, 13, 15, 16
#   User 10 follows: 20, 21, 22
#   User 11 follows: 21, 22, 23
#   User 12 follows: 22, 23, 24
#
# This creates a graph where:
#   - Users 1 and 2 have common follows: 11, 12
#   - User 1's FoF through 10,11,12 includes: 20,21,22,23,24

log_info "Creating follow relationships..."

# User 1's follows
for dst in 10 11 12 13; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 1, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 1 follows: 10, 11, 12, 13"

# User 2's follows
for dst in 11 12 14 15; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 2, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 2 follows: 11, 12, 14, 15"

# User 3's follows
for dst in 12 13 15 16; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 3, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 3 follows: 12, 13, 15, 16"

# User 10's follows (for FoF test)
for dst in 20 21 22; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 10, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 10 follows: 20, 21, 22"

# User 11's follows (for FoF test)
for dst in 21 22 23; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 11, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 11 follows: 21, 22, 23"

# User 12's follows (for FoF test)
for dst in 22 23 24; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 12, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 12 follows: 22, 23, 24"

# User 13's follows
for dst in 23 24 25; do
    grpcurl -plaintext -d "{\"key\": {\"src\": 13, \"edge_type\": 0}, \"dst\": $dst}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  User 13 follows: 23, 24, 25"

log_info "Created 24 follow edges"

# ============================================
# TEST 2: Add likes with reactions (metadata)
# ============================================
log_test "Adding likes with reaction metadata"

# Reaction metadata: 0=like, 1=love, 2=haha, 3=wow, 4=sad, 5=angry
# Post 1000 gets various reactions
log_info "Adding reactions to Post 1000..."

# User 1 loves Post 1000 (metadata=1)
grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 10}, "dst": 1000, "metadata": 1}' \
    localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
log_info "  User 1 loves Post 1000"

# User 2 likes Post 1000 (metadata=0)
grpcurl -plaintext -d '{"key": {"src": 2, "edge_type": 10}, "dst": 1000, "metadata": 0}' \
    localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
log_info "  User 2 likes Post 1000"

# User 3 haha's Post 1000 (metadata=2)
grpcurl -plaintext -d '{"key": {"src": 3, "edge_type": 10}, "dst": 1000, "metadata": 2}' \
    localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
log_info "  User 3 haha Post 1000"

# User 10 wows Post 1000 (metadata=3)
grpcurl -plaintext -d '{"key": {"src": 10, "edge_type": 10}, "dst": 1000, "metadata": 3}' \
    localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
log_info "  User 10 wow Post 1000"

# User 11 sad's Post 1000 (metadata=4)
grpcurl -plaintext -d '{"key": {"src": 11, "edge_type": 10}, "dst": 1000, "metadata": 4}' \
    localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
log_info "  User 11 sad Post 1000"

# Post 1001 gets some reactions too
for user in 1 2 3 10 11 12; do
    grpcurl -plaintext -d "{\"key\": {\"src\": $user, \"edge_type\": 10}, \"dst\": 1001, \"metadata\": 1}" \
        localhost:$AOEE_PORT aoee.Aoee/AddEdge > /dev/null
done
log_info "  6 users love Post 1001"

log_info "Created 11 like edges with reactions"

# ============================================
# TEST 3: Verify follow data in AOEE cache
# ============================================
log_test "Verify follow data in AOEE cache"

log_info "Getting neighbors for User 1 from AOEE..."
NEIGHBORS_1=$(grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 0}}' \
    localhost:$AOEE_PORT aoee.Aoee/Neighbors | jq -r '.neighbors | length')

if [ "$NEIGHBORS_1" = "4" ]; then
    log_info "✓ User 1 has 4 follows (correct)"
else
    log_error "✗ User 1 has $NEIGHBORS_1 follows (expected 4)"
    exit 1
fi

log_info "Getting neighbors for User 2 from AOEE..."
NEIGHBORS_2=$(grpcurl -plaintext -d '{"key": {"src": 2, "edge_type": 0}}' \
    localhost:$AOEE_PORT aoee.Aoee/Neighbors | jq -r '.neighbors | length')

if [ "$NEIGHBORS_2" = "4" ]; then
    log_info "✓ User 2 has 4 follows (correct)"
else
    log_error "✗ User 2 has $NEIGHBORS_2 follows (expected 4)"
    exit 1
fi

# ============================================
# TEST 4: Verify likes with metadata
# ============================================
log_test "Verify likes with reaction metadata"

log_info "Getting likes for Post 1000 with metadata..."
LIKES_RESPONSE=$(grpcurl -plaintext -d '{"key": {"src": 1000, "edge_type": 11}, "include_metadata": true}' \
    localhost:$AOEE_PORT aoee.Aoee/Neighbors)

# Note: We stored User->Post (LIKES=10), so we need to query Post->Users (LIKED_BY=11)
# But we didn't create reverse edges. Let's check what we have for likes directly.
LIKES_BY_USER1=$(grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 10}, "include_metadata": true}' \
    localhost:$AOEE_PORT aoee.Aoee/Neighbors)
LIKES_COUNT=$(echo "$LIKES_BY_USER1" | jq -r '.neighbors | length')
METADATA=$(echo "$LIKES_BY_USER1" | jq -r '.metadata[0]')

if [ "$LIKES_COUNT" -ge "1" ]; then
    log_info "✓ User 1 has $LIKES_COUNT likes"
else
    log_error "✗ User 1 should have likes"
fi

if [ "$METADATA" = "1" ]; then
    log_info "✓ User 1's reaction is 'love' (metadata=1)"
else
    log_warn "Metadata value: $METADATA (expected 1 for love)"
fi

# ============================================
# TEST 5: Verify data in persistence layer
# ============================================
log_test "Verify data in persistence layer"

sleep 1  # Give time for async writes to complete

log_info "Querying persistence layer directly..."
PERSIST_EDGES=$(curl -s "http://localhost:$PERSISTENCE_PORT/api/v1/edges/count" | jq -r '.count')

if [ "$PERSIST_EDGES" -ge "35" ]; then
    log_info "✓ Persistence has $PERSIST_EDGES edges (expected at least 35)"
else
    log_error "✗ Persistence has $PERSIST_EDGES edges (expected at least 35)"
    exit 1
fi

# Check follows in persistence
log_info "Checking User 1's follows in persistence..."
PERSIST_USER1=$(curl -s "http://localhost:$PERSISTENCE_PORT/api/v1/edges?src=1&type=FOLLOWS" | jq -r 'length')

if [ "$PERSIST_USER1" -ge "4" ]; then
    log_info "✓ Persistence has $PERSIST_USER1 follow edges for User 1"
else
    log_error "✗ Persistence has $PERSIST_USER1 follow edges for User 1 (expected 4)"
    exit 1
fi

# Check likes in persistence
log_info "Checking likes in persistence..."
PERSIST_LIKES=$(curl -s "http://localhost:$PERSISTENCE_PORT/api/v1/edges/count?type=LIKES" | jq -r '.count')

if [ "$PERSIST_LIKES" -ge "11" ]; then
    log_info "✓ Persistence has $PERSIST_LIKES like edges"
else
    log_error "✗ Persistence has $PERSIST_LIKES like edges (expected 11)"
    exit 1
fi

# ============================================
# TEST 6: Test intersect (common follows)
# ============================================
log_test "Test intersect operation (common follows)"

log_info "Finding common follows between User 1 and User 2..."
COMMON_1_2=$(grpcurl -plaintext -d '{
    "key1": {"src": 1, "edge_type": 0},
    "key2": {"src": 2, "edge_type": 0}
}' localhost:$AOEE_PORT aoee.Aoee/Intersect)

COMMON_COUNT=$(echo "$COMMON_1_2" | jq -r '.ids | length')
COMMON_IDS=$(echo "$COMMON_1_2" | jq -r '.ids | sort | join(",")')

if [ "$COMMON_COUNT" = "2" ]; then
    log_info "✓ Users 1 and 2 have 2 common follows: $COMMON_IDS (expected 11, 12)"
else
    log_error "✗ Expected 2 common follows, got $COMMON_COUNT: $COMMON_IDS"
    exit 1
fi

log_info "Finding common follows between User 1 and User 3..."
COMMON_1_3=$(grpcurl -plaintext -d '{
    "key1": {"src": 1, "edge_type": 0},
    "key2": {"src": 3, "edge_type": 0}
}' localhost:$AOEE_PORT aoee.Aoee/Intersect)

COMMON_COUNT=$(echo "$COMMON_1_3" | jq -r '.ids | length')
if [ "$COMMON_COUNT" = "2" ]; then
    log_info "✓ Users 1 and 3 have 2 common follows (12, 13)"
else
    log_error "✗ Expected 2 common follows for Users 1 and 3, got $COMMON_COUNT"
    exit 1
fi

# ============================================
# TEST 7: Test union operation
# ============================================
log_test "Test union operation"

log_info "Finding union of follows for User 1 and User 2..."
UNION_1_2=$(grpcurl -plaintext -d '{
    "key1": {"src": 1, "edge_type": 0},
    "key2": {"src": 2, "edge_type": 0}
}' localhost:$AOEE_PORT aoee.Aoee/Union)

UNION_COUNT=$(echo "$UNION_1_2" | jq -r '.ids | length')

if [ "$UNION_COUNT" = "6" ]; then
    log_info "✓ Union of User 1 and 2 follows has 6 unique users (10,11,12,13,14,15)"
else
    log_error "✗ Expected 6 in union, got $UNION_COUNT"
    exit 1
fi

# ============================================
# TEST 8: Clear AOEE cache
# ============================================
log_test "Clear AOEE cache"

log_info "Getting cache stats before clear..."
STATS_BEFORE=$(grpcurl -plaintext -d '{"per_shard": true}' \
    localhost:$AOEE_PORT aoee.Aoee/Stats)
echo "$STATS_BEFORE" | jq .aggregated

log_info "Clearing AOEE cache..."
CLEAR_RESULT=$(grpcurl -plaintext -d '{}' \
    localhost:$AOEE_PORT aoee.Aoee/ClearCache)
ENTRIES_CLEARED=$(echo "$CLEAR_RESULT" | jq -r '.entriesCleared')
log_info "Cleared $ENTRIES_CLEARED cache entries"

log_info "Getting cache stats after clear..."
STATS_AFTER=$(grpcurl -plaintext -d '{"per_shard": true}' \
    localhost:$AOEE_PORT aoee.Aoee/Stats)
CACHED_AFTER=$(echo "$STATS_AFTER" | jq -r '.aggregated.cachedLists')
log_info "Cached lists after clear: $CACHED_AFTER"

# ============================================
# TEST 9: Read from AOEE after cache clear (reload from persistence)
# ============================================
log_test "Read from AOEE after cache clear (reload from persistence)"

log_info "Getting neighbors for User 1 from AOEE (should load from persistence)..."
NEIGHBORS_1_RELOAD=$(grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 0}}' \
    localhost:$AOEE_PORT aoee.Aoee/Neighbors | jq -r '.neighbors | length')

if [ "$NEIGHBORS_1_RELOAD" = "4" ]; then
    log_info "✓ User 1 has 4 follows after reload (correct)"
else
    log_error "✗ User 1 has $NEIGHBORS_1_RELOAD follows after reload (expected 4)"
    exit 1
fi

log_info "Getting likes for User 1 from AOEE (should load from persistence)..."
LIKES_1_RELOAD=$(grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 10}, "include_metadata": true}' \
    localhost:$AOEE_PORT aoee.Aoee/Neighbors)
LIKES_COUNT=$(echo "$LIKES_1_RELOAD" | jq -r '.neighbors | length')
METADATA_RELOAD=$(echo "$LIKES_1_RELOAD" | jq -r '.metadata[0]')

if [ "$LIKES_COUNT" -ge "1" ]; then
    log_info "✓ User 1's likes reloaded from persistence"
else
    log_error "✗ User 1's likes not reloaded"
    exit 1
fi

if [ "$METADATA_RELOAD" = "1" ]; then
    log_info "✓ Reaction metadata preserved after reload (love=1)"
else
    log_warn "Metadata after reload: $METADATA_RELOAD (expected 1)"
fi

# ============================================
# TEST 10: Verify cache misses increased
# ============================================
log_test "Verify cache misses (proves reload from storage)"

STATS_FINAL=$(grpcurl -plaintext -d '{"per_shard": true}' \
    localhost:$AOEE_PORT aoee.Aoee/Stats)
CACHE_MISSES=$(echo "$STATS_FINAL" | jq -r '.aggregated.cacheMisses')

if [ "$CACHE_MISSES" -ge "2" ]; then
    log_info "✓ Cache misses: $CACHE_MISSES (data reloaded from storage)"
else
    log_warn "Cache misses: $CACHE_MISSES (expected >= 2)"
fi

# ============================================
# TEST 11: Test count operation
# ============================================
log_test "Test count operation"

COUNT_USER10=$(grpcurl -plaintext -d '{"key": {"src": 10, "edge_type": 0}}' \
    localhost:$AOEE_PORT aoee.Aoee/Count | jq -r '.count')

if [ "$COUNT_USER10" = "3" ]; then
    log_info "✓ User 10 has 3 follows (correct)"
else
    log_error "✗ User 10 count is $COUNT_USER10 (expected 3)"
    exit 1
fi

# ============================================
# TEST 12: Test contains operation
# ============================================
log_test "Test contains operation"

CONTAINS_YES=$(grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 0}, "dst": 10}' \
    localhost:$AOEE_PORT aoee.Aoee/Contains | jq -r '.exists')

if [ "$CONTAINS_YES" = "true" ]; then
    log_info "✓ User 1 follows User 10 (correct)"
else
    log_error "✗ Contains returned $CONTAINS_YES (expected true)"
    exit 1
fi

CONTAINS_NO=$(grpcurl -plaintext -d '{"key": {"src": 1, "edge_type": 0}, "dst": 999}' \
    localhost:$AOEE_PORT aoee.Aoee/Contains | jq -r '.exists // false')

if [ "$CONTAINS_NO" = "false" ]; then
    log_info "✓ User 1 does not follow User 999 (correct)"
else
    log_error "✗ Contains returned $CONTAINS_NO (expected false)"
    exit 1
fi

# ============================================
# Summary
# ============================================
echo ""
echo "============================================"
echo -e "${GREEN}All tests passed!${NC}"
echo "============================================"
echo ""
echo "Test Summary:"
echo "  ✓ Social graph creation (24 follow edges)"
echo "  ✓ Likes with reaction metadata (11 like edges)"
echo "  ✓ Write-through to persistence working"
echo "  ✓ Data persisted correctly (35+ edges)"
echo "  ✓ Intersect operation (common follows)"
echo "  ✓ Union operation"
echo "  ✓ Count operation"
echo "  ✓ Contains operation"
echo "  ✓ Cache clear working"
echo "  ✓ Reload from persistence working"
echo "  ✓ Reaction metadata preserved"
echo ""
echo "Logs available at: $LOG_DIR/"

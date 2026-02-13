# AOEE Benchmarking Guide

This guide explains how to run performance benchmarks on AOEE to measure latency and throughput at scale.

## Quick Start

```bash
# 1. Start all services
./start-services.sh

# 2. Run a small benchmark
./run-benchmark.sh small
```

## Prerequisites

Ensure all AOEE services are running:

- **Rust AOEE Server** on port 50051 (gRPC)
- **Spring Boot Proxy** on port 8080 (REST)

Start them with:
```bash
./start-services.sh
```

## Benchmark Script

The `run-benchmark.sh` script provides an easy way to run benchmarks:

```bash
./run-benchmark.sh [command]

Commands:
  small     Run small benchmark (1K users, ~125K edges)
  medium    Run medium benchmark (10K users, ~1.5M edges)  
  large     Run large benchmark (100K users, ~15M edges)
  presets   Show preset configurations
  generate  Generate data only (no benchmarking)
  help      Show help
```

### Examples

```bash
# Run small benchmark (fastest, good for quick tests)
./run-benchmark.sh small

# Run medium benchmark (realistic workload)
./run-benchmark.sh medium

# Run large benchmark (stress test)
./run-benchmark.sh large

# Just generate data without running benchmarks
./run-benchmark.sh generate medium

# View available presets
./run-benchmark.sh presets
```

## Benchmark Presets

| Preset | Users | Posts | Groups | Est. Edges | Est. Time |
|--------|-------|-------|--------|------------|-----------|
| small | 1,000 | 2,000 | 10 | ~125K | ~30 sec |
| medium | 10,000 | 20,000 | 50 | ~1.5M | ~5 min |
| large | 100,000 | 200,000 | 200 | ~15M | ~30+ min |

### Preset Details

**Small** (quick validation):
- 1,000 users with avg 50 follows each (max 200)
- 2,000 posts with avg 20 likes each (max 500)
- 20 "celebrity" users (2%) with 5x more followers
- 100 "viral" posts (5%) with many more likes

**Medium** (realistic workload):
- 10,000 users with avg 100 follows each (max 2,000)
- 20,000 posts with avg 50 likes each (max 5,000)
- 100 celebrities (1%) with 10x more followers
- 400 viral posts (2%)

**Large** (stress test):
- 100,000 users with avg 200 follows each (max 10,000)
- 200,000 posts with avg 100 likes each (max 50,000)
- 500 celebrities (0.5%) with 20x more followers
- 2,000 viral posts (1%)

## Data Generation

The benchmark generates a realistic social graph:

### Edge Types Created

1. **FOLLOWS** - Directional follow relationships
   - Power-law distribution (celebrities get more followers)
   - Average and max configurable per user

2. **FRIEND_OF** - Bidirectional friendships
   - Created as two edges (A→B and B→A)
   - Prevents duplicate friendships

3. **LIKES** - Users liking posts
   - Includes reaction metadata (👍❤️😂😮😢😠)
   - Viral posts get significantly more likes

4. **MEMBER_OF** - Group memberships
   - Users assigned to multiple groups
   - Configurable average members per group

### Power-Law Distribution

To simulate real social networks:
- A small percentage of users are "celebrities"
- Celebrities appear multiple times in follow candidate lists
- This creates users with thousands of followers (like real influencers)

## Operations Benchmarked

| Operation | What It Tests |
|-----------|---------------|
| `neighbors_follows` | Basic adjacency list retrieval |
| `neighbors_likes_metadata` | Retrieval with metadata (reactions) |
| `contains` | Point lookup (does edge exist?) |
| `count` | Edge count without full retrieval |
| `intersection_mutual_friends` | Set intersection (mutual friends) |
| `friend_of_friend` | 2-hop traversal with scoring |
| `large_list_popular_users` | High fan-out queries (celebrities) |
| `viral_post_query` | Users with many outgoing edges |

## Understanding Results

### Latency Metrics (in microseconds)

- **Min**: Best case latency
- **Median**: 50th percentile (typical response)
- **P90**: 90% of requests faster than this
- **P95**: 95% of requests faster than this  
- **P99**: 99% of requests faster than this
- **Max**: Worst case latency
- **StdDev**: Variation in response times

### Throughput

Operations per second (ops/s). Higher is better.

### Example Output

```
=== AOEE Benchmark Summary ===

Data Generation:
  - 1,000 users, 2,000 posts, 10 groups
  - 125,657 total edges in 8,575 ms (14654 edges/sec)
  - Edge breakdown: 47,477 follows, 18,912 friends, 58,142 likes, 1,126 members

Operation Performance (latency in µs):
  Operation                          Median        P95        P99        Max   Throughput
  ------------------------------------------------------------------------------------
  neighbors_follows                    67.5       91.9      151.3      246.0         14/s
  neighbors_likes_metadata             76.5      103.5      162.8      245.2         12/s
  contains                             64.4       79.4       93.1      269.8         15/s
  count                                65.4       85.2      134.6      403.4         15/s
  intersection_mutual_friends          67.1       91.2      148.7      266.8         14/s
  friend_of_friend                     69.1       91.0      129.0     1927.8         14/s
  large_list_popular_users             67.3       97.3      172.1      224.8         14/s
  viral_post_query                     76.1       95.8      134.9      250.6         13/s
```

## Custom Benchmarks

For custom configurations, use the REST API directly:

```bash
curl -X POST http://localhost:8080/api/benchmark/run \
  -H "Content-Type: application/json" \
  -d '{
    "numUsers": 5000,
    "avgFollowsPerUser": 150,
    "maxFollowsPerUser": 5000,
    "popularUserRatio": 0.01,
    "popularUserFollowerMultiplier": 15,
    "numPosts": 10000,
    "avgLikesPerPost": 100,
    "maxLikesPerPost": 20000,
    "viralPostRatio": 0.02,
    "avgFriendsPerUser": 75,
    "numGroups": 30,
    "avgMembersPerGroup": 300,
    "warmupIterations": 200,
    "benchmarkIterations": 5000,
    "includeMetadata": true
  }'
```

### Configuration Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `numUsers` | Total users to create | 10,000 |
| `avgFollowsPerUser` | Average follows per user | 100 |
| `maxFollowsPerUser` | Maximum follows (for heavy users) | 2,000 |
| `popularUserRatio` | % of users that are celebrities | 0.01 (1%) |
| `popularUserFollowerMultiplier` | How many more followers celebrities get | 10 |
| `numPosts` | Total posts to create | 20,000 |
| `avgLikesPerPost` | Average likes per post | 50 |
| `maxLikesPerPost` | Maximum likes (for viral posts) | 5,000 |
| `viralPostRatio` | % of posts that go viral | 0.02 (2%) |
| `avgFriendsPerUser` | Average bidirectional friends | 50 |
| `numGroups` | Number of groups | 50 |
| `avgMembersPerGroup` | Average members per group | 500 |
| `warmupIterations` | Warmup runs before measuring | 200 |
| `benchmarkIterations` | Measured iterations | 5,000 |
| `includeMetadata` | Include metadata in queries | true |

## Saving Results

The benchmark script automatically saves full results to JSON:

```bash
./run-benchmark.sh small
# Creates: benchmark-results-small.json
```

The JSON file includes:
- Full configuration used
- Data generation statistics
- Per-operation latency distributions
- All percentile values

## Tips for Accurate Benchmarks

1. **Run on a quiet machine** - Close other applications
2. **Let services warm up** - Run a small benchmark first
3. **Use consistent hardware** - Compare results on same machine
4. **Check logs for errors** - `tail -f logs/spring-boot.log`
5. **Monitor memory** - Large benchmarks need significant RAM

## Troubleshooting

**"Services not running"**
```bash
./start-services.sh
```

**Slow data generation**
- Large benchmarks create millions of edges
- Each edge requires a gRPC call
- Consider running overnight for 100K+ users

**Out of memory**
- Large benchmarks need 4GB+ RAM for the Rust server
- Reduce `numUsers` or `maxLikesPerPost`

**Timeouts**
- Spring Boot has no timeout by default for these endpoints
- Very large benchmarks may take 30+ minutes

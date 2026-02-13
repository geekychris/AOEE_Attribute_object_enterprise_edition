# AOEE Benchmark Plan

## Objectives
- Validate p50/p99 latency for core operations
- Measure QPS/core for intersections and friend-of-friend
- Quantify compaction overhead and tail-latency impact
- Measure memory per edge under different encodings

## Microbenchmarks
1. neighbors(src,type) with list sizes: 10, 100, 1k, 10k, 100k
2. intersect(a,b) size pairs: (1k,1k), (10k,1k), (100k,1k)
3. contains(src,type,dst): binary search vs skip-table seek vs bitmap
4. add_edge: write rates 100/s, 1k/s, 10k/s per key (hot-key stress)
5. compaction: buffer thresholds 64/256/1024; segment target 4KB/16KB/64KB

## Workload Benchmarks (mixes)
- Read-heavy: 95% reads / 5% writes
- Balanced: 70/30
- Burst: 50/50 + hot keys

## Metrics
- p50/p90/p99 latency per endpoint
- CPU cycles/op and instructions/op
- Cache hit ratio (objects, postings)
- Memory bytes/edge by representation
- Compaction time, segments/key, bytes written
- Tail latency correlation with compaction events

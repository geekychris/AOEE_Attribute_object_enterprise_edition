# AOEE Capacity Planning Cheatsheet

## Memory per edge (rule-of-thumb)
- Raw u32 dst_id: 4 bytes
- Delta+varint typical sparse: ~1–2 bytes/edge
- Overhead (segment headers + skip tables): ~1–5%
- Write buffers + tombstones budget: +5–15% depending on churn
- Reverse edges: ~2x edges

## Example
Edges: 2 billion
Avg bytes/edge (compressed): 1.6
Base: 3.2 GB
+ overhead 15%: 3.7 GB
+ reverse edges: 7.4 GB
+ buffers/objects/metadata (30%): ~9.6 GB

Replication factor 2 => ~19.2 GB across primaries+replicas.

## Hot keys
If degree > 10M:
- use roaring bitmap
- cap fanout and sampling
- isolate to dedicated shard class (optional)

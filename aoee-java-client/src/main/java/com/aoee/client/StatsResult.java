package com.aoee.client;

import java.util.List;

/**
 * AOEE server statistics.
 */
public record StatsResult(
        ShardStats aggregated,
        List<ShardStats> perShard
) {
    /**
     * Statistics for a single shard.
     */
    public record ShardStats(
            int shardId,
            long cachedLists,
            long totalEdges,
            long reads,
            long writes,
            long cacheHits,
            long cacheMisses
    ) {
        public double cacheHitRate() {
            long total = cacheHits + cacheMisses;
            return total > 0 ? (double) cacheHits / total : 0.0;
        }
    }
}

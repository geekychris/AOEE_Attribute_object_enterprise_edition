package com.aoee.spring.model;

import java.util.List;

public record ServerStatsResponse(
        ShardStats aggregated,
        List<ShardStats> perShard
) {
    public record ShardStats(
            int shardId,
            long cachedLists,
            long totalEdges,
            long reads,
            long writes,
            long cacheHits,
            long cacheMisses,
            double cacheHitRate
    ) {}
}

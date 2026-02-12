package com.aoee.spring.controller;

import com.aoee.client.AoeeClient;
import com.aoee.client.StatsResult;
import com.aoee.spring.model.ServerStatsResponse;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

@RestController
@RequestMapping("/api/stats")
public class StatsController {

    private final AoeeClient client;

    public StatsController(AoeeClient client) {
        this.client = client;
    }

    @GetMapping
    public ResponseEntity<ServerStatsResponse> getStats(
            @RequestParam(required = false, defaultValue = "false") boolean perShard) {
        
        StatsResult result = client.getStats(perShard);
        
        ServerStatsResponse.ShardStats aggregated = null;
        if (result.aggregated() != null) {
            var s = result.aggregated();
            aggregated = new ServerStatsResponse.ShardStats(
                    s.shardId(), s.cachedLists(), s.totalEdges(),
                    s.reads(), s.writes(), s.cacheHits(), s.cacheMisses(),
                    s.cacheHitRate()
            );
        }
        
        List<ServerStatsResponse.ShardStats> perShardStats = result.perShard().stream()
                .map(s -> new ServerStatsResponse.ShardStats(
                        s.shardId(), s.cachedLists(), s.totalEdges(),
                        s.reads(), s.writes(), s.cacheHits(), s.cacheMisses(),
                        s.cacheHitRate()
                ))
                .toList();
        
        return ResponseEntity.ok(new ServerStatsResponse(aggregated, perShardStats));
    }

    @GetMapping("/health")
    public ResponseEntity<Map<String, Object>> health() {
        boolean connected = client.isConnected();
        return ResponseEntity.ok(Map.of(
                "connected", connected,
                "target", client.getTarget()
        ));
    }
}

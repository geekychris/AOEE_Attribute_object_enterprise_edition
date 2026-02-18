package com.aoee.spring.benchmark;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

/**
 * REST controller for running benchmarks.
 */
@RestController
@RequestMapping("/api/benchmark")
public class BenchmarkController {
    private static final Logger log = LoggerFactory.getLogger(BenchmarkController.class);
    
    private final BenchmarkService benchmarkService;
    
    public BenchmarkController(BenchmarkService benchmarkService) {
        this.benchmarkService = benchmarkService;
    }
    
    /**
     * Run a complete benchmark with preset configuration.
     * 
     * @param size Preset size: "small", "medium", or "large"
     */
    @PostMapping("/run/{size}")
    public ResponseEntity<BenchmarkResult> runPresetBenchmark(@PathVariable String size) {
        BenchmarkConfig config = switch (size.toLowerCase()) {
            case "small" -> BenchmarkConfig.small();
            case "medium" -> BenchmarkConfig.medium();
            case "large" -> BenchmarkConfig.large();
            default -> BenchmarkConfig.small();
        };
        
        log.info("Starting {} benchmark", size);
        BenchmarkResult result = benchmarkService.runBenchmark(config);
        log.info("Benchmark complete: {}", result.summary());
        
        return ResponseEntity.ok(result);
    }
    
    /**
     * Run a benchmark with custom configuration.
     */
    @PostMapping("/run")
    public ResponseEntity<BenchmarkResult> runCustomBenchmark(@RequestBody BenchmarkConfigRequest request) {
        BenchmarkConfig config = BenchmarkConfig.builder()
            .numUsers(request.numUsers())
            .avgFollowsPerUser(request.avgFollowsPerUser())
            .maxFollowsPerUser(request.maxFollowsPerUser())
            .popularUserRatio(request.popularUserRatio())
            .popularUserFollowerMultiplier(request.popularUserFollowerMultiplier())
            .numPosts(request.numPosts())
            .avgLikesPerPost(request.avgLikesPerPost())
            .maxLikesPerPost(request.maxLikesPerPost())
            .viralPostRatio(request.viralPostRatio())
            .avgFriendsPerUser(request.avgFriendsPerUser())
            .numGroups(request.numGroups())
            .avgMembersPerGroup(request.avgMembersPerGroup())
            .warmupIterations(request.warmupIterations())
            .benchmarkIterations(request.benchmarkIterations())
            .includeMetadata(request.includeMetadata())
            .build();
        
        log.info("Starting custom benchmark with {} users, {} posts", 
            config.numUsers(), config.numPosts());
        BenchmarkResult result = benchmarkService.runBenchmark(config);
        log.info("Benchmark complete: {}", result.summary());
        
        return ResponseEntity.ok(result);
    }
    
    /**
     * Generate data only (no benchmarking).
     */
    @PostMapping("/generate/{size}")
    public ResponseEntity<BenchmarkResult.DataGenerationStats> generateData(@PathVariable String size) {
        BenchmarkConfig config = switch (size.toLowerCase()) {
            case "small" -> BenchmarkConfig.small();
            case "medium" -> BenchmarkConfig.medium();
            case "large" -> BenchmarkConfig.large();
            default -> BenchmarkConfig.small();
        };
        
        log.info("Generating {} dataset", size);
        var stats = benchmarkService.generateData(config);
        log.info("Data generation complete: {} edges", stats.totalEdges());
        
        return ResponseEntity.ok(stats);
    }
    
    /**
     * Get current benchmark data statistics.
     */
    @GetMapping("/stats")
    public ResponseEntity<Map<String, Object>> getStats() {
        return ResponseEntity.ok(benchmarkService.getDataStats());
    }
    
    /**
     * Get preset configuration details.
     */
    @GetMapping("/presets")
    public ResponseEntity<Map<String, BenchmarkConfig>> getPresets() {
        return ResponseEntity.ok(Map.of(
            "small", BenchmarkConfig.small(),
            "medium", BenchmarkConfig.medium(),
            "large", BenchmarkConfig.large()
        ));
    }
    
    /**
     * Request body for custom benchmark configuration.
     */
    public record BenchmarkConfigRequest(
        int numUsers,
        int avgFollowsPerUser,
        int maxFollowsPerUser,
        double popularUserRatio,
        int popularUserFollowerMultiplier,
        int numPosts,
        int avgLikesPerPost,
        int maxLikesPerPost,
        double viralPostRatio,
        int avgFriendsPerUser,
        int numGroups,
        int avgMembersPerGroup,
        int warmupIterations,
        int benchmarkIterations,
        boolean includeMetadata
    ) {
        // Defaults for optional fields
        public BenchmarkConfigRequest {
            if (numUsers <= 0) numUsers = 10_000;
            if (avgFollowsPerUser <= 0) avgFollowsPerUser = 100;
            if (maxFollowsPerUser <= 0) maxFollowsPerUser = 2000;
            if (popularUserRatio <= 0) popularUserRatio = 0.01;
            if (popularUserFollowerMultiplier <= 0) popularUserFollowerMultiplier = 10;
            if (numPosts <= 0) numPosts = 20_000;
            if (avgLikesPerPost <= 0) avgLikesPerPost = 50;
            if (maxLikesPerPost <= 0) maxLikesPerPost = 5000;
            if (viralPostRatio <= 0) viralPostRatio = 0.02;
            if (avgFriendsPerUser <= 0) avgFriendsPerUser = 50;
            if (numGroups <= 0) numGroups = 50;
            if (avgMembersPerGroup <= 0) avgMembersPerGroup = 500;
            if (warmupIterations <= 0) warmupIterations = 200;
            if (benchmarkIterations <= 0) benchmarkIterations = 5000;
        }
    }
}

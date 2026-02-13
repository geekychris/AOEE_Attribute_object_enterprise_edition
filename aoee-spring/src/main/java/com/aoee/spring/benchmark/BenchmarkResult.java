package com.aoee.spring.benchmark;

import java.util.List;
import java.util.Map;

/**
 * Results from a benchmark run.
 */
public record BenchmarkResult(
    // Configuration used
    BenchmarkConfig config,
    
    // Data generation stats
    DataGenerationStats dataGeneration,
    
    // Operation results
    List<OperationResult> operations,
    
    // Summary
    long totalDurationMs,
    String summary
) {
    /**
     * Statistics from data generation phase.
     */
    public record DataGenerationStats(
        int usersCreated,
        int postsCreated,
        int groupsCreated,
        long followEdges,
        long friendEdges,
        long likeEdges,
        long memberEdges,
        long totalEdges,
        long durationMs,
        double edgesPerSecond
    ) {}
    
    /**
     * Result of a single operation benchmark.
     */
    public record OperationResult(
        String operation,
        String description,
        int iterations,
        LatencyStats latency,
        double throughput,  // ops/sec
        Map<String, Object> details
    ) {}
    
    /**
     * Latency statistics in microseconds.
     */
    public record LatencyStats(
        double min,
        double max,
        double mean,
        double median,
        double p90,
        double p95,
        double p99,
        double stdDev
    ) {}
}

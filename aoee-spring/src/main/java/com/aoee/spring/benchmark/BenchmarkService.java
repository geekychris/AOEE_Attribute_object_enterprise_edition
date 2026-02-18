package com.aoee.spring.benchmark;

import com.aoee.client.AoeeClient;
import com.aoee.client.EdgeType;
import com.aoee.spring.benchmark.BenchmarkResult.*;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.util.*;
import java.util.concurrent.ThreadLocalRandom;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

/**
 * Service for generating benchmark data and running performance tests.
 */
@Service
public class BenchmarkService {
    private static final Logger log = LoggerFactory.getLogger(BenchmarkService.class);
    
    private final AoeeClient client;
    
    // Track generated data for benchmarking
    private final List<Long> userIds = new ArrayList<>();
    private final List<Long> postIds = new ArrayList<>();
    private final List<Long> groupIds = new ArrayList<>();
    private final List<Long> popularUserIds = new ArrayList<>();
    private final List<Long> viralPostIds = new ArrayList<>();
    private final Map<Long, Integer> userFollowerCounts = new HashMap<>();
    private final Map<Long, Integer> postLikeCounts = new HashMap<>();
    
    // ID ranges
    private static final long USER_ID_BASE = 1;
    private static final long POST_ID_BASE = 1_000_000;
    private static final long GROUP_ID_BASE = 2_000_000;
    
    // Batch size for edge insertion
    private static final int BATCH_SIZE = 10_000;
    
    // Batch size for entity persistence
    private static final int ENTITY_BATCH_SIZE = 5_000;
    
    public BenchmarkService(AoeeClient client) {
        this.client = client;
    }
    
    /**
     * Run a complete benchmark with data generation and operation testing.
     */
    public BenchmarkResult runBenchmark(BenchmarkConfig config) {
        long startTime = System.currentTimeMillis();
        log.info("Starting benchmark with config: {} users, {} posts", 
            config.numUsers(), config.numPosts());
        
        // Clear previous data tracking
        clearTracking();
        
        // Phase 1: Generate data
        log.info("Phase 1: Generating benchmark data...");
        DataGenerationStats dataStats = generateData(config);
        
        // Phase 2: Run operation benchmarks
        log.info("Phase 2: Running operation benchmarks...");
        List<OperationResult> operations = runOperationBenchmarks(config);
        
        long totalDuration = System.currentTimeMillis() - startTime;
        
        // Build summary
        String summary = buildSummary(config, dataStats, operations);
        
        return new BenchmarkResult(
            config,
            dataStats,
            operations,
            totalDuration,
            summary
        );
    }
    
    /**
     * Generate benchmark data according to configuration.
     */
    public DataGenerationStats generateData(BenchmarkConfig config) {
        long startTime = System.currentTimeMillis();
        clearTracking();
        
        Random random = new Random(42); // Fixed seed for reproducibility
        
        // Generate user IDs
        for (int i = 0; i < config.numUsers(); i++) {
            userIds.add(USER_ID_BASE + i);
        }
        
        // Identify popular users (celebrities)
        int numPopular = (int) (config.numUsers() * config.popularUserRatio());
        Collections.shuffle(new ArrayList<>(userIds), random);
        for (int i = 0; i < numPopular && i < userIds.size(); i++) {
            popularUserIds.add(userIds.get(i));
        }
        
        // Generate post IDs
        for (int i = 0; i < config.numPosts(); i++) {
            postIds.add(POST_ID_BASE + i);
        }
        
        // Identify viral posts
        int numViral = (int) (config.numPosts() * config.viralPostRatio());
        Collections.shuffle(new ArrayList<>(postIds), random);
        for (int i = 0; i < numViral && i < postIds.size(); i++) {
            viralPostIds.add(postIds.get(i));
        }
        
        // Generate group IDs
        for (int i = 0; i < config.numGroups(); i++) {
            groupIds.add(GROUP_ID_BASE + i);
        }
        
        // Persist entities to database
        log.info("Persisting entities to database...");
        persistEntities();
        
        long followEdges = 0;
        long friendEdges = 0;
        long likeEdges = 0;
        long memberEdges = 0;
        
        // Generate FOLLOWS edges (with power-law distribution)
        log.info("Generating follow relationships...");
        followEdges = generateFollowEdges(config, random);
        
        // Generate FRIEND edges
        log.info("Generating friend relationships...");
        friendEdges = generateFriendEdges(config, random);
        
        // Generate LIKES edges
        log.info("Generating like relationships...");
        likeEdges = generateLikeEdges(config, random);
        
        // Generate MEMBER edges
        log.info("Generating group memberships...");
        memberEdges = generateMemberEdges(config, random);
        
        long duration = System.currentTimeMillis() - startTime;
        long totalEdges = followEdges + friendEdges + likeEdges + memberEdges;
        double edgesPerSecond = totalEdges * 1000.0 / duration;
        
        log.info("Data generation complete: {} edges in {}ms ({} edges/sec)",
            totalEdges, duration, String.format("%.0f", edgesPerSecond));
        
        return new DataGenerationStats(
            config.numUsers(),
            config.numPosts(),
            config.numGroups(),
            followEdges,
            friendEdges,
            likeEdges,
            memberEdges,
            totalEdges,
            duration,
            edgesPerSecond
        );
    }
    
    private long generateFollowEdges(BenchmarkConfig config, Random random) {
        long count = 0;
        Set<Long> popularSet = new HashSet<>(popularUserIds);
        List<AoeeClient.EdgeData> batch = new ArrayList<>(BATCH_SIZE);
        
        for (Long userId : userIds) {
            // Determine how many users this user follows
            int numFollows = sampleCount(config.avgFollowsPerUser(), config.maxFollowsPerUser(), random);
            
            // Build candidate list (prefer popular users)
            List<Long> candidates = new ArrayList<>();
            
            // Add popular users with higher probability
            for (Long popular : popularUserIds) {
                if (!popular.equals(userId)) {
                    // Add popular users multiple times to increase their probability
                    for (int i = 0; i < config.popularUserFollowerMultiplier(); i++) {
                        candidates.add(popular);
                    }
                }
            }
            
            // Add regular users
            for (Long other : userIds) {
                if (!other.equals(userId) && !popularSet.contains(other)) {
                    candidates.add(other);
                }
            }
            
            // Sample follows
            Set<Long> followed = new HashSet<>();
            for (int i = 0; i < numFollows && !candidates.isEmpty(); i++) {
                Long target = candidates.get(random.nextInt(candidates.size()));
                if (followed.add(target)) {
                    batch.add(new AoeeClient.EdgeData(userId, EdgeType.FOLLOWS, target));
                    userFollowerCounts.merge(target, 1, Integer::sum);
                    count++;
                    
                    if (batch.size() >= BATCH_SIZE) {
                        client.addEdges(batch);
                        batch.clear();
                        log.debug("Generated {} follow edges...", count);
                    }
                }
            }
        }
        
        // Flush remaining
        if (!batch.isEmpty()) {
            client.addEdges(batch);
        }
        
        return count;
    }
    
    private long generateFriendEdges(BenchmarkConfig config, Random random) {
        long count = 0;
        Set<String> existingFriendships = new HashSet<>();
        List<AoeeClient.EdgeData> batch = new ArrayList<>(BATCH_SIZE);
        
        for (Long userId : userIds) {
            int numFriends = sampleCount(config.avgFriendsPerUser() / 2, config.avgFriendsPerUser(), random);
            
            for (int i = 0; i < numFriends; i++) {
                Long friendId = userIds.get(random.nextInt(userIds.size()));
                if (!friendId.equals(userId)) {
                    String key = Math.min(userId, friendId) + "-" + Math.max(userId, friendId);
                    if (existingFriendships.add(key)) {
                        // Add bidirectional friendship
                        batch.add(new AoeeClient.EdgeData(userId, EdgeType.FRIEND_OF, friendId));
                        batch.add(new AoeeClient.EdgeData(friendId, EdgeType.FRIEND_OF, userId));
                        count += 2;
                        
                        if (batch.size() >= BATCH_SIZE) {
                            client.addEdges(batch);
                            batch.clear();
                        }
                    }
                }
            }
        }
        
        // Flush remaining
        if (!batch.isEmpty()) {
            client.addEdges(batch);
        }
        
        return count;
    }
    
    private long generateLikeEdges(BenchmarkConfig config, Random random) {
        long count = 0;
        Set<Long> viralSet = new HashSet<>(viralPostIds);
        int[] reactions = {0, 1, 2, 3, 4, 5}; // like, love, haha, wow, sad, angry
        List<AoeeClient.EdgeData> batch = new ArrayList<>(BATCH_SIZE);
        
        for (Long postId : postIds) {
            // Viral posts get more likes
            int numLikes;
            if (viralSet.contains(postId)) {
                numLikes = sampleCount(config.maxLikesPerPost() / 2, config.maxLikesPerPost(), random);
            } else {
                numLikes = sampleCount(config.avgLikesPerPost(), config.avgLikesPerPost() * 3, random);
            }
            numLikes = Math.min(numLikes, userIds.size());
            
            // Sample likers
            Set<Long> likers = new HashSet<>();
            for (int i = 0; i < numLikes; i++) {
                Long likerId = userIds.get(random.nextInt(userIds.size()));
                if (likers.add(likerId)) {
                    int reaction = reactions[random.nextInt(reactions.length)];
                    batch.add(new AoeeClient.EdgeData(likerId, EdgeType.LIKES, postId, 0, reaction));
                    count++;
                    
                    if (batch.size() >= BATCH_SIZE) {
                        client.addEdges(batch);
                        batch.clear();
                        log.debug("Generated {} like edges...", count);
                    }
                }
            }
            
            postLikeCounts.put(postId, likers.size());
        }
        
        // Flush remaining
        if (!batch.isEmpty()) {
            client.addEdges(batch);
        }
        
        return count;
    }
    
    private long generateMemberEdges(BenchmarkConfig config, Random random) {
        long count = 0;
        List<AoeeClient.EdgeData> batch = new ArrayList<>(BATCH_SIZE);
        
        for (Long groupId : groupIds) {
            int numMembers = sampleCount(config.avgMembersPerGroup(), config.avgMembersPerGroup() * 2, random);
            numMembers = Math.min(numMembers, userIds.size());
            
            Set<Long> members = new HashSet<>();
            for (int i = 0; i < numMembers; i++) {
                Long memberId = userIds.get(random.nextInt(userIds.size()));
                if (members.add(memberId)) {
                    batch.add(new AoeeClient.EdgeData(memberId, EdgeType.MEMBER_OF, groupId));
                    count++;
                    
                    if (batch.size() >= BATCH_SIZE) {
                        client.addEdges(batch);
                        batch.clear();
                    }
                }
            }
        }
        
        // Flush remaining
        if (!batch.isEmpty()) {
            client.addEdges(batch);
        }
        
        return count;
    }
    
    /**
     * Persist generated entities to the database via Rust server write-through.
     */
    private void persistEntities() {
        try {
            // Persist users in batches via Rust server
            long userCount = persistEntitiesBatch(userIds, "USER", "User");
            log.info("Persisted {} user entities", userCount);
            
            // Persist posts in batches via Rust server
            long postCount = persistEntitiesBatch(postIds, "POST", "Post");
            log.info("Persisted {} post entities", postCount);
            
            // Persist groups in batches via Rust server
            long groupCount = persistEntitiesBatch(groupIds, "GROUP", "Group");
            log.info("Persisted {} group entities", groupCount);
        } catch (Exception e) {
            log.warn("Failed to persist entities to database: {}", e.getMessage());
            log.warn("Exception class: {}", e.getClass().getName());
            if (e.getCause() != null) {
                log.warn("Cause: {}", e.getCause().getMessage());
            }
        }
    }
    
    private long persistEntitiesBatch(List<Long> ids, String entityType, String namePrefix) {
        long total = 0;
        List<AoeeClient.EntityData> batch = new ArrayList<>(ENTITY_BATCH_SIZE);
        
        for (Long id : ids) {
            batch.add(new AoeeClient.EntityData(id, entityType, namePrefix + " " + id));
            
            if (batch.size() >= ENTITY_BATCH_SIZE) {
                var result = client.createEntities(batch);
                total += result.entitiesCreated();
                if (result.entitiesFailed() > 0) {
                    log.warn("{} entities failed to persist", result.entitiesFailed());
                }
                batch.clear();
            }
        }
        
        // Flush remaining
        if (!batch.isEmpty()) {
            var result = client.createEntities(batch);
            total += result.entitiesCreated();
            if (result.entitiesFailed() > 0) {
                log.warn("{} entities failed to persist", result.entitiesFailed());
            }
        }
        
        return total;
    }
    
    /**
     * Run operation benchmarks.
     */
    private List<OperationResult> runOperationBenchmarks(BenchmarkConfig config) {
        List<OperationResult> results = new ArrayList<>();
        Random random = new Random(123);
        
        // 1. Neighbor lookup (FOLLOWS)
        results.add(benchmarkNeighborLookup(config, random));
        
        // 2. Neighbor lookup with metadata (LIKES)
        results.add(benchmarkNeighborLookupWithMetadata(config, random));
        
        // 3. Contains check
        results.add(benchmarkContains(config, random));
        
        // 4. Count operation
        results.add(benchmarkCount(config, random));
        
        // 5. Intersection (mutual friends)
        results.add(benchmarkIntersection(config, random));
        
        // 6. Friend-of-Friend
        results.add(benchmarkFriendOfFriend(config, random));
        
        // 7. Large list traversal (popular users)
        results.add(benchmarkLargeListTraversal(config, random));
        
        // 8. Viral post likes
        results.add(benchmarkViralPostLikes(config, random));
        
        return results;
    }
    
    private OperationResult benchmarkNeighborLookup(BenchmarkConfig config, Random random) {
        log.info("Benchmarking neighbor lookup...");
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            client.getNeighbors(userId, EdgeType.FOLLOWS);
        }
        
        // Benchmark
        long[] latencies = new long[config.benchmarkIterations()];
        int totalNeighbors = 0;
        
        for (int i = 0; i < config.benchmarkIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            var neighbors = client.getNeighbors(userId, EdgeType.FOLLOWS);
            latencies[i] = System.nanoTime() - start;
            totalNeighbors += neighbors.size();
        }
        
        return buildOperationResult(
            "neighbors_follows",
            "Get FOLLOWS neighbors for random users",
            config.benchmarkIterations(),
            latencies,
            Map.of("avgNeighborsReturned", totalNeighbors / config.benchmarkIterations())
        );
    }
    
    private OperationResult benchmarkNeighborLookupWithMetadata(BenchmarkConfig config, Random random) {
        log.info("Benchmarking neighbor lookup with metadata...");
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            client.getNeighborsWithMetadata(userId, EdgeType.LIKES, 0, true);
        }
        
        // Benchmark
        long[] latencies = new long[config.benchmarkIterations()];
        int totalLikes = 0;
        
        for (int i = 0; i < config.benchmarkIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            var result = client.getNeighborsWithMetadata(userId, EdgeType.LIKES, 0, true);
            latencies[i] = System.nanoTime() - start;
            totalLikes += result.neighbors().size();
        }
        
        return buildOperationResult(
            "neighbors_likes_metadata",
            "Get LIKES neighbors with metadata for random users",
            config.benchmarkIterations(),
            latencies,
            Map.of("avgLikesReturned", totalLikes / config.benchmarkIterations())
        );
    }
    
    private OperationResult benchmarkContains(BenchmarkConfig config, Random random) {
        log.info("Benchmarking contains check...");
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            Long targetId = userIds.get(random.nextInt(userIds.size()));
            client.contains(userId, EdgeType.FOLLOWS, targetId);
        }
        
        // Benchmark
        long[] latencies = new long[config.benchmarkIterations()];
        int found = 0;
        
        for (int i = 0; i < config.benchmarkIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            Long targetId = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            boolean exists = client.contains(userId, EdgeType.FOLLOWS, targetId);
            latencies[i] = System.nanoTime() - start;
            if (exists) found++;
        }
        
        return buildOperationResult(
            "contains",
            "Check if FOLLOWS edge exists between random users",
            config.benchmarkIterations(),
            latencies,
            Map.of("hitRate", (double) found / config.benchmarkIterations())
        );
    }
    
    private OperationResult benchmarkCount(BenchmarkConfig config, Random random) {
        log.info("Benchmarking count operation...");
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            client.count(userId, EdgeType.FOLLOWS);
        }
        
        // Benchmark
        long[] latencies = new long[config.benchmarkIterations()];
        long totalCount = 0;
        
        for (int i = 0; i < config.benchmarkIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            long count = client.count(userId, EdgeType.FOLLOWS);
            latencies[i] = System.nanoTime() - start;
            totalCount += count;
        }
        
        return buildOperationResult(
            "count",
            "Count FOLLOWS edges for random users",
            config.benchmarkIterations(),
            latencies,
            Map.of("avgCount", totalCount / config.benchmarkIterations())
        );
    }
    
    private OperationResult benchmarkIntersection(BenchmarkConfig config, Random random) {
        log.info("Benchmarking intersection (mutual friends)...");
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long user1 = userIds.get(random.nextInt(userIds.size()));
            Long user2 = userIds.get(random.nextInt(userIds.size()));
            client.intersect(user1, EdgeType.FRIEND_OF, user2, EdgeType.FRIEND_OF);
        }
        
        // Benchmark
        long[] latencies = new long[config.benchmarkIterations()];
        int totalMutual = 0;
        
        for (int i = 0; i < config.benchmarkIterations(); i++) {
            Long user1 = userIds.get(random.nextInt(userIds.size()));
            Long user2 = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            var mutual = client.intersect(user1, EdgeType.FRIEND_OF, user2, EdgeType.FRIEND_OF);
            latencies[i] = System.nanoTime() - start;
            totalMutual += mutual.size();
        }
        
        return buildOperationResult(
            "intersection_mutual_friends",
            "Find mutual FRIEND_OF between random user pairs",
            config.benchmarkIterations(),
            latencies,
            Map.of("avgMutualFriends", totalMutual / config.benchmarkIterations())
        );
    }
    
    private OperationResult benchmarkFriendOfFriend(BenchmarkConfig config, Random random) {
        log.info("Benchmarking friend-of-friend...");
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            client.friendOfFriend(userId, EdgeType.FOLLOWS, 100, 50, 1);
        }
        
        // Benchmark
        long[] latencies = new long[config.benchmarkIterations()];
        int totalCandidates = 0;
        int truncatedCount = 0;
        
        for (int i = 0; i < config.benchmarkIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            var result = client.friendOfFriend(userId, EdgeType.FOLLOWS, 100, 50, 1);
            latencies[i] = System.nanoTime() - start;
            totalCandidates += result.getCandidatesCount();
            if (result.getTruncated()) truncatedCount++;
        }
        
        return buildOperationResult(
            "friend_of_friend",
            "FOF suggestions for random users (fanout=100, max=50)",
            config.benchmarkIterations(),
            latencies,
            Map.of(
                "avgCandidates", totalCandidates / config.benchmarkIterations(),
                "truncatedRate", (double) truncatedCount / config.benchmarkIterations()
            )
        );
    }
    
    private OperationResult benchmarkLargeListTraversal(BenchmarkConfig config, Random random) {
        log.info("Benchmarking large list traversal (popular users)...");
        
        if (popularUserIds.isEmpty()) {
            return buildOperationResult(
                "large_list_traversal",
                "No popular users to benchmark",
                0,
                new long[0],
                Map.of()
            );
        }
        
        // Warmup
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = popularUserIds.get(random.nextInt(popularUserIds.size()));
            client.getNeighbors(userId, EdgeType.FOLLOWS);
        }
        
        // Benchmark - focus on popular users with many followers
        int iterations = Math.min(config.benchmarkIterations(), popularUserIds.size() * 100);
        long[] latencies = new long[iterations];
        int maxFollowers = 0;
        int totalFollowers = 0;
        
        for (int i = 0; i < iterations; i++) {
            Long userId = popularUserIds.get(random.nextInt(popularUserIds.size()));
            long start = System.nanoTime();
            var neighbors = client.getNeighbors(userId, EdgeType.FOLLOWS);
            latencies[i] = System.nanoTime() - start;
            totalFollowers += neighbors.size();
            maxFollowers = Math.max(maxFollowers, neighbors.size());
        }
        
        return buildOperationResult(
            "large_list_popular_users",
            "Get FOLLOWS for popular users (high follower count)",
            iterations,
            latencies,
            Map.of(
                "avgFollowers", totalFollowers / iterations,
                "maxFollowers", maxFollowers,
                "popularUserCount", popularUserIds.size()
            )
        );
    }
    
    private OperationResult benchmarkViralPostLikes(BenchmarkConfig config, Random random) {
        log.info("Benchmarking viral post likes...");
        
        if (viralPostIds.isEmpty()) {
            return buildOperationResult(
                "viral_post_likes",
                "No viral posts to benchmark",
                0,
                new long[0],
                Map.of()
            );
        }
        
        // We need to query who liked these posts, but LIKES goes user->post
        // So we benchmark getting likes FOR a post by querying users who liked it
        // This simulates the common "show who liked this post" query
        
        // Warmup - query random users' likes
        for (int i = 0; i < config.warmupIterations(); i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            client.getNeighborsWithMetadata(userId, EdgeType.LIKES, 0, true);
        }
        
        // For viral posts, we track like counts
        int iterations = Math.min(config.benchmarkIterations(), viralPostIds.size() * 100);
        long[] latencies = new long[iterations];
        int maxLikes = 0;
        
        // Benchmark getting likes from users who might have liked viral posts
        for (int i = 0; i < iterations; i++) {
            Long userId = userIds.get(random.nextInt(userIds.size()));
            long start = System.nanoTime();
            var result = client.getNeighborsWithMetadata(userId, EdgeType.LIKES, 0, true);
            latencies[i] = System.nanoTime() - start;
            maxLikes = Math.max(maxLikes, result.neighbors().size());
        }
        
        // Report on viral post like counts
        int maxViralLikes = viralPostIds.stream()
            .mapToInt(id -> postLikeCounts.getOrDefault(id, 0))
            .max()
            .orElse(0);
        
        return buildOperationResult(
            "viral_post_query",
            "Query LIKES with metadata (simulating popular content queries)",
            iterations,
            latencies,
            Map.of(
                "viralPostCount", viralPostIds.size(),
                "maxLikesOnViralPost", maxViralLikes
            )
        );
    }
    
    // Helper methods
    
    private int sampleCount(int avg, int max, Random random) {
        // Simple triangular distribution around average
        double u = random.nextDouble();
        return Math.min((int) (avg * (0.5 + u)), max);
    }
    
    private OperationResult buildOperationResult(
        String operation,
        String description,
        int iterations,
        long[] latenciesNanos,
        Map<String, Object> details
    ) {
        if (iterations == 0 || latenciesNanos.length == 0) {
            return new OperationResult(
                operation, description, 0,
                new LatencyStats(0, 0, 0, 0, 0, 0, 0, 0),
                0, details
            );
        }
        
        // Convert to microseconds and sort
        double[] latencies = Arrays.stream(latenciesNanos)
            .mapToDouble(l -> l / 1000.0)
            .sorted()
            .toArray();
        
        double min = latencies[0];
        double max = latencies[latencies.length - 1];
        double mean = Arrays.stream(latencies).average().orElse(0);
        double median = percentile(latencies, 50);
        double p90 = percentile(latencies, 90);
        double p95 = percentile(latencies, 95);
        double p99 = percentile(latencies, 99);
        double stdDev = stdDev(latencies, mean);
        
        // throughput = iterations / total_seconds
        // total_seconds = sum_nanos / 1e9
        // so: throughput = iterations * 1e9 / sum_nanos
        double throughput = iterations * 1_000_000_000.0 / Arrays.stream(latenciesNanos).sum();
        
        return new OperationResult(
            operation,
            description,
            iterations,
            new LatencyStats(min, max, mean, median, p90, p95, p99, stdDev),
            throughput,
            details
        );
    }
    
    private double percentile(double[] sorted, int p) {
        int index = (int) Math.ceil(p / 100.0 * sorted.length) - 1;
        return sorted[Math.max(0, Math.min(index, sorted.length - 1))];
    }
    
    private double stdDev(double[] values, double mean) {
        double sumSquares = Arrays.stream(values)
            .map(v -> (v - mean) * (v - mean))
            .sum();
        return Math.sqrt(sumSquares / values.length);
    }
    
    private String buildSummary(BenchmarkConfig config, DataGenerationStats dataStats, 
                                List<OperationResult> operations) {
        StringBuilder sb = new StringBuilder();
        sb.append("=== AOEE Benchmark Summary ===\n\n");
        
        sb.append("Data Generation:\n");
        sb.append(String.format("  - %,d users, %,d posts, %,d groups\n", 
            dataStats.usersCreated(), dataStats.postsCreated(), dataStats.groupsCreated()));
        sb.append(String.format("  - %,d total edges in %,d ms (%.0f edges/sec)\n",
            dataStats.totalEdges(), dataStats.durationMs(), dataStats.edgesPerSecond()));
        sb.append(String.format("  - Edge breakdown: %,d follows, %,d friends, %,d likes, %,d members\n",
            dataStats.followEdges(), dataStats.friendEdges(), 
            dataStats.likeEdges(), dataStats.memberEdges()));
        
        sb.append("\nOperation Performance (latency in µs):\n");
        sb.append(String.format("  %-30s %10s %10s %10s %10s %12s\n",
            "Operation", "Median", "P95", "P99", "Max", "Throughput"));
        sb.append("  " + "-".repeat(84) + "\n");
        
        for (OperationResult op : operations) {
            if (op.iterations() > 0) {
                sb.append(String.format("  %-30s %10.1f %10.1f %10.1f %10.1f %10.0f/s\n",
                    op.operation(),
                    op.latency().median(),
                    op.latency().p95(),
                    op.latency().p99(),
                    op.latency().max(),
                    op.throughput()));
            }
        }
        
        return sb.toString();
    }
    
    private void clearTracking() {
        userIds.clear();
        postIds.clear();
        groupIds.clear();
        popularUserIds.clear();
        viralPostIds.clear();
        userFollowerCounts.clear();
        postLikeCounts.clear();
    }
    
    /**
     * Get current data statistics.
     */
    public Map<String, Object> getDataStats() {
        return Map.of(
            "users", userIds.size(),
            "posts", postIds.size(),
            "groups", groupIds.size(),
            "popularUsers", popularUserIds.size(),
            "viralPosts", viralPostIds.size()
        );
    }
}

package com.aoee.spring.benchmark;

/**
 * Configuration for benchmark data generation.
 * 
 * The generator creates a social network with:
 * - Users with follow relationships (power-law distribution for popularity)
 * - Posts authored by users
 * - Likes on posts (with reactions)
 * - Friend relationships
 * - Group memberships
 */
public record BenchmarkConfig(
    // User configuration
    int numUsers,                    // Total number of users
    int avgFollowsPerUser,           // Average number of users each user follows
    int maxFollowsPerUser,           // Max follows (for popular users)
    double popularUserRatio,         // Ratio of "celebrity" users (0.01 = 1%)
    int popularUserFollowerMultiplier, // How many more followers celebrities get
    
    // Content configuration
    int numPosts,                    // Total number of posts
    int avgLikesPerPost,             // Average likes per post
    int maxLikesPerPost,             // Max likes (for viral posts)
    double viralPostRatio,           // Ratio of viral posts
    
    // Social configuration
    int avgFriendsPerUser,           // Average friend count (bidirectional)
    int numGroups,                   // Number of groups
    int avgMembersPerGroup,          // Average group membership
    
    // Benchmark configuration
    int warmupIterations,            // Warmup iterations before measuring
    int benchmarkIterations,         // Number of iterations to measure
    boolean includeMetadata          // Whether to include metadata in queries
) {
    /**
     * Small benchmark: ~1K users, quick to run
     */
    public static BenchmarkConfig small() {
        return new BenchmarkConfig(
            1_000,      // users
            50,         // avg follows
            200,        // max follows
            0.02,       // 2% popular users
            5,          // 5x followers for celebrities
            2_000,      // posts
            20,         // avg likes
            500,        // max likes
            0.05,       // 5% viral posts
            20,         // avg friends
            10,         // groups
            100,        // avg members/group
            100,        // warmup
            1000,       // benchmark iterations
            true
        );
    }
    
    /**
     * Medium benchmark: ~10K users
     */
    public static BenchmarkConfig medium() {
        return new BenchmarkConfig(
            10_000,     // users
            100,        // avg follows
            2_000,      // max follows
            0.01,       // 1% popular users
            10,         // 10x followers for celebrities
            20_000,     // posts
            50,         // avg likes
            5_000,      // max likes
            0.02,       // 2% viral posts
            50,         // avg friends
            50,         // groups
            500,        // avg members/group
            200,        // warmup
            5000,       // benchmark iterations
            true
        );
    }
    
    /**
     * Large benchmark: ~100K users, stress test
     */
    public static BenchmarkConfig large() {
        return new BenchmarkConfig(
            100_000,    // users
            200,        // avg follows
            10_000,     // max follows (celebrities with 10K followers)
            0.005,      // 0.5% popular users
            20,         // 20x followers for celebrities
            200_000,    // posts
            100,        // avg likes
            50_000,     // max likes (viral posts with 50K likes)
            0.01,       // 1% viral posts
            100,        // avg friends
            200,        // groups
            2_000,      // avg members/group
            500,        // warmup
            10000,      // benchmark iterations
            true
        );
    }
    
    /**
     * Custom configuration builder
     */
    public static Builder builder() {
        return new Builder();
    }
    
    public static class Builder {
        private int numUsers = 10_000;
        private int avgFollowsPerUser = 100;
        private int maxFollowsPerUser = 2_000;
        private double popularUserRatio = 0.01;
        private int popularUserFollowerMultiplier = 10;
        private int numPosts = 20_000;
        private int avgLikesPerPost = 50;
        private int maxLikesPerPost = 5_000;
        private double viralPostRatio = 0.02;
        private int avgFriendsPerUser = 50;
        private int numGroups = 50;
        private int avgMembersPerGroup = 500;
        private int warmupIterations = 200;
        private int benchmarkIterations = 5000;
        private boolean includeMetadata = true;
        
        public Builder numUsers(int n) { this.numUsers = n; return this; }
        public Builder avgFollowsPerUser(int n) { this.avgFollowsPerUser = n; return this; }
        public Builder maxFollowsPerUser(int n) { this.maxFollowsPerUser = n; return this; }
        public Builder popularUserRatio(double r) { this.popularUserRatio = r; return this; }
        public Builder popularUserFollowerMultiplier(int m) { this.popularUserFollowerMultiplier = m; return this; }
        public Builder numPosts(int n) { this.numPosts = n; return this; }
        public Builder avgLikesPerPost(int n) { this.avgLikesPerPost = n; return this; }
        public Builder maxLikesPerPost(int n) { this.maxLikesPerPost = n; return this; }
        public Builder viralPostRatio(double r) { this.viralPostRatio = r; return this; }
        public Builder avgFriendsPerUser(int n) { this.avgFriendsPerUser = n; return this; }
        public Builder numGroups(int n) { this.numGroups = n; return this; }
        public Builder avgMembersPerGroup(int n) { this.avgMembersPerGroup = n; return this; }
        public Builder warmupIterations(int n) { this.warmupIterations = n; return this; }
        public Builder benchmarkIterations(int n) { this.benchmarkIterations = n; return this; }
        public Builder includeMetadata(boolean b) { this.includeMetadata = b; return this; }
        
        public BenchmarkConfig build() {
            return new BenchmarkConfig(
                numUsers, avgFollowsPerUser, maxFollowsPerUser,
                popularUserRatio, popularUserFollowerMultiplier,
                numPosts, avgLikesPerPost, maxLikesPerPost, viralPostRatio,
                avgFriendsPerUser, numGroups, avgMembersPerGroup,
                warmupIterations, benchmarkIterations, includeMetadata
            );
        }
    }
}

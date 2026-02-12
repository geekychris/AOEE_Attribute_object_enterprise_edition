package com.aoee.client;

import aoee.AoeeGrpc;
import aoee.AoeeOuterClass.*;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.List;
import java.util.concurrent.TimeUnit;

/**
 * Java client for AOEE (Attribute Object Enterprise Edition) gRPC service.
 * Provides methods for edge operations, queries, and friend-of-friend lookups.
 */
public class AoeeClient implements AutoCloseable {
    private static final Logger logger = LoggerFactory.getLogger(AoeeClient.class);

    private final ManagedChannel channel;
    private final AoeeGrpc.AoeeBlockingStub blockingStub;
    private final String host;
    private final int port;

    /**
     * Create a new AOEE client connecting to the specified host and port.
     */
    public AoeeClient(String host, int port) {
        this.host = host;
        this.port = port;
        this.channel = ManagedChannelBuilder.forAddress(host, port)
                .usePlaintext()
                .build();
        this.blockingStub = AoeeGrpc.newBlockingStub(channel);
        logger.info("Created AOEE client connecting to {}:{}", host, port);
    }

    /**
     * Create a client connecting to localhost:50051 (default).
     */
    public AoeeClient() {
        this("localhost", 50051);
    }

    // ========================================================================
    // Edge Operations
    // ========================================================================

    /**
     * Add an edge from src to dst with the given edge type.
     *
     * @param src Source entity ID
     * @param edgeType Edge type (e.g., 0=Follows, 10=Likes, etc.)
     * @param dst Destination entity ID
     * @return timestamp assigned to the edge (nanoseconds since epoch)
     */
    public long addEdge(long src, int edgeType, long dst) {
        return addEdge(src, edgeType, dst, 0, 0);
    }

    /**
     * Add an edge with timestamp and metadata.
     *
     * @param src Source entity ID
     * @param edgeType Edge type
     * @param dst Destination entity ID
     * @param timestamp Timestamp (0 = auto-generate)
     * @param metadata Metadata byte (meaning depends on edge type, e.g., reaction type for LIKES)
     * @return timestamp assigned to the edge
     */
    public long addEdge(long src, int edgeType, long dst, long timestamp, int metadata) {
        try {
            EdgeKey key = EdgeKey.newBuilder()
                    .setSrc(src)
                    .setEdgeType(edgeType)
                    .build();

            AddEdgeRequest request = AddEdgeRequest.newBuilder()
                    .setKey(key)
                    .setDst(dst)
                    .setTimestamp(timestamp)
                    .setMetadata(metadata)
                    .build();

            AddEdgeResponse response = blockingStub.addEdge(request);
            return response.getTimestamp();
        } catch (StatusRuntimeException e) {
            logger.error("Failed to add edge: {} -> {} (type {})", src, dst, edgeType, e);
            throw new AoeeClientException("Failed to add edge", e);
        }
    }

    /**
     * Delete an edge from src to dst.
     */
    public boolean deleteEdge(long src, int edgeType, long dst) {
        try {
            EdgeKey key = EdgeKey.newBuilder()
                    .setSrc(src)
                    .setEdgeType(edgeType)
                    .build();

            DeleteEdgeRequest request = DeleteEdgeRequest.newBuilder()
                    .setKey(key)
                    .setDst(dst)
                    .build();

            DeleteEdgeResponse response = blockingStub.deleteEdge(request);
            return response.getSuccess();
        } catch (StatusRuntimeException e) {
            logger.error("Failed to delete edge: {} -> {} (type {})", src, dst, edgeType, e);
            throw new AoeeClientException("Failed to delete edge", e);
        }
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /**
     * Get all neighbors (destinations) for a source entity and edge type.
     */
    public List<Long> getNeighbors(long src, int edgeType) {
        return getNeighbors(src, edgeType, 0);
    }

    /**
     * Get neighbors with a limit.
     */
    public List<Long> getNeighbors(long src, int edgeType, int limit) {
        return getNeighborsWithMetadata(src, edgeType, limit, false).neighbors();
    }

    /**
     * Get neighbors with optional metadata.
     *
     * @param src Source entity ID
     * @param edgeType Edge type
     * @param limit Max results (0 = no limit)
     * @param includeMetadata If true, includes timestamps and metadata in response
     * @return NeighborsResult with neighbors and optional metadata
     */
    public NeighborsResult getNeighborsWithMetadata(long src, int edgeType, int limit, boolean includeMetadata) {
        try {
            EdgeKey key = EdgeKey.newBuilder()
                    .setSrc(src)
                    .setEdgeType(edgeType)
                    .build();

            NeighborsRequest request = NeighborsRequest.newBuilder()
                    .setKey(key)
                    .setLimit(limit)
                    .setIncludeMetadata(includeMetadata)
                    .build();

            NeighborsResponse response = blockingStub.neighbors(request);
            return new NeighborsResult(
                response.getNeighborsList(),
                response.getTimestampsList(),
                response.getMetadataList()
            );
        } catch (StatusRuntimeException e) {
            logger.error("Failed to get neighbors for {} (type {})", src, edgeType, e);
            throw new AoeeClientException("Failed to get neighbors", e);
        }
    }

    /**
     * Result of a neighbors query with optional metadata.
     * Note: metadata values are stored as uint32 in proto but represent a single byte (0-255).
     */
    public record NeighborsResult(
        List<Long> neighbors,
        List<Long> timestamps,
        List<Integer> metadata
    ) {
        public int getMetadataAt(int index) {
            return metadata != null && index < metadata.size() ? metadata.get(index) : 0;
        }
    }

    /**
     * Check if an edge exists.
     */
    public boolean contains(long src, int edgeType, long dst) {
        try {
            EdgeKey key = EdgeKey.newBuilder()
                    .setSrc(src)
                    .setEdgeType(edgeType)
                    .build();

            ContainsRequest request = ContainsRequest.newBuilder()
                    .setKey(key)
                    .setDst(dst)
                    .build();

            ContainsResponse response = blockingStub.contains(request);
            return response.getExists();
        } catch (StatusRuntimeException e) {
            logger.error("Failed to check contains for {} -> {} (type {})", src, dst, edgeType, e);
            throw new AoeeClientException("Failed to check contains", e);
        }
    }

    /**
     * Get the count of edges for a source entity and edge type.
     */
    public long count(long src, int edgeType) {
        try {
            EdgeKey key = EdgeKey.newBuilder()
                    .setSrc(src)
                    .setEdgeType(edgeType)
                    .build();

            CountRequest request = CountRequest.newBuilder()
                    .setKey(key)
                    .build();

            CountResponse response = blockingStub.count(request);
            return response.getCount();
        } catch (StatusRuntimeException e) {
            logger.error("Failed to get count for {} (type {})", src, edgeType, e);
            throw new AoeeClientException("Failed to get count", e);
        }
    }

    // ========================================================================
    // Set Operations
    // ========================================================================

    /**
     * Intersect two edge lists (find common neighbors).
     */
    public List<Long> intersect(long src1, int edgeType1, long src2, int edgeType2) {
        try {
            EdgeKey key1 = EdgeKey.newBuilder()
                    .setSrc(src1)
                    .setEdgeType(edgeType1)
                    .build();
            EdgeKey key2 = EdgeKey.newBuilder()
                    .setSrc(src2)
                    .setEdgeType(edgeType2)
                    .build();

            IntersectRequest request = IntersectRequest.newBuilder()
                    .setKey1(key1)
                    .setKey2(key2)
                    .build();

            IntersectResponse response = blockingStub.intersect(request);
            return response.getIdsList();
        } catch (StatusRuntimeException e) {
            logger.error("Failed to intersect {}:{} with {}:{}", src1, edgeType1, src2, edgeType2, e);
            throw new AoeeClientException("Failed to intersect", e);
        }
    }

    /**
     * Union two edge lists (combine all neighbors).
     */
    public List<Long> union(long src1, int edgeType1, long src2, int edgeType2) {
        try {
            EdgeKey key1 = EdgeKey.newBuilder()
                    .setSrc(src1)
                    .setEdgeType(edgeType1)
                    .build();
            EdgeKey key2 = EdgeKey.newBuilder()
                    .setSrc(src2)
                    .setEdgeType(edgeType2)
                    .build();

            UnionRequest request = UnionRequest.newBuilder()
                    .setKey1(key1)
                    .setKey2(key2)
                    .build();

            UnionResponse response = blockingStub.union(request);
            return response.getIdsList();
        } catch (StatusRuntimeException e) {
            logger.error("Failed to union {}:{} with {}:{}", src1, edgeType1, src2, edgeType2, e);
            throw new AoeeClientException("Failed to union", e);
        }
    }

    // ========================================================================
    // Friend-of-Friend
    // ========================================================================

    /**
     * Find friend-of-friend candidates with default settings.
     */
    public FofResult friendOfFriend(long source, int edgeType) {
        return friendOfFriend(source, edgeType, 0, 0, 0, List.of());
    }

    /**
     * Find friend-of-friend candidates with full configuration.
     *
     * @param source Source entity ID
     * @param edgeType Edge type to follow
     * @param fanoutCap Maximum number of direct friends to process (0 = default 1000)
     * @param maxResults Maximum number of results (0 = default 100)
     * @param minScore Minimum mutual friend count (0 = default 1)
     * @param exclusions IDs to exclude from results
     * @return FOF result with candidates and metadata
     */
    public FofResult friendOfFriend(long source, int edgeType, int fanoutCap,
                                     int maxResults, int minScore, List<Long> exclusions) {
        try {
            FofRequest.Builder requestBuilder = FofRequest.newBuilder()
                    .setSource(source)
                    .setEdgeType(edgeType);

            if (fanoutCap > 0) requestBuilder.setFanoutCap(fanoutCap);
            if (maxResults > 0) requestBuilder.setMaxResults(maxResults);
            if (minScore > 0) requestBuilder.setMinScore(minScore);
            if (exclusions != null && !exclusions.isEmpty()) {
                requestBuilder.addAllExclusions(exclusions);
            }

            FofResponse response = blockingStub.friendOfFriend(requestBuilder.build());

            List<FofResult.Candidate> candidates = response.getCandidatesList().stream()
                    .map(c -> new FofResult.Candidate(c.getId(), c.getScore()))
                    .toList();

            return new FofResult(candidates, response.getTruncated(), response.getElapsedMs());
        } catch (StatusRuntimeException e) {
            logger.error("Failed to get FOF for {} (type {})", source, edgeType, e);
            throw new AoeeClientException("Failed to get friend-of-friend", e);
        }
    }

    // ========================================================================
    // Stats
    // ========================================================================

    /**
     * Get aggregated server statistics.
     */
    public StatsResult getStats() {
        return getStats(false);
    }

    /**
     * Get server statistics, optionally including per-shard details.
     */
    public StatsResult getStats(boolean perShard) {
        try {
            StatsRequest request = StatsRequest.newBuilder()
                    .setPerShard(perShard)
                    .build();

            StatsResponse response = blockingStub.stats(request);

            StatsResult.ShardStats aggregated = null;
            if (response.hasAggregated()) {
                ShardStats s = response.getAggregated();
                aggregated = new StatsResult.ShardStats(
                        s.getShardId(), s.getCachedLists(), s.getTotalEdges(),
                        s.getReads(), s.getWrites(), s.getCacheHits(), s.getCacheMisses()
                );
            }

            List<StatsResult.ShardStats> perShardStats = response.getPerShardList().stream()
                    .map(s -> new StatsResult.ShardStats(
                            s.getShardId(), s.getCachedLists(), s.getTotalEdges(),
                            s.getReads(), s.getWrites(), s.getCacheHits(), s.getCacheMisses()
                    ))
                    .toList();

            return new StatsResult(aggregated, perShardStats);
        } catch (StatusRuntimeException e) {
            logger.error("Failed to get stats", e);
            throw new AoeeClientException("Failed to get stats", e);
        }
    }

    // ========================================================================
    // Connection Management
    // ========================================================================

    /**
     * Check if the client can connect to the server.
     */
    public boolean isConnected() {
        try {
            getStats(false);
            return true;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * Get the connection target.
     */
    public String getTarget() {
        return host + ":" + port;
    }

    @Override
    public void close() {
        try {
            channel.shutdown().awaitTermination(5, TimeUnit.SECONDS);
            logger.info("AOEE client closed");
        } catch (InterruptedException e) {
            logger.warn("Interrupted while closing AOEE client", e);
            Thread.currentThread().interrupt();
        }
    }
}

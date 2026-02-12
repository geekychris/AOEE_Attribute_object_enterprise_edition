package com.aoee.spring.model;

import java.util.List;

/**
 * Response containing neighbors and optional metadata.
 * 
 * @param src Source entity ID
 * @param edgeType Edge type name
 * @param neighbors List of destination entity IDs
 * @param timestamps Optional parallel array of timestamps (only if requested)
 * @param metadata Optional parallel array of metadata bytes (only if requested)
 */
public record NeighborsResponse(
    long src, 
    String edgeType, 
    List<Long> neighbors,
    List<Long> timestamps,   // Parallel to neighbors, may be null
    List<Integer> metadata   // Parallel to neighbors, may be null
) {
    public NeighborsResponse(long src, String edgeType, List<Long> neighbors) {
        this(src, edgeType, neighbors, null, null);
    }
}

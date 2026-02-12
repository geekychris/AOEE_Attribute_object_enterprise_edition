package com.aoee.spring.model;

/**
 * Request to add or delete an edge.
 * 
 * @param src Source entity ID
 * @param edgeType Edge type name (e.g., "FOLLOWS", "LIKES")
 * @param dst Destination entity ID
 * @param timestamp Optional timestamp (nanoseconds since epoch, 0 = auto-generate)
 * @param metadata Optional metadata byte (e.g., reaction type for LIKES: 0=like, 1=love, 2=haha, 3=wow, 4=sad, 5=angry)
 */
public record EdgeRequest(
    long src, 
    String edgeType, 
    long dst,
    Long timestamp,  // Optional - null or 0 = auto-generate
    Integer metadata // Optional - null or absent = 0
) {
    public long getTimestamp() {
        return timestamp != null ? timestamp : 0L;
    }
    
    public int getMetadata() {
        return metadata != null ? metadata : 0;
    }
}

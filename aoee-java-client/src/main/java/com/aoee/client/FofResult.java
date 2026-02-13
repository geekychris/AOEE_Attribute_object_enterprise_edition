package com.aoee.client;

import java.util.List;

/**
 * Result of a Friend-of-Friend query.
 */
public record FofResult(
        List<Candidate> candidates,
        boolean truncated,
        long elapsedMs
) {
    /**
     * A FOF candidate with their mutual friend count (score).
     */
    public record Candidate(long id, int score) {}
    
    /**
     * Get the number of candidates returned.
     */
    public int getCandidatesCount() {
        return candidates != null ? candidates.size() : 0;
    }
    
    /**
     * Whether the query was truncated due to limits.
     */
    public boolean getTruncated() {
        return truncated;
    }
    
    /**
     * Get elapsed time in milliseconds.
     */
    public long getElapsedMs() {
        return elapsedMs;
    }
}

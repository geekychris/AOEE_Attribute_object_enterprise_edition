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
}

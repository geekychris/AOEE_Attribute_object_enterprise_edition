package com.aoee.spring.model;

import java.util.List;

public record FofResponse(
        long source,
        List<Candidate> candidates,
        boolean truncated,
        long elapsedMs
) {
    public record Candidate(long id, int score) {}
}

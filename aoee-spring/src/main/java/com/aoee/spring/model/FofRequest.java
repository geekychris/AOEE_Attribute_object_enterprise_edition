package com.aoee.spring.model;

import java.util.List;

public record FofRequest(
        long source,
        String edgeType,
        Integer fanoutCap,
        Integer maxResults,
        Integer minScore,
        List<Long> exclusions
) {}

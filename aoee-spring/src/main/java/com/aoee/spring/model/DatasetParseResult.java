package com.aoee.spring.model;

import java.util.List;
import java.util.Map;

public record DatasetParseResult(
        boolean valid,
        int entityCount,
        int edgeCount,
        Map<String, Integer> entitiesByType,
        Map<String, Integer> edgesByType,
        List<String> errors
) {}

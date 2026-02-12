package com.aoee.spring.model;

import java.util.List;

public record DatasetLoadResponse(
        boolean success,
        int entitiesLoaded,
        int edgesLoaded,
        int errors,
        List<String> errorMessages,
        long elapsedMs
) {}

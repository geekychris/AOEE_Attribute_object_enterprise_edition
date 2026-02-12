package com.aoee.spring.model;

public record SetOperationRequest(
        long src1, String edgeType1,
        long src2, String edgeType2
) {}

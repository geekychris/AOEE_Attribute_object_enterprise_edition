package com.aoee.spring.model;

public record ContainsResponse(long src, String edgeType, long dst, boolean exists) {}

package com.aoee.spring.model;

import java.util.List;

public record SetOperationResponse(String operation, List<Long> ids) {}

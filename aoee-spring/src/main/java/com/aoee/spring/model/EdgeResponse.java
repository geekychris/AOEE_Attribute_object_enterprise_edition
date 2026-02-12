package com.aoee.spring.model;

/**
 * Response from edge operations.
 * 
 * @param success Whether the operation succeeded
 * @param message Status message
 * @param timestamp The timestamp assigned to the edge (nanoseconds since epoch)
 */
public record EdgeResponse(boolean success, String message, Long timestamp) {
    public EdgeResponse(boolean success, String message) {
        this(success, message, null);
    }
}

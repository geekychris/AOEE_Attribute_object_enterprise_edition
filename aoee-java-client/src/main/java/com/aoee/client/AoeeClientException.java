package com.aoee.client;

/**
 * Exception thrown by the AOEE client when operations fail.
 */
public class AoeeClientException extends RuntimeException {
    public AoeeClientException(String message) {
        super(message);
    }

    public AoeeClientException(String message, Throwable cause) {
        super(message, cause);
    }
}

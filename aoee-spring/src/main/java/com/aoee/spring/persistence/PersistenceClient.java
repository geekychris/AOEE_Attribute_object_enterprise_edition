package com.aoee.spring.persistence;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.http.MediaType;
import org.springframework.stereotype.Component;
import org.springframework.web.client.RestClient;
import org.springframework.web.client.RestClientException;

import java.util.Map;

/**
 * HTTP client for communicating with the aoee-persistence service.
 * Handles write-through operations to persist edges to the database.
 */
@Component
public class PersistenceClient {
    
    private static final Logger logger = LoggerFactory.getLogger(PersistenceClient.class);
    
    private final PersistenceConfig config;
    private final RestClient restClient;
    
    public PersistenceClient(PersistenceConfig config) {
        this.config = config;
        this.restClient = RestClient.builder()
            .baseUrl(config.getUrl())
            .build();
    }
    
    /**
     * Persist an edge to the database.
     * Called during write-through on addEdge operations.
     */
    public boolean persistEdge(long src, String edgeType, long dst, Long timestampNs, Integer metadata) {
        if (!config.isEnabled() || !config.isWriteThrough()) {
            return true;
        }
        
        try {
            Map<String, Object> request = Map.of(
                "src", src,
                "edgeType", edgeType.toUpperCase(),
                "dst", dst,
                "timestampNs", timestampNs != null ? timestampNs : 0L,
                "metadata", metadata != null ? metadata : 0
            );
            
            restClient.post()
                .uri("/api/v1/edges")
                .contentType(MediaType.APPLICATION_JSON)
                .body(request)
                .retrieve()
                .toBodilessEntity();
            
            logger.debug("Persisted edge: {} -[{}]-> {}", src, edgeType, dst);
            return true;
        } catch (RestClientException e) {
            logger.warn("Failed to persist edge to database: {} -[{}]-> {} - {}", 
                src, edgeType, dst, e.getMessage());
            return false;
        }
    }
    
    /**
     * Delete an edge from the database.
     * Called during write-through on deleteEdge operations.
     */
    public boolean deleteEdge(long src, String edgeType, long dst) {
        if (!config.isEnabled() || !config.isWriteThrough()) {
            return true;
        }
        
        try {
            restClient.delete()
                .uri("/api/v1/edges/{src}/{edgeType}/{dst}", src, edgeType.toUpperCase(), dst)
                .retrieve()
                .toBodilessEntity();
            
            logger.debug("Deleted edge from persistence: {} -[{}]-> {}", src, edgeType, dst);
            return true;
        } catch (RestClientException e) {
            logger.warn("Failed to delete edge from database: {} -[{}]-> {} - {}", 
                src, edgeType, dst, e.getMessage());
            return false;
        }
    }
    
    /**
     * Export all edges from persistence in AOEE dataset format.
     */
    public String exportEdges() {
        if (!config.isEnabled()) {
            throw new IllegalStateException("Persistence is not enabled");
        }
        
        try {
            return restClient.get()
                .uri("/api/v1/export/edges")
                .retrieve()
                .body(String.class);
        } catch (RestClientException e) {
            logger.error("Failed to export edges from persistence: {}", e.getMessage());
            throw new RuntimeException("Failed to export edges from persistence", e);
        }
    }
    
    /**
     * Get stats from the persistence service.
     */
    @SuppressWarnings("unchecked")
    public Map<String, Object> getStats() {
        if (!config.isEnabled()) {
            return Map.of("enabled", false);
        }
        
        try {
            return restClient.get()
                .uri("/api/v1/export/stats")
                .retrieve()
                .body(Map.class);
        } catch (RestClientException e) {
            logger.warn("Failed to get stats from persistence: {}", e.getMessage());
            return Map.of("enabled", true, "error", e.getMessage());
        }
    }
    
    /**
     * Check if persistence service is available.
     */
    public boolean isAvailable() {
        if (!config.isEnabled()) {
            return false;
        }
        
        try {
            restClient.get()
                .uri("/actuator/health")
                .retrieve()
                .toBodilessEntity();
            return true;
        } catch (Exception e) {
            return false;
        }
    }
    
    public boolean isEnabled() {
        return config.isEnabled();
    }
    
    public boolean isWriteThrough() {
        return config.isWriteThrough();
    }
}

package com.aoee.spring.controller;

import com.aoee.client.AoeeClient;
import com.aoee.client.EdgeType;
import com.aoee.spring.persistence.PersistenceClient;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Controller for cache management operations including warming from persistence.
 */
@RestController
@RequestMapping("/api/cache")
public class CacheController {
    
    private static final Logger logger = LoggerFactory.getLogger(CacheController.class);
    
    private final AoeeClient aoeeClient;
    private final PersistenceClient persistenceClient;
    
    // Pattern for parsing dataset lines
    // Format: EDGE_TYPE [metadata] SRC_ID DST_ID [reaction_name]
    private static final Pattern EDGE_PATTERN = Pattern.compile(
        "^(\\w+)\\s+(\\d+)\\s+(\\d+)(?:\\s+(\\w+))?$|^(\\w+)\\s+(\\d+)\\s+(\\d+)\\s+(\\d+)(?:\\s+(\\w+))?$"
    );
    
    public CacheController(AoeeClient aoeeClient, PersistenceClient persistenceClient) {
        this.aoeeClient = aoeeClient;
        this.persistenceClient = persistenceClient;
    }
    
    /**
     * Warm the AOEE cache from the persistence service.
     * Loads all persisted edges into the in-memory cache.
     */
    @PostMapping("/warm")
    public ResponseEntity<Map<String, Object>> warmCache() {
        if (!persistenceClient.isEnabled()) {
            return ResponseEntity.ok(Map.of(
                "success", false,
                "message", "Persistence is not enabled"
            ));
        }
        
        logger.info("Starting cache warming from persistence...");
        long startTime = System.currentTimeMillis();
        
        try {
            String dataset = persistenceClient.exportEdges();
            int edgesLoaded = loadDatasetIntoCache(dataset);
            
            long elapsed = System.currentTimeMillis() - startTime;
            logger.info("Cache warming complete: {} edges loaded in {}ms", edgesLoaded, elapsed);
            
            return ResponseEntity.ok(Map.of(
                "success", true,
                "edgesLoaded", edgesLoaded,
                "elapsedMs", elapsed
            ));
        } catch (Exception e) {
            logger.error("Cache warming failed", e);
            return ResponseEntity.ok(Map.of(
                "success", false,
                "message", e.getMessage()
            ));
        }
    }
    
    /**
     * Flush the AOEE cache to storage.
     * 
     * @param clearAfterFlush If true, also clears the cache after flushing
     */
    @PostMapping("/flush")
    public ResponseEntity<Map<String, Object>> flushCache(
            @RequestParam(defaultValue = "false") boolean clearAfterFlush) {
        logger.info("Flushing AOEE cache (clearAfterFlush={})", clearAfterFlush);
        
        try {
            var result = aoeeClient.flushCache(clearAfterFlush);
            logger.info("Cache flush complete: {} entries flushed", result.entriesAffected());
            
            return ResponseEntity.ok(Map.of(
                "success", result.success(),
                "entriesFlushed", result.entriesAffected(),
                "clearedAfterFlush", clearAfterFlush
            ));
        } catch (Exception e) {
            logger.error("Cache flush failed", e);
            return ResponseEntity.ok(Map.of(
                "success", false,
                "message", e.getMessage()
            ));
        }
    }
    
    /**
     * Clear the AOEE cache (removes all entries from memory).
     */
    @PostMapping("/clear")
    public ResponseEntity<Map<String, Object>> clearCache(
            @RequestParam(defaultValue = "0") int shardId) {
        logger.info("Clearing AOEE cache (shardId={})", shardId);
        
        try {
            var result = aoeeClient.clearCache(shardId);
            logger.info("Cache clear complete: {} entries cleared", result.entriesAffected());
            
            return ResponseEntity.ok(Map.of(
                "success", result.success(),
                "entriesCleared", result.entriesAffected(),
                "shardId", shardId
            ));
        } catch (Exception e) {
            logger.error("Cache clear failed", e);
            return ResponseEntity.ok(Map.of(
                "success", false,
                "message", e.getMessage()
            ));
        }
    }
    
    /**
     * Get persistence status and stats.
     */
    @GetMapping("/persistence/status")
    public ResponseEntity<Map<String, Object>> getPersistenceStatus() {
        boolean enabled = persistenceClient.isEnabled();
        boolean available = persistenceClient.isAvailable();
        
        Map<String, Object> response = new java.util.HashMap<>();
        response.put("enabled", enabled);
        response.put("available", available);
        response.put("writeThrough", persistenceClient.isWriteThrough());
        
        if (enabled && available) {
            response.put("stats", persistenceClient.getStats());
        }
        
        return ResponseEntity.ok(response);
    }
    
    /**
     * Parse and load a dataset string into the AOEE cache.
     */
    private int loadDatasetIntoCache(String dataset) {
        int count = 0;
        String[] lines = dataset.split("\n");
        
        for (String line : lines) {
            line = line.trim();
            if (line.isEmpty() || line.startsWith("#")) {
                continue;
            }
            
            try {
                String[] parts = line.split("\\s+");
                if (parts.length < 3) {
                    continue;
                }
                
                String edgeTypeStr = parts[0].toUpperCase();
                int idx = 1;
                int metadata = 0;
                
                // Check if second part is metadata for LIKES
                if ("LIKES".equals(edgeTypeStr) && parts.length >= 4) {
                    try {
                        int possibleMeta = Integer.parseInt(parts[1]);
                        if (possibleMeta >= 0 && possibleMeta <= 5) {
                            metadata = possibleMeta;
                            idx = 2;
                        }
                    } catch (NumberFormatException e) {
                        // Check for reaction name
                        metadata = getMetadataFromReaction(parts[1]);
                        if (metadata > 0) {
                            idx = 2;
                        }
                    }
                }
                
                long srcId = Long.parseLong(parts[idx]);
                long dstId = Long.parseLong(parts[idx + 1]);
                
                int edgeType = EdgeType.fromName(edgeTypeStr);
                aoeeClient.addEdge(srcId, edgeType, dstId, 0, metadata);
                count++;
                
                if (count % 10000 == 0) {
                    logger.info("Loaded {} edges...", count);
                }
            } catch (Exception e) {
                logger.debug("Skipping invalid line: {}", line);
            }
        }
        
        return count;
    }
    
    private int getMetadataFromReaction(String reaction) {
        return switch (reaction.toLowerCase()) {
            case "love" -> 1;
            case "haha" -> 2;
            case "wow" -> 3;
            case "sad" -> 4;
            case "angry" -> 5;
            default -> 0;
        };
    }
}

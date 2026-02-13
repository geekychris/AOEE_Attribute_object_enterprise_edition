package com.aoee.spring.controller;

import com.aoee.client.AoeeClient;
import com.aoee.client.EdgeType;
import com.aoee.spring.model.*;
import com.aoee.spring.persistence.PersistenceClient;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api/edges")
public class EdgeController {

    private static final Logger logger = LoggerFactory.getLogger(EdgeController.class);

    private final AoeeClient client;
    private final PersistenceClient persistenceClient;

    public EdgeController(AoeeClient client, PersistenceClient persistenceClient) {
        this.client = client;
        this.persistenceClient = persistenceClient;
    }

    @PostMapping
    public ResponseEntity<EdgeResponse> addEdge(@RequestBody EdgeRequest request) {
        // Add to AOEE cache
        long timestamp = client.addEdge(
                request.src(),
                EdgeType.fromName(request.edgeType()),
                request.dst(),
                request.getTimestamp(),
                request.getMetadata()
        );
        
        // Write-through to persistence
        boolean persisted = persistenceClient.persistEdge(
                request.src(),
                request.edgeType(),
                request.dst(),
                timestamp,
                request.getMetadata()
        );
        
        if (!persisted && persistenceClient.isEnabled()) {
            logger.warn("Edge added to cache but persistence failed: {} -[{}]-> {}",
                    request.src(), request.edgeType(), request.dst());
        }
        
        return ResponseEntity.ok(new EdgeResponse(true, "Edge added", timestamp));
    }

    @DeleteMapping
    public ResponseEntity<EdgeResponse> deleteEdge(@RequestBody EdgeRequest request) {
        // Delete from AOEE cache
        boolean success = client.deleteEdge(
                request.src(),
                EdgeType.fromName(request.edgeType()),
                request.dst()
        );
        
        // Write-through to persistence
        if (success) {
            boolean persisted = persistenceClient.deleteEdge(
                    request.src(),
                    request.edgeType(),
                    request.dst()
            );
            
            if (!persisted && persistenceClient.isEnabled()) {
                logger.warn("Edge deleted from cache but persistence failed: {} -[{}]-> {}",
                        request.src(), request.edgeType(), request.dst());
            }
        }
        
        return ResponseEntity.ok(new EdgeResponse(success, "Edge deleted"));
    }

    @GetMapping("/{src}/{edgeType}")
    public ResponseEntity<NeighborsResponse> getNeighbors(
            @PathVariable long src,
            @PathVariable String edgeType,
            @RequestParam(required = false, defaultValue = "0") int limit,
            @RequestParam(required = false, defaultValue = "false") boolean includeMetadata) {
        
        var result = client.getNeighborsWithMetadata(
            src, EdgeType.fromName(edgeType), limit, includeMetadata);
        
        return ResponseEntity.ok(new NeighborsResponse(
            src, edgeType, result.neighbors(),
            includeMetadata ? result.timestamps() : null,
            includeMetadata ? result.metadata() : null
        ));
    }

    @GetMapping("/{src}/{edgeType}/contains/{dst}")
    public ResponseEntity<ContainsResponse> contains(
            @PathVariable long src,
            @PathVariable String edgeType,
            @PathVariable long dst) {
        
        boolean exists = client.contains(src, EdgeType.fromName(edgeType), dst);
        return ResponseEntity.ok(new ContainsResponse(src, edgeType, dst, exists));
    }

    @GetMapping("/{src}/{edgeType}/count")
    public ResponseEntity<CountResponse> count(
            @PathVariable long src,
            @PathVariable String edgeType) {
        
        long count = client.count(src, EdgeType.fromName(edgeType));
        return ResponseEntity.ok(new CountResponse(src, edgeType, count));
    }
}

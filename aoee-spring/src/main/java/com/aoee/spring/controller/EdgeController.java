package com.aoee.spring.controller;

import com.aoee.client.AoeeClient;
import com.aoee.client.EdgeType;
import com.aoee.spring.model.*;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;

/**
 * Edge operations - proxies to AOEE Rust server.
 * 
 * Note: Persistence is handled by the AOEE server's write-through layer,
 * not by this UI application.
 */
@RestController
@RequestMapping("/api/edges")
public class EdgeController {

    private final AoeeClient client;

    public EdgeController(AoeeClient client) {
        this.client = client;
    }

    @PostMapping
    public ResponseEntity<EdgeResponse> addEdge(@RequestBody EdgeRequest request) {
        long timestamp = client.addEdge(
                request.src(),
                EdgeType.fromName(request.edgeType()),
                request.dst(),
                request.getTimestamp(),
                request.getMetadata()
        );
        return ResponseEntity.ok(new EdgeResponse(true, "Edge added", timestamp));
    }

    @DeleteMapping
    public ResponseEntity<EdgeResponse> deleteEdge(@RequestBody EdgeRequest request) {
        boolean success = client.deleteEdge(
                request.src(),
                EdgeType.fromName(request.edgeType()),
                request.dst()
        );
        return ResponseEntity.ok(new EdgeResponse(success, success ? "Edge deleted" : "Edge not found"));
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

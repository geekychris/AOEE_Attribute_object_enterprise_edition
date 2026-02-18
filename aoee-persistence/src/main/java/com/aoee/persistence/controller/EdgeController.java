package com.aoee.persistence.controller;

import com.aoee.persistence.entity.EdgeModel;
import com.aoee.persistence.service.EdgeService;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

@RestController
@RequestMapping("/api/v1/edges")
@CrossOrigin(origins = "*")
public class EdgeController {

    private final EdgeService edgeService;

    public EdgeController(EdgeService edgeService) {
        this.edgeService = edgeService;
    }

    // DTOs
    public record CreateEdgeRequest(Long src, String edgeType, Long dst, Long timestampNs, Integer metadata) {}
    public record EdgeResponse(Long id, Long srcId, String edgeType, Long dstId, Long timestampNs, Integer metadata, String createdAt) {
        public static EdgeResponse from(EdgeModel edge) {
            return new EdgeResponse(
                edge.getId(),
                edge.getSrcId(),
                edge.getEdgeType(),
                edge.getDstId(),
                edge.getTimestampNs(),
                edge.getMetadata() != null ? edge.getMetadata().intValue() : 0,
                edge.getCreatedAt() != null ? edge.getCreatedAt().toString() : null
            );
        }
    }

    @PostMapping
    public ResponseEntity<EdgeResponse> createEdge(@RequestBody CreateEdgeRequest request) {
        EdgeModel edge = edgeService.createEdge(
            request.src(), request.edgeType(), request.dst(),
            request.timestampNs(), request.metadata());
        return ResponseEntity.ok(EdgeResponse.from(edge));
    }

    /**
     * Batch create edges - much more efficient for bulk operations.
     */
    @PostMapping("/batch")
    public ResponseEntity<Map<String, Object>> createEdgesBatch(@RequestBody List<CreateEdgeRequest> requests) {
        List<EdgeService.EdgeBatchItem> items = requests.stream()
            .map(r -> new EdgeService.EdgeBatchItem(
                r.src(), r.edgeType(), r.dst(), r.timestampNs(), r.metadata()))
            .toList();
        
        int count = edgeService.createEdgesBatch(items);
        return ResponseEntity.ok(Map.of(
            "success", true,
            "edgesCreated", count
        ));
    }

    @GetMapping("/{src}/{edgeType}/{dst}")
    public ResponseEntity<EdgeResponse> getEdge(
            @PathVariable Long src,
            @PathVariable String edgeType,
            @PathVariable Long dst) {
        return edgeService.getEdge(src, edgeType.toUpperCase(), dst)
            .map(e -> ResponseEntity.ok(EdgeResponse.from(e)))
            .orElse(ResponseEntity.notFound().build());
    }

    @GetMapping("/{src}/{edgeType}/{dst}/exists")
    public ResponseEntity<Map<String, Boolean>> edgeExists(
            @PathVariable Long src,
            @PathVariable String edgeType,
            @PathVariable Long dst) {
        boolean exists = edgeService.edgeExists(src, edgeType.toUpperCase(), dst);
        return ResponseEntity.ok(Map.of("exists", exists));
    }

    @DeleteMapping("/{src}/{edgeType}/{dst}")
    public ResponseEntity<Map<String, Object>> deleteEdge(
            @PathVariable Long src,
            @PathVariable String edgeType,
            @PathVariable Long dst) {
        boolean deleted = edgeService.deleteEdge(src, edgeType.toUpperCase(), dst);
        return ResponseEntity.ok(Map.of(
            "success", deleted,
            "message", deleted ? "Edge deleted" : "Edge not found"
        ));
    }

    @GetMapping
    public ResponseEntity<List<EdgeResponse>> getEdges(
            @RequestParam Long src,
            @RequestParam String type,
            @RequestParam(defaultValue = "0") int limit) {
        List<EdgeModel> edges;
        if (limit > 0) {
            edges = edgeService.getEdgesBySrc(src, type.toUpperCase(), 0, limit).getContent();
        } else {
            edges = edgeService.getEdgesBySrc(src, type.toUpperCase());
        }
        return ResponseEntity.ok(edges.stream().map(EdgeResponse::from).toList());
    }

    @GetMapping("/neighbors")
    public ResponseEntity<Map<String, Object>> getNeighbors(
            @RequestParam Long src,
            @RequestParam String type,
            @RequestParam(defaultValue = "0") int limit) {
        List<Long> neighbors = edgeService.getNeighbors(src, type.toUpperCase(), limit);
        return ResponseEntity.ok(Map.of(
            "src", src,
            "edgeType", type.toUpperCase(),
            "neighbors", neighbors,
            "count", neighbors.size()
        ));
    }

    @GetMapping("/reverse-neighbors")
    public ResponseEntity<Map<String, Object>> getReverseNeighbors(
            @RequestParam Long dst,
            @RequestParam String type) {
        List<Long> neighbors = edgeService.getReverseNeighbors(dst, type.toUpperCase());
        return ResponseEntity.ok(Map.of(
            "dst", dst,
            "edgeType", type.toUpperCase(),
            "neighbors", neighbors,
            "count", neighbors.size()
        ));
    }

    @GetMapping("/mutual")
    public ResponseEntity<Map<String, Object>> getMutualConnections(
            @RequestParam Long id1,
            @RequestParam Long id2,
            @RequestParam(defaultValue = "FRIEND_OF") String type) {
        List<Long> mutual = edgeService.getMutualConnections(id1, id2, type.toUpperCase());
        return ResponseEntity.ok(Map.of(
            "id1", id1,
            "id2", id2,
            "edgeType", type.toUpperCase(),
            "mutual", mutual,
            "count", mutual.size()
        ));
    }

    @GetMapping("/count")
    public ResponseEntity<Map<String, Object>> getCount(
            @RequestParam(required = false) Long src,
            @RequestParam(required = false) String type) {
        long count;
        if (src != null && type != null) {
            count = edgeService.countEdges(src, type.toUpperCase());
        } else if (type != null) {
            count = edgeService.countByType(type.toUpperCase());
        } else {
            count = edgeService.countAll();
        }
        return ResponseEntity.ok(Map.of("count", count));
    }

    @GetMapping("/types")
    public ResponseEntity<List<String>> getEdgeTypes() {
        return ResponseEntity.ok(edgeService.getEdgeTypes());
    }

    // For testing - delete all edges
    @DeleteMapping
    public ResponseEntity<Map<String, Object>> deleteAllEdges() {
        long count = edgeService.countAll();
        edgeService.deleteAll();
        return ResponseEntity.ok(Map.of(
            "success", true,
            "deleted", count,
            "message", "All edges deleted"
        ));
    }
}

package com.aoee.persistence.controller;

import com.aoee.persistence.service.EdgeService;
import com.aoee.persistence.service.EntityService;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

@RestController
@RequestMapping("/api/v1/export")
@CrossOrigin(origins = "*")
public class ExportController {

    private final EdgeService edgeService;
    private final EntityService entityService;

    public ExportController(EdgeService edgeService, EntityService entityService) {
        this.edgeService = edgeService;
        this.entityService = entityService;
    }

    /**
     * Export all edges in AOEE dataset format.
     */
    @GetMapping(value = "/edges", produces = MediaType.TEXT_PLAIN_VALUE)
    public ResponseEntity<String> exportEdges() {
        String dataset = edgeService.exportToDatasetFormat();
        return ResponseEntity.ok()
            .header(HttpHeaders.CONTENT_DISPOSITION, "attachment; filename=aoee-dataset.txt")
            .body(dataset);
    }

    /**
     * Import edges from AOEE dataset format.
     */
    @PostMapping(value = "/edges", consumes = MediaType.TEXT_PLAIN_VALUE)
    public ResponseEntity<Map<String, Object>> importEdges(@RequestBody String datasetContent) {
        int count = edgeService.importFromDatasetFormat(datasetContent);
        return ResponseEntity.ok(Map.of(
            "success", true,
            "edgesImported", count
        ));
    }

    /**
     * Get statistics about the persisted data.
     */
    @GetMapping("/stats")
    public ResponseEntity<Map<String, Object>> getStats() {
        return ResponseEntity.ok(Map.of(
            "entities", Map.of(
                "total", entityService.countAll(),
                "types", entityService.getEntityTypes()
            ),
            "edges", Map.of(
                "total", edgeService.countAll(),
                "types", edgeService.getEdgeTypes()
            )
        ));
    }

    /**
     * Clear all data (for testing).
     */
    @DeleteMapping("/all")
    public ResponseEntity<Map<String, Object>> clearAll() {
        // This is dangerous - only for development/testing
        long edgeCount = edgeService.countAll();
        long entityCount = entityService.countAll();
        
        // Delete all edges first (due to potential foreign keys)
        edgeService.getAllEdges().forEach(e -> 
            edgeService.deleteEdge(e.getSrcId(), e.getEdgeType(), e.getDstId()));
        
        return ResponseEntity.ok(Map.of(
            "success", true,
            "edgesDeleted", edgeCount,
            "entitiesDeleted", entityCount
        ));
    }
}

package com.aoee.persistence.controller;

import com.aoee.persistence.entity.EntityModel;
import com.aoee.persistence.service.EntityService;
import org.springframework.data.domain.Page;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

@RestController
@RequestMapping("/api/v1/entities")
@CrossOrigin(origins = "*")
public class EntityController {

    private final EntityService entityService;

    public EntityController(EntityService entityService) {
        this.entityService = entityService;
    }

    // DTOs
    public record CreateEntityRequest(Long id, String entityType, String name) {}
    public record EntityResponse(Long id, String entityType, String name, String createdAt, String updatedAt) {
        public static EntityResponse from(EntityModel entity) {
            return new EntityResponse(
                entity.getId(),
                entity.getEntityType(),
                entity.getName(),
                entity.getCreatedAt() != null ? entity.getCreatedAt().toString() : null,
                entity.getUpdatedAt() != null ? entity.getUpdatedAt().toString() : null
            );
        }
    }

    @PostMapping
    public ResponseEntity<EntityResponse> createEntity(@RequestBody CreateEntityRequest request) {
        EntityModel entity = entityService.createOrUpdateEntity(
            request.id(), request.entityType(), request.name());
        return ResponseEntity.ok(EntityResponse.from(entity));
    }

    /**
     * Batch create entities - much more efficient for bulk operations.
     */
    @PostMapping("/batch")
    public ResponseEntity<Map<String, Object>> createEntitiesBatch(@RequestBody List<CreateEntityRequest> requests) {
        List<EntityService.EntityBatchItem> items = requests.stream()
            .map(r -> new EntityService.EntityBatchItem(r.id(), r.entityType(), r.name()))
            .toList();
        
        int count = entityService.createEntitiesBatch(items);
        return ResponseEntity.ok(Map.of(
            "success", true,
            "entitiesCreated", count
        ));
    }

    @GetMapping("/{id}")
    public ResponseEntity<EntityResponse> getEntity(@PathVariable Long id) {
        return entityService.getEntity(id)
            .map(e -> ResponseEntity.ok(EntityResponse.from(e)))
            .orElse(ResponseEntity.notFound().build());
    }

    @GetMapping
    public ResponseEntity<Map<String, Object>> getEntities(
            @RequestParam(required = false) String type,
            @RequestParam(defaultValue = "0") int page,
            @RequestParam(defaultValue = "100") int size) {
        
        Page<EntityModel> entities;
        if (type != null && !type.isEmpty()) {
            entities = entityService.getEntitiesByType(type, page, size);
        } else {
            entities = entityService.getAllEntities(page, size);
        }

        List<EntityResponse> content = entities.getContent().stream()
            .map(EntityResponse::from)
            .toList();

        return ResponseEntity.ok(Map.of(
            "content", content,
            "page", entities.getNumber(),
            "size", entities.getSize(),
            "totalElements", entities.getTotalElements(),
            "totalPages", entities.getTotalPages()
        ));
    }

    @GetMapping("/types")
    public ResponseEntity<List<String>> getEntityTypes() {
        return ResponseEntity.ok(entityService.getEntityTypes());
    }

    @DeleteMapping("/{id}")
    public ResponseEntity<Map<String, Object>> deleteEntity(@PathVariable Long id) {
        boolean deleted = entityService.deleteEntity(id);
        return ResponseEntity.ok(Map.of(
            "success", deleted,
            "message", deleted ? "Entity deleted" : "Entity not found"
        ));
    }

    @GetMapping("/count")
    public ResponseEntity<Map<String, Long>> getCount(@RequestParam(required = false) String type) {
        long count = type != null ? entityService.countByType(type) : entityService.countAll();
        return ResponseEntity.ok(Map.of("count", count));
    }
}

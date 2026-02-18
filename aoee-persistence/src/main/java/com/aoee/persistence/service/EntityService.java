package com.aoee.persistence.service;

import com.aoee.persistence.entity.EntityModel;
import com.aoee.persistence.repository.EdgeRepository;
import com.aoee.persistence.repository.EntityRepository;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.PageRequest;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

@Service
@Transactional
public class EntityService {

    private final EntityRepository entityRepository;
    private final EdgeRepository edgeRepository;

    public EntityService(EntityRepository entityRepository, EdgeRepository edgeRepository) {
        this.entityRepository = entityRepository;
        this.edgeRepository = edgeRepository;
    }

    public EntityModel createEntity(Long id, String entityType, String name) {
        EntityModel entity = new EntityModel(id, entityType, name);
        return entityRepository.save(entity);
    }

    public EntityModel createOrUpdateEntity(Long id, String entityType, String name) {
        Optional<EntityModel> existing = entityRepository.findById(id);
        if (existing.isPresent()) {
            EntityModel entity = existing.get();
            entity.setEntityType(entityType);
            if (name != null) {
                entity.setName(name);
            }
            return entityRepository.save(entity);
        }
        return createEntity(id, entityType, name);
    }

    /**
     * Batch create entities efficiently.
     * @return number of entities created
     */
    public int createEntitiesBatch(List<EntityBatchItem> entities) {
        List<EntityModel> toSave = new ArrayList<>();
        for (EntityBatchItem item : entities) {
            toSave.add(new EntityModel(item.id(), item.entityType(), item.name()));
        }
        entityRepository.saveAll(toSave);
        return toSave.size();
    }

    /**
     * DTO for batch entity creation.
     */
    public record EntityBatchItem(Long id, String entityType, String name) {}

    @Transactional(readOnly = true)
    public Optional<EntityModel> getEntity(Long id) {
        return entityRepository.findById(id);
    }

    @Transactional(readOnly = true)
    public List<EntityModel> getEntitiesByType(String entityType) {
        return entityRepository.findByEntityType(entityType);
    }

    @Transactional(readOnly = true)
    public Page<EntityModel> getEntitiesByType(String entityType, int page, int size) {
        return entityRepository.findByEntityType(entityType, PageRequest.of(page, size));
    }

    @Transactional(readOnly = true)
    public Page<EntityModel> getAllEntities(int page, int size) {
        return entityRepository.findAll(PageRequest.of(page, size));
    }

    @Transactional(readOnly = true)
    public List<EntityModel> getEntitiesByIds(List<Long> ids) {
        return entityRepository.findByIdIn(ids);
    }

    @Transactional(readOnly = true)
    public List<String> getEntityTypes() {
        return entityRepository.findDistinctEntityTypes();
    }

    public boolean deleteEntity(Long id) {
        if (entityRepository.existsById(id)) {
            // First delete all edges involving this entity
            edgeRepository.deleteByEntityId(id);
            entityRepository.deleteById(id);
            return true;
        }
        return false;
    }

    @Transactional(readOnly = true)
    public long countByType(String entityType) {
        return entityRepository.countByEntityType(entityType);
    }

    @Transactional(readOnly = true)
    public long countAll() {
        return entityRepository.count();
    }
}

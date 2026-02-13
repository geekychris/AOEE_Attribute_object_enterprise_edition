package com.aoee.persistence.repository;

import com.aoee.persistence.entity.EntityModel;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Query;
import org.springframework.stereotype.Repository;

import java.util.List;

@Repository
public interface EntityRepository extends JpaRepository<EntityModel, Long> {

    List<EntityModel> findByEntityType(String entityType);

    Page<EntityModel> findByEntityType(String entityType, Pageable pageable);

    @Query("SELECT DISTINCT e.entityType FROM EntityModel e")
    List<String> findDistinctEntityTypes();

    long countByEntityType(String entityType);

    List<EntityModel> findByIdIn(List<Long> ids);
}

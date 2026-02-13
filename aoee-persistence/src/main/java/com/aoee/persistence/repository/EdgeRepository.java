package com.aoee.persistence.repository;

import com.aoee.persistence.entity.EdgeModel;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Modifying;
import org.springframework.data.jpa.repository.Query;
import org.springframework.data.repository.query.Param;
import org.springframework.stereotype.Repository;

import java.util.List;
import java.util.Optional;

@Repository
public interface EdgeRepository extends JpaRepository<EdgeModel, Long> {

    // Find edges by source and type
    List<EdgeModel> findBySrcIdAndEdgeType(Long srcId, String edgeType);

    Page<EdgeModel> findBySrcIdAndEdgeType(Long srcId, String edgeType, Pageable pageable);

    // Find edges by destination and type (reverse lookup)
    List<EdgeModel> findByDstIdAndEdgeType(Long dstId, String edgeType);

    // Check if edge exists
    boolean existsBySrcIdAndEdgeTypeAndDstId(Long srcId, String edgeType, Long dstId);

    // Find specific edge
    Optional<EdgeModel> findBySrcIdAndEdgeTypeAndDstId(Long srcId, String edgeType, Long dstId);

    // Delete specific edge
    @Modifying
    void deleteBySrcIdAndEdgeTypeAndDstId(Long srcId, String edgeType, Long dstId);

    // Count edges by source and type
    long countBySrcIdAndEdgeType(Long srcId, String edgeType);

    // Get neighbors (destination IDs)
    @Query("SELECT e.dstId FROM EdgeModel e WHERE e.srcId = :srcId AND e.edgeType = :edgeType ORDER BY e.dstId")
    List<Long> findNeighbors(@Param("srcId") Long srcId, @Param("edgeType") String edgeType);

    @Query("SELECT e.dstId FROM EdgeModel e WHERE e.srcId = :srcId AND e.edgeType = :edgeType ORDER BY e.dstId")
    Page<Long> findNeighbors(@Param("srcId") Long srcId, @Param("edgeType") String edgeType, Pageable pageable);

    // Get reverse neighbors (source IDs)
    @Query("SELECT e.srcId FROM EdgeModel e WHERE e.dstId = :dstId AND e.edgeType = :edgeType ORDER BY e.srcId")
    List<Long> findReverseNeighbors(@Param("dstId") Long dstId, @Param("edgeType") String edgeType);

    // Find mutual connections (intersection)
    @Query("SELECT e1.dstId FROM EdgeModel e1, EdgeModel e2 " +
           "WHERE e1.srcId = :id1 AND e1.edgeType = :edgeType " +
           "AND e2.srcId = :id2 AND e2.edgeType = :edgeType " +
           "AND e1.dstId = e2.dstId " +
           "ORDER BY e1.dstId")
    List<Long> findMutualConnections(@Param("id1") Long id1, @Param("id2") Long id2, @Param("edgeType") String edgeType);

    // Find all edges by type (for export)
    List<EdgeModel> findByEdgeType(String edgeType);

    // Count by edge type
    long countByEdgeType(String edgeType);

    @Query("SELECT DISTINCT e.edgeType FROM EdgeModel e")
    List<String> findDistinctEdgeTypes();

    // Delete all edges involving an entity
    @Modifying
    @Query("DELETE FROM EdgeModel e WHERE e.srcId = :entityId OR e.dstId = :entityId")
    void deleteByEntityId(@Param("entityId") Long entityId);
}

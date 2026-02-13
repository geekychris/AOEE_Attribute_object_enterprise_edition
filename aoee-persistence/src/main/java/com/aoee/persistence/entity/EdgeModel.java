package com.aoee.persistence.entity;

import jakarta.persistence.*;
import java.time.Instant;

/**
 * Represents a directed edge in the graph (follows, likes, friend_of, member_of)
 */
@Entity
@Table(name = "edges",
       uniqueConstraints = @UniqueConstraint(columnNames = {"src_id", "edge_type", "dst_id"}),
       indexes = {
           @Index(name = "idx_edges_src_type", columnList = "src_id, edge_type"),
           @Index(name = "idx_edges_dst_type", columnList = "dst_id, edge_type")
       })
public class EdgeModel {

    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(name = "src_id", nullable = false)
    private Long srcId;

    @Column(name = "edge_type", nullable = false, length = 50)
    private String edgeType;

    @Column(name = "dst_id", nullable = false)
    private Long dstId;

    @Column(name = "timestamp_ns", nullable = false)
    private Long timestampNs;

    @Column(nullable = false)
    private Short metadata = 0;

    @Column(name = "created_at", nullable = false, updatable = false)
    private Instant createdAt;

    public EdgeModel() {
    }

    public EdgeModel(Long srcId, String edgeType, Long dstId, Long timestampNs, Short metadata) {
        this.srcId = srcId;
        this.edgeType = edgeType;
        this.dstId = dstId;
        this.timestampNs = timestampNs != null && timestampNs > 0 ? timestampNs : System.nanoTime();
        this.metadata = metadata != null ? metadata : 0;
        this.createdAt = Instant.now();
    }

    @PrePersist
    protected void onCreate() {
        if (createdAt == null) {
            createdAt = Instant.now();
        }
        if (timestampNs == null || timestampNs == 0) {
            timestampNs = System.nanoTime();
        }
        if (metadata == null) {
            metadata = 0;
        }
    }

    // Getters and Setters
    public Long getId() {
        return id;
    }

    public void setId(Long id) {
        this.id = id;
    }

    public Long getSrcId() {
        return srcId;
    }

    public void setSrcId(Long srcId) {
        this.srcId = srcId;
    }

    public String getEdgeType() {
        return edgeType;
    }

    public void setEdgeType(String edgeType) {
        this.edgeType = edgeType;
    }

    public Long getDstId() {
        return dstId;
    }

    public void setDstId(Long dstId) {
        this.dstId = dstId;
    }

    public Long getTimestampNs() {
        return timestampNs;
    }

    public void setTimestampNs(Long timestampNs) {
        this.timestampNs = timestampNs;
    }

    public Short getMetadata() {
        return metadata;
    }

    public void setMetadata(Short metadata) {
        this.metadata = metadata;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }

    public void setCreatedAt(Instant createdAt) {
        this.createdAt = createdAt;
    }

    @Override
    public String toString() {
        return "EdgeModel{" +
                "srcId=" + srcId +
                ", edgeType='" + edgeType + '\'' +
                ", dstId=" + dstId +
                ", metadata=" + metadata +
                '}';
    }
}

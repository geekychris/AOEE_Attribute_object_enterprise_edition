-- AOEE Persistence Schema
-- V1: Initial schema for entities and edges

-- Entities table: stores nodes/vertices in the graph
CREATE TABLE entities (
    id BIGINT PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Index for querying by entity type
CREATE INDEX idx_entities_type ON entities(entity_type);

-- Edges table: stores directed relationships between entities
CREATE TABLE edges (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    src_id BIGINT NOT NULL,
    edge_type VARCHAR(50) NOT NULL,
    dst_id BIGINT NOT NULL,
    timestamp_ns BIGINT NOT NULL,
    metadata SMALLINT DEFAULT 0 NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT uk_edges_src_type_dst UNIQUE (src_id, edge_type, dst_id)
);

-- Index for forward traversal: get all edges from a source with a given type
CREATE INDEX idx_edges_src_type ON edges(src_id, edge_type);

-- Index for reverse traversal: find all edges pointing to a destination
CREATE INDEX idx_edges_dst_type ON edges(dst_id, edge_type);

-- Index for edge type queries (e.g., count all LIKES)
CREATE INDEX idx_edges_type ON edges(edge_type);

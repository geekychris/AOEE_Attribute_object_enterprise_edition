package com.aoee.persistence.service;

import com.aoee.persistence.entity.EdgeModel;
import com.aoee.persistence.repository.EdgeRepository;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.PageRequest;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

@Service
@Transactional
public class EdgeService {

    private final EdgeRepository edgeRepository;

    public EdgeService(EdgeRepository edgeRepository) {
        this.edgeRepository = edgeRepository;
    }

    public EdgeModel createEdge(Long srcId, String edgeType, Long dstId, Long timestampNs, Integer metadata) {
        // Check if edge already exists
        Optional<EdgeModel> existing = edgeRepository.findBySrcIdAndEdgeTypeAndDstId(srcId, edgeType, dstId);
        if (existing.isPresent()) {
            // Update existing edge
            EdgeModel edge = existing.get();
            if (timestampNs != null && timestampNs > 0) {
                edge.setTimestampNs(timestampNs);
            }
            if (metadata != null) {
                edge.setMetadata(metadata.shortValue());
            }
            return edgeRepository.save(edge);
        }

        // Create new edge
        Short meta = metadata != null ? metadata.shortValue() : 0;
        EdgeModel edge = new EdgeModel(srcId, edgeType, dstId, timestampNs, meta);
        return edgeRepository.save(edge);
    }

    /**
     * Batch create/update edges efficiently.
     * @return number of edges created/updated
     */
    public int createEdgesBatch(List<EdgeBatchItem> edges) {
        List<EdgeModel> toSave = new ArrayList<>();
        
        for (EdgeBatchItem item : edges) {
            Short meta = item.metadata() != null ? item.metadata().shortValue() : 0;
            EdgeModel edge = new EdgeModel(
                item.srcId(), 
                item.edgeType().toUpperCase(), 
                item.dstId(), 
                item.timestampNs(), 
                meta
            );
            toSave.add(edge);
        }
        
        // Use saveAll for batch insert (more efficient than individual saves)
        edgeRepository.saveAll(toSave);
        return toSave.size();
    }

    /**
     * DTO for batch edge creation.
     */
    public record EdgeBatchItem(Long srcId, String edgeType, Long dstId, Long timestampNs, Integer metadata) {}

    @Transactional(readOnly = true)
    public Optional<EdgeModel> getEdge(Long srcId, String edgeType, Long dstId) {
        return edgeRepository.findBySrcIdAndEdgeTypeAndDstId(srcId, edgeType, dstId);
    }

    @Transactional(readOnly = true)
    public boolean edgeExists(Long srcId, String edgeType, Long dstId) {
        return edgeRepository.existsBySrcIdAndEdgeTypeAndDstId(srcId, edgeType, dstId);
    }

    public boolean deleteEdge(Long srcId, String edgeType, Long dstId) {
        if (edgeRepository.existsBySrcIdAndEdgeTypeAndDstId(srcId, edgeType, dstId)) {
            edgeRepository.deleteBySrcIdAndEdgeTypeAndDstId(srcId, edgeType, dstId);
            return true;
        }
        return false;
    }

    @Transactional(readOnly = true)
    public List<EdgeModel> getEdgesBySrc(Long srcId, String edgeType) {
        return edgeRepository.findBySrcIdAndEdgeType(srcId, edgeType);
    }

    @Transactional(readOnly = true)
    public Page<EdgeModel> getEdgesBySrc(Long srcId, String edgeType, int page, int size) {
        return edgeRepository.findBySrcIdAndEdgeType(srcId, edgeType, PageRequest.of(page, size));
    }

    @Transactional(readOnly = true)
    public List<EdgeModel> getEdgesByDst(Long dstId, String edgeType) {
        return edgeRepository.findByDstIdAndEdgeType(dstId, edgeType);
    }

    @Transactional(readOnly = true)
    public List<Long> getNeighbors(Long srcId, String edgeType) {
        return edgeRepository.findNeighbors(srcId, edgeType);
    }

    @Transactional(readOnly = true)
    public List<Long> getNeighbors(Long srcId, String edgeType, int limit) {
        if (limit <= 0) {
            return edgeRepository.findNeighbors(srcId, edgeType);
        }
        return edgeRepository.findNeighbors(srcId, edgeType, PageRequest.of(0, limit)).getContent();
    }

    @Transactional(readOnly = true)
    public List<Long> getReverseNeighbors(Long dstId, String edgeType) {
        return edgeRepository.findReverseNeighbors(dstId, edgeType);
    }

    @Transactional(readOnly = true)
    public long countEdges(Long srcId, String edgeType) {
        return edgeRepository.countBySrcIdAndEdgeType(srcId, edgeType);
    }

    @Transactional(readOnly = true)
    public List<Long> getMutualConnections(Long id1, Long id2, String edgeType) {
        return edgeRepository.findMutualConnections(id1, id2, edgeType);
    }

    @Transactional(readOnly = true)
    public List<EdgeModel> getAllEdgesByType(String edgeType) {
        return edgeRepository.findByEdgeType(edgeType);
    }

    @Transactional(readOnly = true)
    public List<EdgeModel> getAllEdges() {
        return edgeRepository.findAll();
    }

    @Transactional(readOnly = true)
    public List<String> getEdgeTypes() {
        return edgeRepository.findDistinctEdgeTypes();
    }

    @Transactional(readOnly = true)
    public long countByType(String edgeType) {
        return edgeRepository.countByEdgeType(edgeType);
    }

    @Transactional(readOnly = true)
    public long countAll() {
        return edgeRepository.count();
    }

    /**
     * Delete all edges (for testing).
     */
    public void deleteAll() {
        edgeRepository.deleteAll();
    }

    /**
     * Export all edges in AOEE dataset format.
     * Format: EDGE_TYPE METADATA SRC_ID DST_ID [reaction_name]
     */
    @Transactional(readOnly = true)
    public String exportToDatasetFormat() {
        StringBuilder sb = new StringBuilder();
        sb.append("# AOEE Dataset Export\n");
        sb.append("# Format: EDGE_TYPE SRC_ID DST_ID [metadata/reaction]\n\n");

        List<EdgeModel> edges = edgeRepository.findAll();
        for (EdgeModel edge : edges) {
            sb.append(edge.getEdgeType());
            
            // For LIKES, include reaction name
            if ("LIKES".equals(edge.getEdgeType()) && edge.getMetadata() != null && edge.getMetadata() > 0) {
                sb.append(" ").append(edge.getMetadata());
            }
            
            sb.append(" ").append(edge.getSrcId());
            sb.append(" ").append(edge.getDstId());
            
            // Add reaction name as comment for readability
            if ("LIKES".equals(edge.getEdgeType()) && edge.getMetadata() != null && edge.getMetadata() > 0) {
                sb.append(" ").append(getReactionName(edge.getMetadata()));
            }
            
            sb.append("\n");
        }

        return sb.toString();
    }

    private String getReactionName(short metadata) {
        return switch (metadata) {
            case 0 -> "like";
            case 1 -> "love";
            case 2 -> "haha";
            case 3 -> "wow";
            case 4 -> "sad";
            case 5 -> "angry";
            default -> "like";
        };
    }

    /**
     * Import edges from AOEE dataset format.
     * Returns count of imported edges.
     */
    public int importFromDatasetFormat(String datasetContent) {
        int count = 0;
        String[] lines = datasetContent.split("\n");
        
        for (String line : lines) {
            line = line.trim();
            if (line.isEmpty() || line.startsWith("#")) {
                continue;
            }
            
            String[] parts = line.split("\\s+");
            if (parts.length < 3) {
                continue;
            }
            
            String edgeType = parts[0].toUpperCase();
            int idx = 1;
            Integer metadata = 0;
            
            // Check if second part is metadata (number) or src_id
            try {
                // If it's a small number (0-5), it's likely metadata for LIKES
                int possibleMeta = Integer.parseInt(parts[1]);
                if ("LIKES".equals(edgeType) && possibleMeta >= 0 && possibleMeta <= 5 && parts.length >= 4) {
                    metadata = possibleMeta;
                    idx = 2;
                }
            } catch (NumberFormatException e) {
                // Not a number, check for reaction name
                if ("LIKES".equals(edgeType)) {
                    metadata = getMetadataFromReaction(parts[1]);
                    if (metadata > 0) {
                        idx = 2;
                    }
                }
            }
            
            try {
                Long srcId = Long.parseLong(parts[idx]);
                Long dstId = Long.parseLong(parts[idx + 1]);
                
                createEdge(srcId, edgeType, dstId, null, metadata);
                count++;
            } catch (Exception e) {
                // Skip invalid lines
            }
        }
        
        return count;
    }

    private int getMetadataFromReaction(String reaction) {
        return switch (reaction.toLowerCase()) {
            case "love" -> 1;
            case "haha" -> 2;
            case "wow" -> 3;
            case "sad" -> 4;
            case "angry" -> 5;
            default -> 0;
        };
    }
}

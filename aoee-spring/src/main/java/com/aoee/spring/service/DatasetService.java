package com.aoee.spring.service;

import com.aoee.client.AoeeClient;
import com.aoee.client.EdgeType;
import com.aoee.client.ReactionType;
import com.aoee.spring.model.DatasetLoadResponse;
import com.aoee.spring.model.DatasetParseResult;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

@Service
public class DatasetService {
    private static final Logger logger = LoggerFactory.getLogger(DatasetService.class);

    private final AoeeClient client;

    // Pattern for edge lines: EDGE_TYPE SRC DST [METADATA] [# comment]
    // Metadata is optional, used for reactions on LIKES edges
    private static final Pattern EDGE_PATTERN = Pattern.compile(
            "^(\\w+)\\s+(\\d+)\\s+(\\d+)(?:\\s+(\\w+))?(?:\\s*#.*)?$"
    );

    // Edge types that support metadata
    private static final Set<String> METADATA_EDGE_TYPES = Set.of("LIKES");

    // Pattern for entity lines: ENTITY TYPE ID NAME
    private static final Pattern ENTITY_PATTERN = Pattern.compile(
            "^ENTITY\\s+(\\w+)\\s+(\\d+)\\s+(.+?)(?:\\s*#.*)?$"
    );

    public DatasetService(AoeeClient client) {
        this.client = client;
    }

    /**
     * Parse a dataset and return statistics without loading.
     */
    public DatasetParseResult parseDataset(String content) {
        List<String> errors = new ArrayList<>();
        Map<String, Integer> entitiesByType = new HashMap<>();
        Map<String, Integer> edgesByType = new HashMap<>();
        int entityCount = 0;
        int edgeCount = 0;

        String[] lines = content.split("\n");
        int lineNum = 0;

        for (String line : lines) {
            lineNum++;
            line = line.trim();

            // Skip empty lines and comments
            if (line.isEmpty() || line.startsWith("#")) {
                continue;
            }

            // Try entity pattern
            if (line.startsWith("ENTITY")) {
                Matcher m = ENTITY_PATTERN.matcher(line);
                if (m.matches()) {
                    String type = m.group(1);
                    entitiesByType.merge(type, 1, Integer::sum);
                    entityCount++;
                } else {
                    errors.add("Line " + lineNum + ": Invalid entity format: " + line);
                }
                continue;
            }

            // Try edge pattern
            Matcher m = EDGE_PATTERN.matcher(line);
            if (m.matches()) {
                String edgeType = m.group(1);
                String metadataStr = m.group(4); // Optional metadata
                try {
                    EdgeType.fromName(edgeType);
                    // Validate metadata if provided
                    if (metadataStr != null && METADATA_EDGE_TYPES.contains(edgeType)) {
                        ReactionType.fromName(metadataStr); // Validate reaction name
                    }
                    edgesByType.merge(edgeType, 1, Integer::sum);
                    edgeCount++;
                } catch (IllegalArgumentException e) {
                    errors.add("Line " + lineNum + ": " + e.getMessage());
                }
            } else {
                errors.add("Line " + lineNum + ": Invalid format: " + line);
            }
        }

        return new DatasetParseResult(
                errors.isEmpty(),
                entityCount,
                edgeCount,
                entitiesByType,
                edgesByType,
                errors
        );
    }

    /**
     * Load a dataset into AOEE.
     */
    public DatasetLoadResponse loadDataset(String content) {
        long startTime = System.currentTimeMillis();
        List<String> errors = new ArrayList<>();
        int entitiesLoaded = 0;
        int edgesLoaded = 0;

        String[] lines = content.split("\n");
        int lineNum = 0;

        for (String line : lines) {
            lineNum++;
            line = line.trim();

            // Skip empty lines and comments
            if (line.isEmpty() || line.startsWith("#")) {
                continue;
            }

            // Skip entity lines (documentation only)
            if (line.startsWith("ENTITY")) {
                Matcher m = ENTITY_PATTERN.matcher(line);
                if (m.matches()) {
                    entitiesLoaded++;
                }
                continue;
            }

            // Try edge pattern
            Matcher m = EDGE_PATTERN.matcher(line);
            if (m.matches()) {
                String edgeType = m.group(1);
                long src = Long.parseLong(m.group(2));
                long dst = Long.parseLong(m.group(3));
                String metadataStr = m.group(4); // Optional metadata

                try {
                    int edgeTypeCode = EdgeType.fromName(edgeType);
                    
                    // Parse metadata for supported edge types
                    if (metadataStr != null && METADATA_EDGE_TYPES.contains(edgeType)) {
                        int metadata = ReactionType.fromName(metadataStr);
                        client.addEdge(src, edgeTypeCode, dst, 0, metadata); // 0 = auto-generate timestamp
                    } else {
                        client.addEdge(src, edgeTypeCode, dst);
                    }
                    edgesLoaded++;

                    // Log progress every 1000 edges
                    if (edgesLoaded % 1000 == 0) {
                        logger.info("Loaded {} edges...", edgesLoaded);
                    }
                } catch (IllegalArgumentException e) {
                    errors.add("Line " + lineNum + ": " + e.getMessage());
                } catch (Exception e) {
                    errors.add("Line " + lineNum + ": Failed to add edge: " + e.getMessage());
                }
            } else {
                errors.add("Line " + lineNum + ": Invalid format: " + line);
            }
        }

        long elapsed = System.currentTimeMillis() - startTime;
        logger.info("Dataset loaded: {} entities, {} edges in {}ms", entitiesLoaded, edgesLoaded, elapsed);

        return new DatasetLoadResponse(
                errors.isEmpty(),
                entitiesLoaded,
                edgesLoaded,
                errors.size(),
                errors.size() > 10 ? errors.subList(0, 10) : errors,
                elapsed
        );
    }
}

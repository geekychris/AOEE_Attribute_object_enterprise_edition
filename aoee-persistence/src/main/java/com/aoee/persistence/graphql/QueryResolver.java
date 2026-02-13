package com.aoee.persistence.graphql;

import com.aoee.persistence.entity.EdgeModel;
import com.aoee.persistence.entity.EntityModel;
import com.aoee.persistence.service.EdgeService;
import com.aoee.persistence.service.EntityService;
import org.springframework.graphql.data.method.annotation.Argument;
import org.springframework.graphql.data.method.annotation.QueryMapping;
import org.springframework.graphql.data.method.annotation.SchemaMapping;
import org.springframework.stereotype.Controller;

import java.util.List;
import java.util.Map;

@Controller
public class QueryResolver {

    private final EntityService entityService;
    private final EdgeService edgeService;

    public QueryResolver(EntityService entityService, EdgeService edgeService) {
        this.entityService = entityService;
        this.edgeService = edgeService;
    }

    // Entity queries
    @QueryMapping
    public EntityModel entity(@Argument Long id) {
        return entityService.getEntity(id).orElse(null);
    }

    @QueryMapping
    public List<EntityModel> entities(@Argument String type, @Argument Integer page, @Argument Integer size) {
        int p = page != null ? page : 0;
        int s = size != null ? size : 100;
        if (type != null && !type.isEmpty()) {
            return entityService.getEntitiesByType(type, p, s).getContent();
        }
        return entityService.getAllEntities(p, s).getContent();
    }

    @QueryMapping
    public List<String> entityTypes() {
        return entityService.getEntityTypes();
    }

    @QueryMapping
    public long entityCount(@Argument String type) {
        return type != null ? entityService.countByType(type) : entityService.countAll();
    }

    // Edge queries
    @QueryMapping
    public EdgeModel edge(@Argument Long src, @Argument String edgeType, @Argument Long dst) {
        return edgeService.getEdge(src, edgeType.toUpperCase(), dst).orElse(null);
    }

    @QueryMapping
    public List<EdgeModel> edges(@Argument Long src, @Argument String edgeType, @Argument Integer limit) {
        int lim = limit != null ? limit : 0;
        if (lim > 0) {
            return edgeService.getEdgesBySrc(src, edgeType.toUpperCase(), 0, lim).getContent();
        }
        return edgeService.getEdgesBySrc(src, edgeType.toUpperCase());
    }

    @QueryMapping
    public boolean edgeExists(@Argument Long src, @Argument String edgeType, @Argument Long dst) {
        return edgeService.edgeExists(src, edgeType.toUpperCase(), dst);
    }

    @QueryMapping
    public List<String> edgeTypes() {
        return edgeService.getEdgeTypes();
    }

    @QueryMapping
    public long edgeCount(@Argument Long src, @Argument String type) {
        if (src != null && type != null) {
            return edgeService.countEdges(src, type.toUpperCase());
        } else if (type != null) {
            return edgeService.countByType(type.toUpperCase());
        }
        return edgeService.countAll();
    }

    // Graph queries
    @QueryMapping
    public Map<String, Object> neighbors(@Argument Long src, @Argument String edgeType, @Argument Integer limit) {
        List<Long> neighbors = edgeService.getNeighbors(src, edgeType.toUpperCase(), limit != null ? limit : 0);
        return Map.of(
            "src", src,
            "edgeType", edgeType.toUpperCase(),
            "neighbors", neighbors,
            "count", neighbors.size()
        );
    }

    @QueryMapping
    public Map<String, Object> reverseNeighbors(@Argument Long dst, @Argument String edgeType) {
        List<Long> neighbors = edgeService.getReverseNeighbors(dst, edgeType.toUpperCase());
        return Map.of(
            "src", dst,
            "edgeType", edgeType.toUpperCase(),
            "neighbors", neighbors,
            "count", neighbors.size()
        );
    }

    @QueryMapping
    public Map<String, Object> mutualConnections(@Argument Long id1, @Argument Long id2, @Argument String edgeType) {
        String type = edgeType != null ? edgeType.toUpperCase() : "FRIEND_OF";
        List<Long> mutual = edgeService.getMutualConnections(id1, id2, type);
        return Map.of(
            "id1", id1,
            "id2", id2,
            "edgeType", type,
            "mutual", mutual,
            "count", mutual.size()
        );
    }

    @QueryMapping
    public Map<String, Object> stats() {
        return Map.of(
            "totalEntities", entityService.countAll(),
            "totalEdges", edgeService.countAll(),
            "entityTypes", entityService.getEntityTypes(),
            "edgeTypes", edgeService.getEdgeTypes()
        );
    }

    // Entity field resolvers
    @SchemaMapping(typeName = "Entity", field = "outgoingEdges")
    public List<EdgeModel> outgoingEdges(EntityModel entity, @Argument String edgeType, @Argument Integer limit) {
        if (edgeType != null) {
            if (limit != null && limit > 0) {
                return edgeService.getEdgesBySrc(entity.getId(), edgeType.toUpperCase(), 0, limit).getContent();
            }
            return edgeService.getEdgesBySrc(entity.getId(), edgeType.toUpperCase());
        }
        // Return all edges if no type specified (limited for safety)
        return edgeService.getEdgesBySrc(entity.getId(), "FOLLOWS", 0, limit != null ? limit : 100).getContent();
    }

    @SchemaMapping(typeName = "Entity", field = "incomingEdges")
    public List<EdgeModel> incomingEdges(EntityModel entity, @Argument String edgeType, @Argument Integer limit) {
        if (edgeType != null) {
            List<EdgeModel> edges = edgeService.getEdgesByDst(entity.getId(), edgeType.toUpperCase());
            if (limit != null && limit > 0) {
                return edges.stream().limit(limit).toList();
            }
            return edges;
        }
        return List.of();
    }

    @SchemaMapping(typeName = "Entity", field = "neighborCount")
    public long neighborCount(EntityModel entity, @Argument String edgeType) {
        return edgeService.countEdges(entity.getId(), edgeType.toUpperCase());
    }

    // Edge field resolvers
    @SchemaMapping(typeName = "Edge", field = "src")
    public EntityModel edgeSrc(EdgeModel edge) {
        return entityService.getEntity(edge.getSrcId()).orElse(null);
    }

    @SchemaMapping(typeName = "Edge", field = "dst")
    public EntityModel edgeDst(EdgeModel edge) {
        return entityService.getEntity(edge.getDstId()).orElse(null);
    }
}

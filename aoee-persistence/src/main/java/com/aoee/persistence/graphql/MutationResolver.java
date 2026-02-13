package com.aoee.persistence.graphql;

import com.aoee.persistence.entity.EdgeModel;
import com.aoee.persistence.entity.EntityModel;
import com.aoee.persistence.service.EdgeService;
import com.aoee.persistence.service.EntityService;
import org.springframework.graphql.data.method.annotation.Argument;
import org.springframework.graphql.data.method.annotation.MutationMapping;
import org.springframework.stereotype.Controller;

@Controller
public class MutationResolver {

    private final EntityService entityService;
    private final EdgeService edgeService;

    public MutationResolver(EntityService entityService, EdgeService edgeService) {
        this.entityService = entityService;
        this.edgeService = edgeService;
    }

    @MutationMapping
    public EntityModel createEntity(@Argument Long id, @Argument String entityType, @Argument String name) {
        return entityService.createOrUpdateEntity(id, entityType, name);
    }

    @MutationMapping
    public boolean deleteEntity(@Argument Long id) {
        return entityService.deleteEntity(id);
    }

    @MutationMapping
    public EdgeModel createEdge(@Argument Long src, @Argument String edgeType, 
                                @Argument Long dst, @Argument Integer metadata) {
        return edgeService.createEdge(src, edgeType.toUpperCase(), dst, null, metadata);
    }

    @MutationMapping
    public boolean deleteEdge(@Argument Long src, @Argument String edgeType, @Argument Long dst) {
        return edgeService.deleteEdge(src, edgeType.toUpperCase(), dst);
    }

    @MutationMapping
    public int importDataset(@Argument String content) {
        return edgeService.importFromDatasetFormat(content);
    }
}

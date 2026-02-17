//! gRPC service implementation

use crate::proto::aoee_server::Aoee;
use crate::proto::*;
use aoee_core::{EdgeKey, EdgeType, EntityId, EntityType, IdGenerator, FofConfig, FofQuery};
use aoee_shard::{ShardManager, manager::ManagerConfig};
use aoee_storage::{EdgeStore, InMemoryStore};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tonic::{Request, Response, Status};

/// Maximum IDs that can be generated in a single request
const MAX_GENERATE_IDS: u32 = 10_000;

/// AOEE gRPC service
pub struct AoeeService<S: EdgeStore + 'static> {
    manager: Arc<ShardManager<S>>,
    /// ID generators per entity type (lazily created)
    id_generators: Arc<RwLock<HashMap<u16, IdGenerator>>>,
}

impl AoeeService<InMemoryStore> {
    /// Create a new service with in-memory storage
    pub async fn new_in_memory(config: ManagerConfig) -> Self {
        let manager = ShardManager::new(config, |_| Arc::new(InMemoryStore::new()));
        manager.initialize().await;
        AoeeService {
            manager: Arc::new(manager),
            id_generators: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<S: EdgeStore + 'static> AoeeService<S> {
    /// Create a new service with custom storage
    pub async fn new<F>(config: ManagerConfig, storage_factory: F) -> Self
    where
        F: Fn(u32) -> Arc<S> + Send + Sync + 'static,
    {
        let manager = ShardManager::new(config, storage_factory);
        manager.initialize().await;
        AoeeService {
            manager: Arc::new(manager),
            id_generators: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create an ID generator for the given entity type
    fn get_or_create_generator(&self, entity_type: EntityType) -> IdGenerator {
        let type_code = entity_type.as_raw();
        
        // Try read lock first
        {
            let generators = self.id_generators.read();
            if let Some(gen) = generators.get(&type_code) {
                return gen.clone();
            }
        }
        
        // Need to create - upgrade to write lock
        let mut generators = self.id_generators.write();
        // Double-check after acquiring write lock
        if let Some(gen) = generators.get(&type_code) {
            return gen.clone();
        }
        
        let gen = IdGenerator::new(entity_type);
        generators.insert(type_code, gen.clone());
        gen
    }

    fn proto_to_edge_key(key: Option<EdgeKey_>) -> Result<EdgeKey, Status> {
        let key = key.ok_or_else(|| Status::invalid_argument("Missing edge key"))?;
        let edge_type = EdgeType::from_raw(key.edge_type as u16)
            .ok_or_else(|| Status::invalid_argument("Invalid edge type"))?;
        Ok(EdgeKey::new(EntityId::from_raw(key.src), edge_type))
    }
}

type EdgeKey_ = crate::proto::EdgeKey;

#[tonic::async_trait]
impl<S: EdgeStore + 'static> Aoee for AoeeService<S> {
    async fn add_edge(
        &self,
        request: Request<AddEdgeRequest>,
    ) -> Result<Response<AddEdgeResponse>, Status> {
        let req = request.into_inner();
        let key = Self::proto_to_edge_key(req.key)?;
        let dst = EntityId::from_raw(req.dst);

        let timestamp = self.manager
            .add_edge_with_metadata(key, dst, req.timestamp, req.metadata as u8)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(AddEdgeResponse { 
            success: true,
            timestamp,
        }))
    }

    async fn delete_edge(
        &self,
        request: Request<DeleteEdgeRequest>,
    ) -> Result<Response<DeleteEdgeResponse>, Status> {
        let req = request.into_inner();
        let key = Self::proto_to_edge_key(req.key)?;
        let dst = EntityId::from_raw(req.dst);

        self.manager
            .delete_edge(key, dst)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeleteEdgeResponse { success: true }))
    }

    async fn neighbors(
        &self,
        request: Request<NeighborsRequest>,
    ) -> Result<Response<NeighborsResponse>, Status> {
        let req = request.into_inner();
        let key = Self::proto_to_edge_key(req.key)?;
        let limit = req.limit as usize;

        if req.include_metadata {
            // Get neighbors with timestamps and metadata
            let entries = self.manager
                .neighbors_with_metadata(key, limit)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            let mut neighbors = Vec::with_capacity(entries.len());
            let mut timestamps = Vec::with_capacity(entries.len());
            let mut metadata = Vec::with_capacity(entries.len());

            for (id, ts, meta) in entries {
                neighbors.push(id.as_raw());
                timestamps.push(ts);
                metadata.push(meta as u32);
            }

            Ok(Response::new(NeighborsResponse {
                neighbors,
                next_cursor: 0,
                timestamps,
                metadata,
            }))
        } else {
            // Fast path: just get neighbor IDs
            let neighbors = if limit > 0 {
                self.manager
                    .neighbors_limited(key, limit)
                    .await
            } else {
                self.manager.neighbors(key).await
            }
            .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(NeighborsResponse {
                neighbors: neighbors.into_iter().map(|id| id.as_raw()).collect(),
                next_cursor: 0,
                timestamps: Vec::new(),
                metadata: Vec::new(),
            }))
        }
    }

    async fn contains(
        &self,
        request: Request<ContainsRequest>,
    ) -> Result<Response<ContainsResponse>, Status> {
        let req = request.into_inner();
        let key = Self::proto_to_edge_key(req.key)?;
        let dst = EntityId::from_raw(req.dst);

        let exists = self
            .manager
            .contains(key, dst)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ContainsResponse { exists }))
    }

    async fn count(
        &self,
        request: Request<CountRequest>,
    ) -> Result<Response<CountResponse>, Status> {
        let req = request.into_inner();
        let key = Self::proto_to_edge_key(req.key)?;

        let count = self
            .manager
            .count(key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CountResponse {
            count: count as u64,
        }))
    }

    async fn intersect(
        &self,
        request: Request<IntersectRequest>,
    ) -> Result<Response<IntersectResponse>, Status> {
        let req = request.into_inner();
        let key1 = Self::proto_to_edge_key(req.key1)?;
        let key2 = Self::proto_to_edge_key(req.key2)?;

        let ids = self
            .manager
            .intersect(key1, key2)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(IntersectResponse {
            ids: ids.into_iter().map(|id| id.as_raw()).collect(),
        }))
    }

    async fn union(
        &self,
        request: Request<UnionRequest>,
    ) -> Result<Response<UnionResponse>, Status> {
        let req = request.into_inner();
        let key1 = Self::proto_to_edge_key(req.key1)?;
        let key2 = Self::proto_to_edge_key(req.key2)?;

        let ids = self
            .manager
            .union(key1, key2)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UnionResponse {
            ids: ids.into_iter().map(|id| id.as_raw()).collect(),
        }))
    }

    async fn friend_of_friend(
        &self,
        request: Request<FofRequest>,
    ) -> Result<Response<FofResponse>, Status> {
        let req = request.into_inner();
        let source = EntityId::from_raw(req.source);
        let edge_type = EdgeType::from_raw(req.edge_type as u16)
            .ok_or_else(|| Status::invalid_argument("Invalid edge type"))?;

        // Get direct friends
        let key = EdgeKey::new(source, edge_type);
        let direct_friends = self
            .manager
            .neighbors(key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Configure FOF query
        let config = FofConfig {
            fanout_cap: if req.fanout_cap > 0 {
                req.fanout_cap as usize
            } else {
                1000
            },
            max_results: if req.max_results > 0 {
                req.max_results as usize
            } else {
                100
            },
            min_score: if req.min_score > 0 {
                req.min_score as usize
            } else {
                1
            },
            ..Default::default()
        };

        let exclusions: Vec<EntityId> = req.exclusions.iter().map(|&id| EntityId::from_raw(id)).collect();

        // Execute FOF query
        let query = FofQuery::new(config);
        let manager = self.manager.clone();
        let result = query.execute(
            source,
            &direct_friends,
            |id| {
                // Synchronous wrapper - in production would need async support
                let key = EdgeKey::new(id, edge_type);
                // This is simplified - in production we'd use async properly
                Vec::new() // Placeholder
            },
            &exclusions,
        );

        Ok(Response::new(FofResponse {
            candidates: result
                .candidates
                .into_iter()
                .map(|c| FofCandidate {
                    id: c.id.as_raw(),
                    score: c.score as u32,
                })
                .collect(),
            truncated: result.truncated,
            elapsed_ms: result.elapsed_ms,
        }))
    }

    async fn stats(
        &self,
        request: Request<StatsRequest>,
    ) -> Result<Response<StatsResponse>, Status> {
        let req = request.into_inner();

        let aggregated = self.manager.aggregated_stats().await;
        let per_shard_stats = if req.per_shard {
            self.manager
                .stats()
                .await
                .into_iter()
                .map(|(shard_id, s)| ShardStats {
                    shard_id,
                    cached_lists: s.cached_lists as u64,
                    total_edges: s.total_edges as u64,
                    reads: s.reads,
                    writes: s.writes,
                    cache_hits: s.cache_hits,
                    cache_misses: s.cache_misses,
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(Response::new(StatsResponse {
            aggregated: Some(ShardStats {
                shard_id: 0,
                cached_lists: aggregated.cached_lists as u64,
                total_edges: aggregated.total_edges as u64,
                reads: aggregated.reads,
                writes: aggregated.writes,
                cache_hits: aggregated.cache_hits,
                cache_misses: aggregated.cache_misses,
            }),
            per_shard: per_shard_stats,
        }))
    }

    async fn flush_cache(
        &self,
        request: Request<FlushCacheRequest>,
    ) -> Result<Response<FlushCacheResponse>, Status> {
        let req = request.into_inner();
        
        // Flush all entries (storage write happens via write-through)
        let entries = self.manager.flush_all().await;
        
        // Optionally clear cache after flush
        if req.clear_after_flush {
            self.manager.clear_all_caches().await;
        }
        
        Ok(Response::new(FlushCacheResponse {
            entries_flushed: entries as u64,
            success: true,
        }))
    }

    async fn clear_cache(
        &self,
        request: Request<ClearCacheRequest>,
    ) -> Result<Response<ClearCacheResponse>, Status> {
        let req = request.into_inner();
        
        let entries_cleared = if req.shard_id > 0 {
            // Clear specific shard
            self.manager
                .clear_shard_cache(req.shard_id)
                .await
                .map_err(|e| Status::not_found(e.to_string()))?
        } else {
            // Clear all shards
            self.manager.clear_all_caches().await
        };
        
        Ok(Response::new(ClearCacheResponse {
            entries_cleared: entries_cleared as u64,
            success: true,
        }))
    }

    async fn generate_ids(
        &self,
        request: Request<GenerateIdsRequest>,
    ) -> Result<Response<GenerateIdsResponse>, Status> {
        let req = request.into_inner();
        
        // Validate entity type
        let entity_type = EntityType::from_raw(req.entity_type as u16);
        if entity_type == EntityType::Unknown {
            return Err(Status::invalid_argument(format!(
                "Invalid entity type: {}. Valid types: 0=User, 1=Post, 2=Comment, 3=Photo, etc.",
                req.entity_type
            )));
        }
        
        // Validate count (default to 1, cap at MAX_GENERATE_IDS)
        let count = if req.count == 0 {
            1
        } else if req.count > MAX_GENERATE_IDS {
            return Err(Status::invalid_argument(format!(
                "Count {} exceeds maximum of {}",
                req.count, MAX_GENERATE_IDS
            )));
        } else {
            req.count
        };
        
        // Get or create generator for this type
        let generator = self.get_or_create_generator(entity_type);
        
        // Generate IDs
        let ids: Vec<u64> = generator
            .next_ids(count as usize)
            .into_iter()
            .map(|id| id.as_raw())
            .collect();
        
        Ok(Response::new(GenerateIdsResponse {
            ids,
            entity_type: entity_type.as_raw() as u32,
        }))
    }
}

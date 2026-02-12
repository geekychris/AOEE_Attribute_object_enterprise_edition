//! AOEE Client - gRPC client library

use aoee_core::{EdgeKey, EdgeType, EntityId};
use thiserror::Error;
use tonic::transport::Channel;

pub mod proto {
    tonic::include_proto!("aoee");
}

use proto::aoee_client::AoeeClient as GrpcClient;
use proto::*;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Connection error: {0}")]
    Connection(#[from] tonic::transport::Error),
    #[error("RPC error: {0}")]
    Rpc(#[from] tonic::Status),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// AOEE client
pub struct AoeeClient {
    inner: GrpcClient<Channel>,
}

impl AoeeClient {
    /// Connect to an AOEE server
    pub async fn connect(addr: impl Into<String>) -> Result<Self> {
        let inner = GrpcClient::connect(addr.into()).await?;
        Ok(AoeeClient { inner })
    }

    fn make_edge_key(key: EdgeKey) -> proto::EdgeKey {
        proto::EdgeKey {
            src: key.src.as_raw(),
            edge_type: key.edge_type.as_raw() as u32,
        }
    }

    /// Add an edge
    pub async fn add_edge(&mut self, key: EdgeKey, dst: EntityId) -> Result<u64> {
        self.add_edge_with_metadata(key, dst, 0, 0).await
    }

    /// Add an edge with timestamp and metadata
    pub async fn add_edge_with_metadata(
        &mut self,
        key: EdgeKey,
        dst: EntityId,
        timestamp: u64,
        metadata: u8,
    ) -> Result<u64> {
        let request = AddEdgeRequest {
            key: Some(Self::make_edge_key(key)),
            dst: dst.as_raw(),
            timestamp,
            metadata: metadata as u32,
        };
        let response = self.inner.add_edge(request).await?;
        Ok(response.into_inner().timestamp)
    }

    /// Delete an edge
    pub async fn delete_edge(&mut self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        let request = DeleteEdgeRequest {
            key: Some(Self::make_edge_key(key)),
            dst: dst.as_raw(),
        };
        let response = self.inner.delete_edge(request).await?;
        Ok(response.into_inner().success)
    }

    /// Get neighbors
    pub async fn neighbors(&mut self, key: EdgeKey) -> Result<Vec<EntityId>> {
        let request = NeighborsRequest {
            key: Some(Self::make_edge_key(key)),
            limit: 0,
            cursor: 0,
            include_metadata: false,
        };
        let response = self.inner.neighbors(request).await?;
        Ok(response
            .into_inner()
            .neighbors
            .into_iter()
            .map(EntityId::from_raw)
            .collect())
    }

    /// Get neighbors with limit
    pub async fn neighbors_limited(&mut self, key: EdgeKey, limit: u32) -> Result<Vec<EntityId>> {
        let request = NeighborsRequest {
            key: Some(Self::make_edge_key(key)),
            limit,
            cursor: 0,
            include_metadata: false,
        };
        let response = self.inner.neighbors(request).await?;
        Ok(response
            .into_inner()
            .neighbors
            .into_iter()
            .map(EntityId::from_raw)
            .collect())
    }

    /// Check if edge exists
    pub async fn contains(&mut self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        let request = ContainsRequest {
            key: Some(Self::make_edge_key(key)),
            dst: dst.as_raw(),
        };
        let response = self.inner.contains(request).await?;
        Ok(response.into_inner().exists)
    }

    /// Get edge count
    pub async fn count(&mut self, key: EdgeKey) -> Result<u64> {
        let request = CountRequest {
            key: Some(Self::make_edge_key(key)),
        };
        let response = self.inner.count(request).await?;
        Ok(response.into_inner().count)
    }

    /// Intersect two edge lists
    pub async fn intersect(&mut self, key1: EdgeKey, key2: EdgeKey) -> Result<Vec<EntityId>> {
        let request = IntersectRequest {
            key1: Some(Self::make_edge_key(key1)),
            key2: Some(Self::make_edge_key(key2)),
        };
        let response = self.inner.intersect(request).await?;
        Ok(response
            .into_inner()
            .ids
            .into_iter()
            .map(EntityId::from_raw)
            .collect())
    }

    /// Union two edge lists
    pub async fn union(&mut self, key1: EdgeKey, key2: EdgeKey) -> Result<Vec<EntityId>> {
        let request = UnionRequest {
            key1: Some(Self::make_edge_key(key1)),
            key2: Some(Self::make_edge_key(key2)),
        };
        let response = self.inner.union(request).await?;
        Ok(response
            .into_inner()
            .ids
            .into_iter()
            .map(EntityId::from_raw)
            .collect())
    }

    /// Get stats
    pub async fn stats(&mut self, per_shard: bool) -> Result<StatsResponse> {
        let request = StatsRequest { per_shard };
        let response = self.inner.stats(request).await?;
        Ok(response.into_inner())
    }
}

//! HTTP storage backend for AOEE.
//!
//! Calls an external REST API (like the Java aoee-persistence service)
//! for durable edge storage.

use crate::traits::{EdgeStore, Result, StorageError, StorageStats, StoredEdge};
use aoee_core::{EdgeKey, EdgeType, EntityId};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn, error};

/// HTTP-based edge storage that calls a REST API.
///
/// Compatible with the Java aoee-persistence service.
pub struct HttpStore {
    client: Client,
    base_url: String,
}

/// Edge request for creating edges
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateEdgeRequest {
    src: i64,
    edge_type: String,
    dst: i64,
    timestamp_ns: Option<i64>,
    metadata: Option<i32>,
}

/// Edge response from the API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EdgeResponse {
    #[allow(dead_code)]
    id: Option<i64>,
    src_id: i64,
    #[allow(dead_code)]
    edge_type: String,
    dst_id: i64,
    timestamp_ns: Option<i64>,
    metadata: Option<i32>,
    #[allow(dead_code)]
    created_at: Option<String>,
}

/// Neighbors response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NeighborsResponse {
    #[allow(dead_code)]
    src: i64,
    #[allow(dead_code)]
    edge_type: String,
    neighbors: Vec<i64>,
    #[allow(dead_code)]
    count: usize,
}

/// Exists response
#[derive(Debug, Deserialize)]
struct ExistsResponse {
    exists: bool,
}

/// Count response
#[derive(Debug, Deserialize)]
struct CountResponse {
    count: i64,
}

/// Stats response
#[derive(Debug, Deserialize)]
struct StatsResponse {
    entities: Option<EntityStats>,
    edges: Option<EdgeStats>,
}

#[derive(Debug, Deserialize)]
struct EntityStats {
    total: i64,
    #[allow(dead_code)]
    types: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EdgeStats {
    total: i64,
    #[allow(dead_code)]
    types: Vec<String>,
}

impl HttpStore {
    /// Create a new HTTP store with the given base URL.
    ///
    /// Example: `HttpStore::new("http://localhost:8081")`
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        HttpStore {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Create with custom client configuration
    pub fn with_client(base_url: &str, client: Client) -> Self {
        HttpStore {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Convert EdgeType to string name
    fn edge_type_name(edge_type: EdgeType) -> &'static str {
        match edge_type {
            EdgeType::Follows => "FOLLOWS",
            EdgeType::FollowedBy => "FOLLOWED_BY",
            EdgeType::FriendOf => "FRIEND_OF",
            EdgeType::Blocks => "BLOCKS",
            EdgeType::BlockedBy => "BLOCKED_BY",
            EdgeType::Likes => "LIKES",
            EdgeType::LikedBy => "LIKED_BY",
            EdgeType::CommentsOn => "COMMENTS_ON",
            EdgeType::HasComment => "HAS_COMMENT",
            EdgeType::Shares => "SHARES",
            EdgeType::SharedBy => "SHARED_BY",
            EdgeType::Authored => "AUTHORED",
            EdgeType::AuthoredBy => "AUTHORED_BY",
            EdgeType::MemberOf => "MEMBER_OF",
            EdgeType::HasMember => "HAS_MEMBER",
            EdgeType::Administers => "ADMINISTERS",
            EdgeType::AdministeredBy => "ADMINISTERED_BY",
            EdgeType::Contains => "CONTAINS",
            EdgeType::ContainedIn => "CONTAINED_IN",
            EdgeType::TaggedIn => "TAGGED_IN",
            EdgeType::HasTag => "HAS_TAG",
            EdgeType::Mentions => "MENTIONS",
            EdgeType::MentionedBy => "MENTIONED_BY",
            _ => "CUSTOM",
        }
    }

    /// Check if the service is available
    pub async fn health_check(&self) -> bool {
        match self.client.get(format!("{}/actuator/health", self.base_url))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl EdgeStore for HttpStore {
    async fn persist_edge_with_metadata(
        &self,
        key: EdgeKey,
        dst: EntityId,
        timestamp: u64,
        metadata: u8,
    ) -> Result<()> {
        let url = format!("{}/api/v1/edges", self.base_url);
        
        let request = CreateEdgeRequest {
            src: key.src.as_raw() as i64,
            edge_type: Self::edge_type_name(key.edge_type).to_string(),
            dst: dst.as_raw() as i64,
            timestamp_ns: Some(timestamp as i64),
            metadata: Some(metadata as i32),
        };

        debug!("HTTP persist_edge: {} -[{:?}]-> {} (metadata={})", 
               key.src.as_raw(), key.edge_type, dst.as_raw(), metadata);

        match self.client.post(&url)
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    warn!("HTTP persist_edge failed: {} - {}", status, text);
                    Err(StorageError::Internal(format!("HTTP error: {} - {}", status, text)))
                }
            }
            Err(e) => {
                error!("HTTP persist_edge connection error: {}", e);
                Err(StorageError::Connection(format!("HTTP request failed: {}", e)))
            }
        }
    }

    async fn persist_delete(&self, key: EdgeKey, dst: EntityId, _timestamp: u64) -> Result<()> {
        let url = format!(
            "{}/api/v1/edges/{}/{}/{}",
            self.base_url,
            key.src.as_raw(),
            Self::edge_type_name(key.edge_type),
            dst.as_raw()
        );

        debug!("HTTP persist_delete: {} -[{:?}]-> {}", key.src.as_raw(), key.edge_type, dst.as_raw());

        match self.client.delete(&url).send().await {
            Ok(response) => {
                if response.status().is_success() || response.status().as_u16() == 404 {
                    // 404 is okay - edge might not exist
                    Ok(())
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    warn!("HTTP persist_delete failed: {} - {}", status, text);
                    Err(StorageError::Internal(format!("HTTP error: {} - {}", status, text)))
                }
            }
            Err(e) => {
                error!("HTTP persist_delete connection error: {}", e);
                Err(StorageError::Connection(format!("HTTP request failed: {}", e)))
            }
        }
    }

    async fn load_edges(&self, key: EdgeKey) -> Result<Vec<StoredEdge>> {
        let url = format!(
            "{}/api/v1/edges?src={}&type={}",
            self.base_url,
            key.src.as_raw(),
            Self::edge_type_name(key.edge_type)
        );

        debug!("HTTP load_edges: src={}, type={:?}", key.src.as_raw(), key.edge_type);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let edges: Vec<EdgeResponse> = response.json().await
                        .map_err(|e| StorageError::Serialization(format!("JSON parse error: {}", e)))?;
                    
                    let stored: Vec<StoredEdge> = edges.into_iter().map(|e| {
                        StoredEdge {
                            dst: EntityId::from_raw(e.dst_id as u64),
                            timestamp: e.timestamp_ns.unwrap_or(0) as u64,
                            deleted: false,
                            metadata: e.metadata.unwrap_or(0) as u8,
                        }
                    }).collect();
                    
                    Ok(stored)
                } else if response.status().as_u16() == 404 {
                    Ok(Vec::new())
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    Err(StorageError::Internal(format!("HTTP error: {} - {}", status, text)))
                }
            }
            Err(e) => {
                error!("HTTP load_edges connection error: {}", e);
                Err(StorageError::Connection(format!("HTTP request failed: {}", e)))
            }
        }
    }

    async fn load_edges_paginated(
        &self,
        key: EdgeKey,
        _cursor: Option<EntityId>,
        limit: usize,
    ) -> Result<Vec<StoredEdge>> {
        // The Java API doesn't support cursor-based pagination in the same way,
        // so we fetch with limit and filter client-side if needed
        let url = format!(
            "{}/api/v1/edges?src={}&type={}&limit={}",
            self.base_url,
            key.src.as_raw(),
            Self::edge_type_name(key.edge_type),
            limit
        );

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let edges: Vec<EdgeResponse> = response.json().await
                        .map_err(|e| StorageError::Serialization(format!("JSON parse error: {}", e)))?;
                    
                    let stored: Vec<StoredEdge> = edges.into_iter()
                        .take(limit)
                        .map(|e| StoredEdge {
                            dst: EntityId::from_raw(e.dst_id as u64),
                            timestamp: e.timestamp_ns.unwrap_or(0) as u64,
                            deleted: false,
                            metadata: e.metadata.unwrap_or(0) as u8,
                        })
                        .collect();
                    
                    Ok(stored)
                } else {
                    Ok(Vec::new())
                }
            }
            Err(e) => Err(StorageError::Connection(format!("HTTP request failed: {}", e))),
        }
    }

    async fn edge_exists(&self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        let url = format!(
            "{}/api/v1/edges/{}/{}/{}/exists",
            self.base_url,
            key.src.as_raw(),
            Self::edge_type_name(key.edge_type),
            dst.as_raw()
        );

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let result: ExistsResponse = response.json().await
                        .map_err(|e| StorageError::Serialization(format!("JSON parse error: {}", e)))?;
                    Ok(result.exists)
                } else {
                    Ok(false)
                }
            }
            Err(e) => Err(StorageError::Connection(format!("HTTP request failed: {}", e))),
        }
    }

    async fn count_edges(&self, key: EdgeKey) -> Result<usize> {
        let url = format!(
            "{}/api/v1/edges/count?src={}&type={}",
            self.base_url,
            key.src.as_raw(),
            Self::edge_type_name(key.edge_type)
        );

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let result: CountResponse = response.json().await
                        .map_err(|e| StorageError::Serialization(format!("JSON parse error: {}", e)))?;
                    Ok(result.count as usize)
                } else {
                    Ok(0)
                }
            }
            Err(e) => Err(StorageError::Connection(format!("HTTP request failed: {}", e))),
        }
    }

    async fn scan_keys(&self, _prefix: Option<EntityId>) -> Result<Vec<EdgeKey>> {
        // The HTTP API doesn't support key scanning directly
        // This would require a new endpoint in the Java service
        warn!("HTTP scan_keys not fully supported - returning empty");
        Ok(Vec::new())
    }

    async fn persist_batch(&self, operations: Vec<(EdgeKey, EntityId, u64, bool)>) -> Result<()> {
        // Process batch sequentially (the Java API doesn't have a batch endpoint)
        // Could be optimized with concurrent requests
        for (key, dst, timestamp, is_delete) in operations {
            if is_delete {
                self.persist_delete(key, dst, timestamp).await?;
            } else {
                self.persist_edge(key, dst, timestamp).await?;
            }
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // HTTP is synchronous, no buffering
        Ok(())
    }

    async fn stats(&self) -> StorageStats {
        let url = format!("{}/api/v1/export/stats", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(stats) = response.json::<StatsResponse>().await {
                        return StorageStats {
                            total_keys: stats.entities.map(|e| e.total as usize).unwrap_or(0),
                            total_edges: stats.edges.map(|e| e.total as usize).unwrap_or(0),
                            total_tombstones: 0,
                            bytes_used: 0,
                        };
                    }
                }
                StorageStats::default()
            }
            Err(_) => StorageStats::default(),
        }
    }
}

// Entity persistence methods (not part of EdgeStore trait)
impl HttpStore {
    /// Create a single entity in the persistence layer
    pub async fn create_entity(&self, id: u64, entity_type: &str, name: &str) -> Result<()> {
        let url = format!("{}/api/v1/entities", self.base_url);
        
        let request = serde_json::json!({
            "id": id as i64,
            "entityType": entity_type,
            "name": name
        });

        debug!("HTTP create_entity: {} ({}) - {}", id, entity_type, name);

        match self.client.post(&url)
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    warn!("HTTP create_entity failed: {} - {}", status, text);
                    Err(StorageError::Internal(format!("HTTP error: {} - {}", status, text)))
                }
            }
            Err(e) => {
                error!("HTTP create_entity connection error: {}", e);
                Err(StorageError::Connection(format!("HTTP request failed: {}", e)))
            }
        }
    }

    /// Batch create entities in the persistence layer
    pub async fn create_entities_batch(&self, entities: &[(u64, String, String)]) -> Result<(u64, u64)> {
        let url = format!("{}/api/v1/entities/batch", self.base_url);
        
        let request: Vec<serde_json::Value> = entities.iter()
            .map(|(id, entity_type, name)| {
                serde_json::json!({
                    "id": *id as i64,
                    "entityType": entity_type,
                    "name": name
                })
            })
            .collect();

        debug!("HTTP create_entities_batch: {} entities", entities.len());

        match self.client.post(&url)
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    #[derive(Debug, serde::Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct BatchResponse {
                        entities_created: Option<i64>,
                        success: Option<bool>,
                    }
                    
                    if let Ok(resp) = response.json::<BatchResponse>().await {
                        let created = resp.entities_created.unwrap_or(0) as u64;
                        let failed = entities.len() as u64 - created;
                        Ok((created, failed))
                    } else {
                        Ok((entities.len() as u64, 0))
                    }
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    warn!("HTTP create_entities_batch failed: {} - {}", status, text);
                    Err(StorageError::Internal(format!("HTTP error: {} - {}", status, text)))
                }
            }
            Err(e) => {
                error!("HTTP create_entities_batch connection error: {}", e);
                Err(StorageError::Connection(format!("HTTP request failed: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // HTTP tests require a running server, so we skip them in unit tests
    // Integration tests would be in a separate test file
}

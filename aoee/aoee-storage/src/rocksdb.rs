//! RocksDB storage backend for AOEE.
//!
//! Provides persistent edge storage using RocksDB, a high-performance
//! embedded key-value store.

use crate::traits::{EdgeStore, Result, StorageError, StorageStats, StoredEdge};
use aoee_core::{EdgeKey, EdgeType, EntityId};
use async_trait::async_trait;
use rocksdb::{DB, Options, IteratorMode, Direction};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, warn};

/// RocksDB-based edge storage.
///
/// Key format: `{edge_type}:{src_id}:{dst_id}` (big-endian bytes)
/// Value format: `{timestamp:8}:{deleted:1}:{metadata:1}` (10 bytes)
///
/// This format allows efficient prefix scans for all edges from a source.
pub struct RocksDbStore {
    db: Arc<DB>,
}

impl RocksDbStore {
    /// Open or create a RocksDB database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> std::result::Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB
        opts.set_max_write_buffer_number(3);
        opts.set_target_file_size_base(64 * 1024 * 1024);
        opts.set_level_compaction_dynamic_level_bytes(true);
        opts.set_max_background_jobs(4);
        
        let db = DB::open(&opts, path)
            .map_err(|e| StorageError::Internal(format!("Failed to open RocksDB: {}", e)))?;
        
        Ok(RocksDbStore {
            db: Arc::new(db),
        })
    }

    /// Create key bytes for an edge.
    fn make_key(key: &EdgeKey, dst: EntityId) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(25);
        // Edge type (1 byte)
        bytes.push(key.edge_type.as_raw() as u8);
        // Source entity ID (8 bytes, big-endian for ordering)
        bytes.extend_from_slice(&key.src.as_raw().to_be_bytes());
        // Destination entity ID (8 bytes, big-endian)
        bytes.extend_from_slice(&dst.as_raw().to_be_bytes());
        bytes
    }

    /// Create prefix for scanning all edges from a source.
    fn make_prefix(key: &EdgeKey) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(9);
        bytes.push(key.edge_type.as_raw() as u8);
        bytes.extend_from_slice(&key.src.as_raw().to_be_bytes());
        bytes
    }

    /// Parse destination from a full key.
    fn parse_dst(full_key: &[u8]) -> Option<EntityId> {
        if full_key.len() < 17 {
            return None;
        }
        let dst_bytes: [u8; 8] = full_key[9..17].try_into().ok()?;
        let raw = u64::from_be_bytes(dst_bytes);
        Some(EntityId::from_raw(raw))
    }

    /// Create value bytes.
    fn make_value(timestamp: u64, deleted: bool, metadata: u8) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10);
        bytes.extend_from_slice(&timestamp.to_be_bytes());
        bytes.push(if deleted { 1 } else { 0 });
        bytes.push(metadata);
        bytes
    }

    /// Parse value bytes.
    fn parse_value(bytes: &[u8]) -> Option<(u64, bool, u8)> {
        if bytes.len() < 10 {
            return None;
        }
        let timestamp = u64::from_be_bytes(bytes[0..8].try_into().ok()?);
        let deleted = bytes[8] != 0;
        let metadata = bytes[9];
        Some((timestamp, deleted, metadata))
    }

    /// Check if a key has the given prefix.
    fn has_prefix(key: &[u8], prefix: &[u8]) -> bool {
        key.len() >= prefix.len() && &key[..prefix.len()] == prefix
    }
}

#[async_trait]
impl EdgeStore for RocksDbStore {
    async fn persist_edge_with_metadata(
        &self,
        key: EdgeKey,
        dst: EntityId,
        timestamp: u64,
        metadata: u8,
    ) -> Result<()> {
        let db_key = Self::make_key(&key, dst);
        
        // Check if we need to update (timestamp-based conflict resolution)
        if let Ok(Some(existing)) = self.db.get(&db_key) {
            if let Some((existing_ts, _, _)) = Self::parse_value(&existing) {
                if existing_ts >= timestamp {
                    debug!("Skipping edge persist - existing timestamp {} >= {}", existing_ts, timestamp);
                    return Ok(());
                }
            }
        }
        
        let value = Self::make_value(timestamp, false, metadata);
        self.db.put(&db_key, &value)
            .map_err(|e| StorageError::Internal(format!("RocksDB put failed: {}", e)))?;
        
        Ok(())
    }

    async fn persist_delete(&self, key: EdgeKey, dst: EntityId, timestamp: u64) -> Result<()> {
        let db_key = Self::make_key(&key, dst);
        
        // Check timestamp
        if let Ok(Some(existing)) = self.db.get(&db_key) {
            if let Some((existing_ts, _, metadata)) = Self::parse_value(&existing) {
                if existing_ts >= timestamp {
                    debug!("Skipping edge delete - existing timestamp {} >= {}", existing_ts, timestamp);
                    return Ok(());
                }
                // Preserve metadata in tombstone
                let value = Self::make_value(timestamp, true, metadata);
                self.db.put(&db_key, &value)
                    .map_err(|e| StorageError::Internal(format!("RocksDB put failed: {}", e)))?;
                return Ok(());
            }
        }
        
        // No existing entry, create tombstone
        let value = Self::make_value(timestamp, true, 0);
        self.db.put(&db_key, &value)
            .map_err(|e| StorageError::Internal(format!("RocksDB put failed: {}", e)))?;
        
        Ok(())
    }

    async fn load_edges(&self, key: EdgeKey) -> Result<Vec<StoredEdge>> {
        let prefix = Self::make_prefix(&key);
        let mut edges = Vec::new();
        
        let iter = self.db.iterator(IteratorMode::From(&prefix, Direction::Forward));
        
        for item in iter {
            match item {
                Ok((k, v)) => {
                    if !Self::has_prefix(&k, &prefix) {
                        break;
                    }
                    
                    if let (Some(dst), Some((timestamp, deleted, metadata))) = 
                        (Self::parse_dst(&k), Self::parse_value(&v)) 
                    {
                        if !deleted {
                            edges.push(StoredEdge { dst, timestamp, deleted: false, metadata });
                        }
                    }
                }
                Err(e) => {
                    warn!("RocksDB iteration error: {}", e);
                    break;
                }
            }
        }
        
        // Edges are already sorted by destination due to key format
        Ok(edges)
    }

    async fn load_edges_paginated(
        &self,
        key: EdgeKey,
        cursor: Option<EntityId>,
        limit: usize,
    ) -> Result<Vec<StoredEdge>> {
        let prefix = Self::make_prefix(&key);
        let start_key = match cursor {
            Some(c) => Self::make_key(&key, c),
            None => prefix.clone(),
        };
        
        let mut edges = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(&start_key, Direction::Forward));
        
        for item in iter {
            if edges.len() >= limit {
                break;
            }
            
            match item {
                Ok((k, v)) => {
                    if !Self::has_prefix(&k, &prefix) {
                        break;
                    }
                    
                    if let (Some(dst), Some((timestamp, deleted, metadata))) = 
                        (Self::parse_dst(&k), Self::parse_value(&v)) 
                    {
                        // Skip cursor if provided
                        if cursor.is_some() && Some(dst) == cursor {
                            continue;
                        }
                        
                        if !deleted {
                            edges.push(StoredEdge { dst, timestamp, deleted: false, metadata });
                        }
                    }
                }
                Err(e) => {
                    warn!("RocksDB iteration error: {}", e);
                    break;
                }
            }
        }
        
        Ok(edges)
    }

    async fn edge_exists(&self, key: EdgeKey, dst: EntityId) -> Result<bool> {
        let db_key = Self::make_key(&key, dst);
        
        match self.db.get(&db_key) {
            Ok(Some(value)) => {
                if let Some((_, deleted, _)) = Self::parse_value(&value) {
                    Ok(!deleted)
                } else {
                    Ok(false)
                }
            }
            Ok(None) => Ok(false),
            Err(e) => Err(StorageError::Internal(format!("RocksDB get failed: {}", e))),
        }
    }

    async fn count_edges(&self, key: EdgeKey) -> Result<usize> {
        let prefix = Self::make_prefix(&key);
        let mut count = 0;
        
        let iter = self.db.iterator(IteratorMode::From(&prefix, Direction::Forward));
        
        for item in iter {
            match item {
                Ok((k, v)) => {
                    if !Self::has_prefix(&k, &prefix) {
                        break;
                    }
                    
                    if let Some((_, deleted, _)) = Self::parse_value(&v) {
                        if !deleted {
                            count += 1;
                        }
                    }
                }
                Err(_) => break,
            }
        }
        
        Ok(count)
    }

    async fn scan_keys(&self, prefix: Option<EntityId>) -> Result<Vec<EdgeKey>> {
        let mut keys = std::collections::HashSet::new();
        
        let iter = self.db.iterator(IteratorMode::Start);
        
        for item in iter {
            match item {
                Ok((k, _)) => {
                    if k.len() >= 9 {
                        let edge_type_raw = k[0] as u16;
                        if let Some(edge_type) = EdgeType::from_raw(edge_type_raw) {
                            let src_bytes: [u8; 8] = k[1..9].try_into().unwrap();
                            let src_raw = u64::from_be_bytes(src_bytes);
                            let src = EntityId::from_raw(src_raw);
                            
                            // Apply prefix filter if provided
                            if let Some(p) = prefix {
                                if src != p {
                                    continue;
                                }
                            }
                            
                            keys.insert(EdgeKey::new(src, edge_type));
                        }
                    }
                }
                Err(_) => break,
            }
        }
        
        Ok(keys.into_iter().collect())
    }

    async fn persist_batch(&self, operations: Vec<(EdgeKey, EntityId, u64, bool)>) -> Result<()> {
        let mut batch = rocksdb::WriteBatch::default();
        
        for (key, dst, timestamp, is_delete) in operations {
            let db_key = Self::make_key(&key, dst);
            let value = Self::make_value(timestamp, is_delete, 0);
            batch.put(&db_key, &value);
        }
        
        self.db.write(batch)
            .map_err(|e| StorageError::Internal(format!("RocksDB batch write failed: {}", e)))?;
        
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.db.flush()
            .map_err(|e| StorageError::Internal(format!("RocksDB flush failed: {}", e)))?;
        Ok(())
    }

    async fn stats(&self) -> StorageStats {
        // Getting accurate stats from RocksDB would require iterating all keys
        // For now, return approximate stats from RocksDB properties
        let total_keys = self.db.property_int_value("rocksdb.estimate-num-keys")
            .ok()
            .flatten()
            .unwrap_or(0) as usize;
        
        let bytes_used = self.db.property_int_value("rocksdb.total-sst-files-size")
            .ok()
            .flatten()
            .unwrap_or(0) as usize;
        
        StorageStats {
            total_keys,
            total_edges: 0, // Would need to iterate to count
            total_tombstones: 0,
            bytes_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aoee_core::EntityType;
    use tempfile::tempdir;

    fn make_id(raw: u64) -> EntityId {
        EntityId::new(EntityType::User, raw)
    }

    fn make_key(src: u64, edge_type: EdgeType) -> EdgeKey {
        EdgeKey::new(make_id(src), edge_type)
    }

    #[tokio::test]
    async fn test_persist_and_load() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::open(dir.path()).unwrap();
        
        let key = make_key(1, EdgeType::Follows);
        
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        store.persist_edge(key, make_id(3), 101).await.unwrap();
        store.persist_edge(key, make_id(4), 102).await.unwrap();
        
        let edges = store.load_edges(key).await.unwrap();
        
        assert_eq!(edges.len(), 3);
        assert_eq!(edges[0].dst, make_id(2));
        assert_eq!(edges[1].dst, make_id(3));
        assert_eq!(edges[2].dst, make_id(4));
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::open(dir.path()).unwrap();
        
        let key = make_key(1, EdgeType::Follows);
        
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        store.persist_edge(key, make_id(3), 101).await.unwrap();
        store.persist_delete(key, make_id(2), 102).await.unwrap();
        
        let edges = store.load_edges(key).await.unwrap();
        
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].dst, make_id(3));
    }

    #[tokio::test]
    async fn test_edge_exists() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::open(dir.path()).unwrap();
        
        let key = make_key(1, EdgeType::Follows);
        
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        
        assert!(store.edge_exists(key, make_id(2)).await.unwrap());
        assert!(!store.edge_exists(key, make_id(3)).await.unwrap());
    }

    #[tokio::test]
    async fn test_timestamp_ordering() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::open(dir.path()).unwrap();
        
        let key = make_key(1, EdgeType::Follows);
        
        // Delete first with newer timestamp
        store.persist_delete(key, make_id(2), 200).await.unwrap();
        // Then try to add with older timestamp - should be ignored
        store.persist_edge(key, make_id(2), 100).await.unwrap();
        
        assert!(!store.edge_exists(key, make_id(2)).await.unwrap());
        
        // Add with even newer timestamp - should succeed
        store.persist_edge(key, make_id(2), 300).await.unwrap();
        
        assert!(store.edge_exists(key, make_id(2)).await.unwrap());
    }
}

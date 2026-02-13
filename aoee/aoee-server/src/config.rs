//! Server configuration

use aoee_shard::manager::ManagerConfig;
use serde::{Deserialize, Serialize};
use std::env;

/// Storage backend type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    /// In-memory only (no persistence)
    Memory,
    /// RocksDB embedded database
    RocksDb,
    /// HTTP backend (e.g., Java aoee-persistence service)
    Http,
}

impl Default for StorageType {
    fn default() -> Self {
        StorageType::Memory
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub storage_type: StorageType,
    /// Enable write-through to storage
    pub write_through: bool,
    /// RocksDB data directory (for RocksDb type)
    pub rocksdb_path: String,
    /// HTTP service URL (for Http type)
    pub http_url: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            storage_type: StorageType::Memory,
            write_through: true,
            rocksdb_path: "./data/aoee-rocksdb".to_string(),
            http_url: "http://localhost:8081".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server listen address
    pub listen_addr: String,
    /// Number of shards
    pub num_shards: u32,
    /// Enable metrics endpoint
    pub metrics_enabled: bool,
    /// Metrics listen address
    pub metrics_addr: String,
    /// Storage configuration
    pub storage: StorageConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            listen_addr: "[::1]:50051".to_string(),
            num_shards: 4,
            metrics_enabled: true,
            metrics_addr: "[::1]:9090".to_string(),
            storage: StorageConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Server settings
        if let Ok(addr) = env::var("AOEE_LISTEN_ADDR") {
            config.listen_addr = addr;
        }
        if let Ok(shards) = env::var("AOEE_NUM_SHARDS") {
            if let Ok(n) = shards.parse() {
                config.num_shards = n;
            }
        }
        
        // Storage settings
        if let Ok(storage_type) = env::var("AOEE_STORAGE_TYPE") {
            config.storage.storage_type = match storage_type.to_lowercase().as_str() {
                "rocksdb" | "rocks" => StorageType::RocksDb,
                "http" | "remote" => StorageType::Http,
                _ => StorageType::Memory,
            };
        }
        if let Ok(val) = env::var("AOEE_WRITE_THROUGH") {
            config.storage.write_through = val.to_lowercase() == "true" || val == "1";
        }
        if let Ok(path) = env::var("AOEE_ROCKSDB_PATH") {
            config.storage.rocksdb_path = path;
        }
        if let Ok(url) = env::var("AOEE_HTTP_URL") {
            config.storage.http_url = url;
        }
        
        config
    }
    
    pub fn to_manager_config(&self) -> ManagerConfig {
        let mut config = ManagerConfig {
            num_shards: self.num_shards,
            ..Default::default()
        };
        config.shard_config.write_through = self.storage.write_through;
        config
    }
}

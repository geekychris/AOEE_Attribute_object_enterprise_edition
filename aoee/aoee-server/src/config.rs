//! Server configuration

use aoee_shard::manager::ManagerConfig;
use serde::{Deserialize, Serialize};

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
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            listen_addr: "[::1]:50051".to_string(),
            num_shards: 4,
            metrics_enabled: true,
            metrics_addr: "[::1]:9090".to_string(),
        }
    }
}

impl ServerConfig {
    pub fn to_manager_config(&self) -> ManagerConfig {
        ManagerConfig {
            num_shards: self.num_shards,
            ..Default::default()
        }
    }
}

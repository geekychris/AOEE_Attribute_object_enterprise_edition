//! AOEE Server binary
//!
//! Supports multiple storage backends via environment variables:
//!
//! - `AOEE_STORAGE_TYPE`: memory (default), rocksdb, http
//! - `AOEE_WRITE_THROUGH`: true/false (default: true)
//! - `AOEE_ROCKSDB_PATH`: path for RocksDB data (default: ./data/aoee-rocksdb)
//! - `AOEE_HTTP_URL`: URL for HTTP backend (default: http://localhost:8081)

use aoee_server::{proto::aoee_server::AoeeServer, AoeeService, ServerConfig, StorageType};
use aoee_storage::InMemoryStore;
use std::sync::Arc;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

// Include the file descriptor set for gRPC reflection
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("aoee_descriptor");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load config from environment
    let config = ServerConfig::from_env();
    
    info!("Starting AOEE server on {}", config.listen_addr);
    info!("Storage type: {:?}", config.storage.storage_type);
    info!("Write-through: {}", config.storage.write_through);

    // Create service based on storage type
    let manager_config = config.to_manager_config();
    let addr = config.listen_addr.parse()?;
    
    match config.storage.storage_type {
        StorageType::Memory => {
            info!("Using in-memory storage (no persistence)");
            let service = AoeeService::new_in_memory(manager_config).await;
            let reflection_service = ReflectionBuilder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build()?;
            Server::builder()
                .add_service(reflection_service)
                .add_service(AoeeServer::new(service))
                .serve(addr)
                .await?;
        }
        
        #[cfg(feature = "rocksdb-backend")]
        StorageType::RocksDb => {
            use aoee_storage::RocksDbStore;
            
            let path = config.storage.rocksdb_path.clone();
            info!("Using RocksDB storage at: {}", path);
            
            // Create directory if needed
            std::fs::create_dir_all(&path)?;
            
            let service = AoeeService::new(manager_config, move |shard_id| {
                let shard_path = format!("{}/shard_{}", path, shard_id);
                match RocksDbStore::open(&shard_path) {
                    Ok(store) => Arc::new(store),
                    Err(e) => {
                        panic!("Failed to open RocksDB at {}: {}", shard_path, e);
                    }
                }
            }).await;
            
            let reflection_service = ReflectionBuilder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build()?;
            Server::builder()
                .add_service(reflection_service)
                .add_service(AoeeServer::new(service))
                .serve(addr)
                .await?;
        }
        
        #[cfg(not(feature = "rocksdb-backend"))]
        StorageType::RocksDb => {
            panic!("RocksDB backend not compiled in. Rebuild with --features rocksdb-backend");
        }
        
        #[cfg(feature = "http-backend")]
        StorageType::Http => {
            use aoee_storage::HttpStore;
            
            let url = config.storage.http_url.clone();
            info!("Using HTTP storage at: {}", url);
            
            // Check if service is available
            let test_store = HttpStore::new(&url);
            if !test_store.health_check().await {
                warn!("HTTP persistence service at {} is not responding - continuing anyway", url);
            } else {
                info!("HTTP persistence service is healthy");
            }
            
            let service = AoeeService::new(manager_config, move |_shard_id| {
                // All shards share the same HTTP backend
                Arc::new(HttpStore::new(&url))
            }).await;
            
            let reflection_service = ReflectionBuilder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build()?;
            Server::builder()
                .add_service(reflection_service)
                .add_service(AoeeServer::new(service))
                .serve(addr)
                .await?;
        }
        
        #[cfg(not(feature = "http-backend"))]
        StorageType::Http => {
            panic!("HTTP backend not compiled in. Rebuild with --features http-backend");
        }
    }

    Ok(())
}

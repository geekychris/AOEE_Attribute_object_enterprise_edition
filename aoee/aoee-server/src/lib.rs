//! AOEE Server - gRPC service implementation
//!
//! Supports multiple storage backends:
//! - In-memory (default)
//! - RocksDB (local persistence)
//! - HTTP (remote persistence service)

pub mod service;
pub mod config;

pub mod proto {
    tonic::include_proto!("aoee");
}

pub use config::{ServerConfig, StorageConfig, StorageType};
pub use service::AoeeService;

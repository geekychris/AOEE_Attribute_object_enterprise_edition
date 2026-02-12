//! AOEE Server - gRPC service implementation

pub mod service;
pub mod config;

pub mod proto {
    tonic::include_proto!("aoee");
}

pub use config::ServerConfig;
pub use service::AoeeService;

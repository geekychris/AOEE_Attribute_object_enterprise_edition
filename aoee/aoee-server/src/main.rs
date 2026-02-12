//! AOEE Server binary

use aoee_server::{proto::aoee_server::AoeeServer, AoeeService, ServerConfig};
use aoee_shard::manager::ManagerConfig;
use tonic::transport::Server;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load config (could be from file)
    let config = ServerConfig::default();
    info!("Starting AOEE server on {}", config.listen_addr);

    // Create service
    let manager_config = config.to_manager_config();
    let service = AoeeService::new_in_memory(manager_config).await;

    // Start server
    let addr = config.listen_addr.parse()?;
    Server::builder()
        .add_service(AoeeServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

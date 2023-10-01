use tonic::transport::Server;

use cluster_management_service::ClusterManagementService;
use railyard::cluster_management_server::ClusterManagementServer;
use railyard::railyard_server::RailyardServer;
use railyard_service::RailyardService;

mod cluster_management_service;
mod railyard_service;

pub mod railyard {
    tonic::include_proto!("railyard");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:8000".parse()?;
    let management_addr = "[::1]:8001".parse()?;
    let railyard_service = RailyardService::default();
    let management_service = ClusterManagementService::default();

    Server::builder()
        .add_service(RailyardServer::new(railyard_service))
        .serve(addr)
        .await?;

    Server::builder()
        .add_service(ClusterManagementServer::new(management_service))
        .serve(management_addr)
        .await?;

    Ok(())
}

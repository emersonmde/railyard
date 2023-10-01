use railyard_service::RailyardService;
use railyard::railyard_server::RailyardServer;
use railyard::cluster_management_server::ClusterManagementServer;
use tonic::transport::Server;

mod railyard_service;
mod cluster_management_service;

pub mod railyard {
    tonic::include_proto!("railyard");
    tonic::include_proto!("cluster_management");
}

struct

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:8000".parse()?;
    let management_addr = "[::1]:8001".parse()?;
    let railyard_service = RailyardService::default();

    Server::builder()
        .add_service(RailyardServer::new(railyard_service))
        .serve(addr)
        .await?;

    Server::builder()
        .add_service(ClusterManagementServer::new(railyard_service))
        .serve(management_addr)
        .await?;

    Ok(())
}

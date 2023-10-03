use clap::{arg, command};
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
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
    let matches = command!()
        .arg(arg!(-p --port <PORT> "Port used for management API").required(true))
        .get_matches();

    // let addr = "[::1]:8000".parse()?;
    // let railyard_service = RailyardService::default();
    // Server::builder()
    //     .add_service(RailyardServer::new(railyard_service))
    //     .serve(addr)
    //     .await?;

    let port = matches.get_one::<String>("port").unwrap();
    let management_addr = SocketAddr::from_str(&format!("[::1]:{}", port))
        .expect("Failed to construct management SocketAddr");
    let management_service = ClusterManagementService::default();

    Server::builder()
        .add_service(ClusterManagementServer::new(management_service))
        .serve(management_addr)
        .await?;

    Ok(())
}

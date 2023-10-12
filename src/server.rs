use std::net::SocketAddr;
use std::str::FromStr;

use clap::{arg, command, ArgAction};
use tonic::transport::Server;

use cluster_management_service::ClusterManagementService;
use railyard::cluster_management_server::ClusterManagementServer;

mod cluster_management_service;
mod railyard_service;

pub mod railyard {
    tonic::include_proto!("railyard");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = command!()
        .arg(arg!(-p --port <PORT> "Port used for management API").required(true))
        .arg(
            arg!(--peer <PEER_ADDRESS> "The address of a peer node")
                .action(ArgAction::Append)
                .required(true),
        )
        .get_matches();

    let port = matches.get_one::<String>("port").unwrap();
    let management_addr = SocketAddr::from_str(&format!("[::1]:{}", port))
        .expect("Failed to construct management SocketAddr");
    let management_service = ClusterManagementService::new().await;

    let peers: Vec<_> = matches.get_many::<String>("peer").unwrap().collect();
    println!("Peers {:?}", peers);

    Server::builder()
        .add_service(ClusterManagementServer::new(management_service))
        .serve(management_addr)
        .await?;

    Ok(())
}

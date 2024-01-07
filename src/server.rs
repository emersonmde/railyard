use std::net::SocketAddr;
use std::str::FromStr;

use clap::{arg, command, ArgAction};
use tonic::transport::Server;

use crate::railyard::railyard_server::RailyardServer;
use crate::railyard_service::RailyardService;

mod railyard_service;

pub mod railyard {
    tonic::include_proto!("railyard");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    let matches = command!()
        .arg(arg!(-p --port <PORT> "Port used for management API").required(true))
        .arg(
            arg!(--peer <PEER_ADDRESS> "The address of a peer node")
                .action(ArgAction::Append)
                .required(true),
        )
        .get_matches();

    let port = matches.get_one::<String>("port").unwrap();
    let management_addr = SocketAddr::from_str(&format!("127.0.0.1:{}", port))
        .expect("Failed to construct management SocketAddr");
    let peers: Vec<_> = matches.get_many::<String>("peer").unwrap().collect();
    println!("Peers {:?}", peers);

    // let railyard_service = RailyardService::new(peers).await;
    let railyard_service: RailyardService = if port == "8001" {
        RailyardService::new_with_data(peers).await
    } else {
        RailyardService::new(peers).await
    };
    Server::builder()
        .add_service(RailyardServer::new(railyard_service))
        .serve(management_addr)
        .await?;

    Ok(())
}

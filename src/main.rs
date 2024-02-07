use std::net::SocketAddr;
use std::str::FromStr;

use clap::{arg, command, ArgAction};
use railyard::create_railyard_server;

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
    let addr = SocketAddr::from_str(&format!("127.0.0.1:{}", port))
        .expect("Failed to construct SocketAddr");
    let peers: Vec<_> = matches.get_many::<String>("peer").unwrap().collect();
    println!("Peers {:?}", peers);

    let router = create_railyard_server(peers, port).await?;

    router.serve(addr).await?;

    Ok(())
}

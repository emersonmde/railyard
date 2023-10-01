use node::Node;
use railyard::railyard_server::RailyardServer;
use tonic::transport::Server;

mod node;

pub mod railyard {
    tonic::include_proto!("railyard");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:8000".parse()?;
    let node = Node::default();

    Server::builder()
        .add_service(RailyardServer::new(node))
        .serve(addr)
        .await?;

    Ok(())
}

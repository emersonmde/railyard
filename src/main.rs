use tonic::transport::Server;
use railyard::{service::RailyardService, railyard::railyard_server::RailyardServer};

mod railyard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;

    let railyard_service = RailyardService { 
     };

    Server::builder()
        .add_service(RailyardServer::new(railyard_service))
        .serve(addr)
        .await?;

    Ok(())
}

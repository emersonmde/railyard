use tonic::{Request, Response, Status};
use tonic::transport::Server;
use railyard::railyard_server::{RailyardServer, Railyard};
use railyard::{HealthCheckRequest, HealthCheckResponse};

pub mod railyard {
    tonic::include_proto!("railyard");
}

#[derive(Debug, Default)]
pub struct RailyardNode {}

#[tonic::async_trait]
impl Railyard for RailyardNode {
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let reply = HealthCheckResponse {
            status: true,
            nonce: format!("Hello {}!", request.into_inner().nonce),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let node = RailyardNode::default();

    Server::builder()
        .add_service(RailyardServer::new(node))
        .serve(addr)
        .await?;

    Ok(())
}

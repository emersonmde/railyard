use tonic::{Request, Response, Status};

use crate::railyard::railyard;

use super::railyard::{ProduceMessageRequest, railyard_server::Railyard, ProduceMessageResponse, AppendEntriesRequest, AppendEntriesResponse};


pub struct RailyardService {
    // Here you might have fields for the actual state of the service,
    // like a handle to a database or a thread pool.
}

#[tonic::async_trait]
impl Railyard for RailyardService {
    async fn produce_message(
        &self,
        request: Request<ProduceMessageRequest>,
    )
-> Result<Response<ProduceMessageResponse>, Status> {
        println!("Received ProduceMessage request: {:?}", request);

        let response = railyard::ProduceMessageResponse {
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn append_entries(
        &self,
        request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        println!("Received AppendEntries request: {:?}", request);

        let response = railyard::AppendEntriesResponse {
            success: true,
        };

        Ok(Response::new(response))
    }

    // Implement the rest of the RPC methods in a similar way.
}

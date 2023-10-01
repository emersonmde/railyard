use tonic::{Request, Response, Status};

use crate::railyard::railyard_server::Railyard;
use crate::railyard::{
    CreateStreamRequest, CreateStreamResponse, GetRecordsRequest, GetRecordsResponse,
    GetShardIndexRequest, GetShardIndexResponse, PutRecordRequest, PutRecordResponse,
};

#[derive(Debug, Default)]
pub struct RailyardService {}

#[tonic::async_trait]
impl Railyard for RailyardService {
    async fn create_stream(
        &self,
        _request: Request<CreateStreamRequest>,
    ) -> Result<Response<CreateStreamResponse>, Status> {
        todo!()
    }

    async fn get_records(
        &self,
        _request: Request<GetRecordsRequest>,
    ) -> Result<Response<GetRecordsResponse>, Status> {
        todo!()
    }

    async fn put_record(
        &self,
        _request: Request<PutRecordRequest>,
    ) -> Result<Response<PutRecordResponse>, Status> {
        todo!()
    }

    async fn get_shard_iterator(
        &self,
        _request: Request<GetShardIndexRequest>,
    ) -> Result<Response<GetShardIndexResponse>, Status> {
        todo!()
    }
}

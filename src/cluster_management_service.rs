use tonic::{Request, Response, Status};
use crate::railyard::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, RequestVoteRequest, RequestVoteResponse};
use crate::railyard::cluster_management_server::ClusterManagement;

#[derive(Debug, Default)]

pub struct ClusterManagementService {}


#[tonic::async_trait]

impl ClusterManagement for ClusterManagementService {
    async fn append_entries(&self, _request: Request<AppendEntriesRequest>) -> Result<Response<AppendEntriesResponse>, Status> {
        todo!()
    }

    async fn request_vote(&self, _request: Request<RequestVoteRequest>) -> Result<Response<RequestVoteResponse>, Status> {
        todo!()
    }

    async fn install_snapshot(&self, _request: Request<InstallSnapshotRequest>) -> Result<Response<InstallSnapshotResponse>, Status> {
        todo!()
    }
}
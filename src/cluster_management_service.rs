use tonic::{Request, Response, Status};
use crate::railyard::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, RequestVoteRequest, RequestVoteResponse};
use crate::railyard::cluster_management_server::ClusterManagement;

pub struct ClusterManagementService {}


impl ClusterManagement for ClusterManagementService {
    async fn append_entries(&self, request: Request<AppendEntriesRequest>) -> Result<Response<AppendEntriesResponse>, Status> {
        todo!()
    }

    async fn request_vote(&self, request: Request<RequestVoteRequest>) -> Result<Response<RequestVoteResponse>, Status> {
        todo!()
    }

    async fn install_snapshot(&self, request: Request<InstallSnapshotRequest>) -> Result<Response<InstallSnapshotResponse>, Status> {
        todo!()
    }
}
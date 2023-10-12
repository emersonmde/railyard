use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tonic::{Request, Response, Status};

use crate::railyard::cluster_management_server::ClusterManagement;
use crate::railyard::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    RequestVoteRequest, RequestVoteResponse,
};

#[derive(Debug, PartialEq)]
enum NodeState {
    Follower,
    Candidate,
    _Leader,
}

#[derive(Debug)]
struct ClusterState {
    current_term: i32,
    node_state: NodeState,
    last_heartbeat: Instant,
    _voted_for: Option<String>,
}

#[derive(Debug)]
pub struct ClusterManagementService {
    cluster_state: Arc<Mutex<ClusterState>>,
}

impl ClusterManagementService {
    pub async fn new() -> Self {
        let service = Self {
            cluster_state: Arc::new(Mutex::new(ClusterState {
                current_term: 0,
                node_state: NodeState::Follower,
                last_heartbeat: Instant::now(),
                _voted_for: Option::None,
            })),
        };

        let service_clone = service.cluster_state.clone();
        tokio::spawn(async move {
            Self::election_timeout(service_clone).await;
        });

        service
    }

    async fn election_timeout(service: Arc<Mutex<ClusterState>>) {
        loop {
            let timeout = Duration::from_millis((rand::random::<u64>() % 150) + 150);
            sleep(timeout).await;

            let mut guard = service.lock().await;
            if guard.last_heartbeat.elapsed() >= timeout && guard.node_state != NodeState::Candidate
            {
                guard.node_state = NodeState::Candidate;
                guard.current_term += 1;

                println!(
                    "Election timeout triggered. Current term: {}",
                    guard.current_term
                );
            }
        }
    }
}

#[tonic::async_trait]
impl ClusterManagement for ClusterManagementService {
    async fn append_entries(
        &self,
        _request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        {
            let mut guard = self.cluster_state.lock().await;
            guard.last_heartbeat = Instant::now();
        }

        todo!("Implement append entries")
    }

    async fn request_vote(
        &self,
        _request: Request<RequestVoteRequest>,
    ) -> Result<Response<RequestVoteResponse>, Status> {
        todo!()
    }

    async fn install_snapshot(
        &self,
        _request: Request<InstallSnapshotRequest>,
    ) -> Result<Response<InstallSnapshotResponse>, Status> {
        todo!()
    }
}

use log::{debug, error, info, warn, Level};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::{sleep, timeout, Instant};
use tonic::transport::Channel;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::railyard::cluster_management_client::ClusterManagementClient;
use crate::railyard::cluster_management_server::ClusterManagement;
use crate::railyard::{
    AppendEntriesRequest, AppendEntriesResponse, Entry, InstallSnapshotRequest,
    InstallSnapshotResponse, RequestVoteRequest, RequestVoteResponse,
};

#[derive(Debug, PartialEq, Clone)]
enum NodeState {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug)]
struct ClusterState {
    id: String,
    current_term: u32,
    node_state: NodeState,
    last_heartbeat: Instant,
    voted_for: Option<String>,
    peers: Vec<String>,
    last_known_leader: Option<String>,
    _log: Vec<Entry>,
}

#[derive(Debug)]
pub struct ClusterManagementService {
    cluster_state: Arc<Mutex<ClusterState>>,
}

impl ClusterManagementService {
    pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1000);
    pub const ELECTION_TIMEOUT_BASE: u64 = 5000;
    pub const ELECTION_TIMEOUT_JITTER: u64 = 1500;
    pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(2);
    pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

    pub async fn new(peers: Vec<&String>) -> Self {
        let service = Self {
            cluster_state: Arc::new(Mutex::new(ClusterState {
                id: Uuid::new_v4().to_string(),
                current_term: 0,
                node_state: NodeState::Follower,
                last_heartbeat: Instant::now(),
                voted_for: Option::None,
                peers: peers.iter().cloned().cloned().collect(),
                last_known_leader: Option::None,
                _log: vec![],
            })),
        };

        let election_timeout_state = service.cluster_state.clone();
        tokio::spawn(async move {
            Self::election_timeout(election_timeout_state).await;
        });

        let heartbeat_state = service.cluster_state.clone();
        tokio::spawn(async move {
            Self::send_heartbeat(heartbeat_state).await;
        });

        service
    }

    fn log_with_id(id: &str, level: Level, message: &str) {
        match level {
            Level::Debug => debug!("{}: {}", id, message),
            Level::Info => info!("{}: {}", id, message),
            Level::Warn => warn!("{}: {}", id, message),
            Level::Error => error!("{}: {}", id, message),
            _ => {}
        }
    }

    // Send Empty AppendEntries to all peers
    /**
     * This is the main loop that is responsible for sending heartbeats to other nodes.
     * It will run in a separate thread and will be responsible for sending AppendEntries RPCs to all other nodes.
     * It will only send heartbeats if the node is the leader.
     */
    async fn send_heartbeat(service: Arc<Mutex<ClusterState>>) {
        loop {
            sleep(Self::HEARTBEAT_TIMEOUT).await;

            let mut state = service.lock().await;
            if state.node_state != NodeState::Leader {
                continue;
            }

            state.last_heartbeat = Instant::now();
            let id = state.id.clone();
            let current_term = state.current_term;
            let peers = state.peers.clone();
            drop(state);

            Self::log_with_id(&id, Level::Debug, "Sending heartbeat to peers");

            for peer in peers {
                let mut client: ClusterManagementClient<Channel>;
                let channel = Channel::builder(peer.clone().parse().unwrap())
                    .connect_timeout(Self::CONNECTION_TIMEOUT)
                    .connect()
                    .await;

                match channel {
                    Ok(ch) => client = ClusterManagementClient::new(ch),
                    Err(_) => {
                        Self::log_with_id(
                            &id,
                            Level::Error,
                            &format!("Failed to connect to peer: {}", &peer),
                        );
                        continue;
                    }
                }

                let request = Request::new(AppendEntriesRequest {
                    term: current_term,
                    leader_id: id.clone(),
                    prev_log_index: 0,
                    prev_log_term: 0,
                    entries: vec![],
                    leader_commit: 0,
                });

                Self::log_with_id(
                    &id,
                    Level::Debug,
                    &format!("Sending AppendEntries to peer: {:?}", &peer),
                );

                let response = timeout(Self::REQUEST_TIMEOUT, client.append_entries(request)).await;

                match response {
                    Ok(Ok(response)) => {
                        let response = response.into_inner();
                        Self::log_with_id(
                            &id,
                            Level::Debug,
                            &format!("AppendEntries response from {}: {:?}", &peer, response),
                        );
                    }
                    Ok(Err(_)) | Err(_) => {
                        Self::log_with_id(
                            &id,
                            Level::Error,
                            &format!("Failed to send AppendEntries to peer: {}", &peer),
                        );
                    }
                }
            }
        }
    }

    /**
     * This is the main loop that is responsible for triggering elections.
     * It will run in a separate thread and will be responsible for transitioning
     * the node to a candidate state and sending RequestVote RPCs to all other nodes.
     * If the candidate receives a majority of votes, it will transition to leader.
     */
    async fn election_timeout(cluster_state_mutex: Arc<Mutex<ClusterState>>) {
        loop {
            let timeout_duration = Duration::from_millis(
                (rand::random::<u64>() % Self::ELECTION_TIMEOUT_JITTER)
                    + Self::ELECTION_TIMEOUT_BASE,
            );
            sleep(timeout_duration).await;

            let mut cluster_state = cluster_state_mutex.lock().await;

            if cluster_state.last_heartbeat.elapsed() >= timeout_duration
                && cluster_state.node_state != NodeState::Candidate
            {
                cluster_state.node_state = NodeState::Candidate;
                cluster_state.current_term += 1;

                let mut votes = 0;
                let current_term = cluster_state.current_term;
                let id = cluster_state.id.clone();
                let voted_for = cluster_state.voted_for.clone();
                let peers = cluster_state.peers.clone();
                let num_peers = peers.len();
                drop(cluster_state);

                Self::log_with_id(
                    &id,
                    Level::Info,
                    &format!(
                        "Election timeout triggered, current term: {}, peers: {:?}",
                        current_term, peers
                    ),
                );
                for peer in peers {
                    Self::log_with_id(
                        &id,
                        Level::Debug,
                        &format!("Sending request vote to peer: {:?}", peer.clone()),
                    );
                    let mut client: ClusterManagementClient<Channel>;
                    let channel = Channel::builder(peer.clone().parse().unwrap())
                        .connect_timeout(Self::CONNECTION_TIMEOUT)
                        .connect()
                        .await;

                    match channel {
                        Ok(ch) => client = ClusterManagementClient::new(ch),
                        Err(_) => {
                            Self::log_with_id(
                                &id,
                                Level::Error,
                                &format!("Failed to connect to peer: {}", &peer),
                            );
                            continue;
                        }
                    }

                    let request = Request::new(RequestVoteRequest {
                        term: current_term,
                        candidate_id: id.clone(),
                        last_log_index: 0,
                        last_log_term: 0,
                    });

                    let request_vote_result =
                        timeout(Self::REQUEST_TIMEOUT, client.request_vote(request)).await;

                    match request_vote_result {
                        Ok(Ok(response)) => {
                            let response = response.into_inner();
                            let vote_status = if response.vote_granted {
                                "granted"
                            } else {
                                "denied"
                            };
                            Self::log_with_id(
                                &id,
                                Level::Info,
                                &format!("RequestVote {} form {}", vote_status, &peer),
                            );
                            if response.vote_granted {
                                votes += 1;
                            }
                        }
                        Ok(Err(_)) => {
                            Self::log_with_id(
                                &id,
                                Level::Error,
                                &format!("Failed to send request vote to peer: {}", &peer),
                            );
                        }
                        Err(_) => {
                            Self::log_with_id(
                                &id,
                                Level::Error,
                                &format!("Request vote to peer: {} timed out", &peer),
                            );
                        }
                    }
                }

                if votes >= num_peers / 2 {
                    Self::log_with_id(
                        &id,
                        Level::Info,
                        &format!(
                            "Received majority of votes, transitioning to leader for term {}",
                            current_term
                        ),
                    );
                    let mut leader_cluster_state = cluster_state_mutex.lock().await;
                    leader_cluster_state.node_state = NodeState::Leader;
                    drop(leader_cluster_state)
                } else {
                    Self::log_with_id(
                        &id,
                        Level::Info,
                        "Received less than majority of votes, transitioning to follower",
                    );
                    let mut cluster_state = cluster_state_mutex.lock().await;
                    cluster_state.node_state = NodeState::Follower;
                    cluster_state.voted_for = voted_for;
                }
            }
        }
    }
}

#[tonic::async_trait]
impl ClusterManagement for ClusterManagementService {
    /**
     * This is the RPC that is called by the leader to replicate log entries to other nodes.
     * The leader will send this RPC to all other nodes in the cluster.
     *   1. Reply false if term < currentTerm (§5.1)
     *   2. Reply false if log doesn’t contain an entry at prevLogIndex
     *   whose term matches prevLogTerm (§5.3)
     *   3. If an existing entry conflicts with a new one (same index
     *   but different terms), delete the existing entry and all that
     *   follow it (§5.3)
     *   4. Append any new entries not already in the log
     *   5. If leaderCommit > commitIndex, set commitIndex =
     *   min(leaderCommit, index of last new entry)
     */
    async fn append_entries(
        &self,
        request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        let request = request.into_inner();

        let mut state = self.cluster_state.lock().await;
        let id = state.id.clone();

        state.last_heartbeat = Instant::now();

        if request.term > state.current_term {
            state.current_term = request.term;
            state.node_state = NodeState::Follower;
            state.last_known_leader = Some(request.leader_id.clone());
            state.voted_for = None;
        }

        Self::log_with_id(
            &id,
            Level::Debug,
            &format!(
                "Received AppendEntries from leader: {:?}",
                request.leader_id
            ),
        );
        Ok(Response::new(AppendEntriesResponse {
            term: state.current_term,
            success: false,
        }))
    }

    /**
     * This is the RPC that is called by a candidate to request votes from other nodes.
     * A candidate will send this RPC to all other nodes in the cluster.
     * The candidate will then transition to leader if it receives votes from a majority of nodes.
     *   1. Reply false if term < currentTerm (§5.1)
     *   2. If votedFor is null or candidateId, and candidate’s log is at
     *   least as up-to-date as receiver’s log, grant vote
     */
    async fn request_vote(
        &self,
        request: Request<RequestVoteRequest>,
    ) -> Result<Response<RequestVoteResponse>, Status> {
        let mut state = self.cluster_state.lock().await;
        let request = request.into_inner();
        // TODO: Check log is up to date before granting vote
        if request.term > state.current_term
            && state
                .voted_for
                .clone()
                .map_or(true, |v| v == request.candidate_id)
        {
            println!(
                "{} Voting for candidate: {} in term: {}",
                state.id, request.candidate_id, request.term
            );
            state.voted_for = Some(request.candidate_id);
            state.current_term = request.term;
            state.node_state = NodeState::Follower;
            state.last_heartbeat = Instant::now();

            return Ok(Response::new(RequestVoteResponse {
                term: state.current_term,
                vote_granted: true,
            }));
        }

        return Ok(Response::new(RequestVoteResponse {
            term: state.current_term,
            vote_granted: false,
        }));
    }

    async fn install_snapshot(
        &self,
        _request: Request<InstallSnapshotRequest>,
    ) -> Result<Response<InstallSnapshotResponse>, Status> {
        todo!("Implement install_snapshot")
    }
}

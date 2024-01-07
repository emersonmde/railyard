use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Error, Result};
use log::{debug, error, info, warn, Level};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout, Instant};
use tonic::transport::Channel;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::railyard::railyard_client::RailyardClient;
use crate::railyard::railyard_server::Railyard;
use crate::railyard::{
    AppendEntriesRequest, AppendEntriesResponse, CreateStreamRequest, CreateStreamResponse, Entry,
    GetIteratorIndexRequest, GetIteratorIndexResponse, GetRecordsRequest, GetRecordsResponse,
    InstallSnapshotRequest, InstallSnapshotResponse, PutRecordRequest, PutRecordResponse,
    RequestVoteRequest, RequestVoteResponse,
};

#[derive(Debug, PartialEq, Clone)]
enum NodeState {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, PartialEq, Clone)]
struct Peer {
    id: String,
    address: String,
    last_log_index: u64,
}

#[derive(Debug)]
struct ClusterState {
    id: String,
    current_term: u64,
    node_state: NodeState,
    last_heartbeat: Instant,
    voted_for: Option<String>,
    peers: Vec<Peer>,
    last_known_leader: Option<String>,
    log: Vec<Entry>,
    commit_index: u64,
}

#[derive(Debug)]
pub struct RailyardService {
    cluster_state: Arc<Mutex<ClusterState>>,
}

impl RailyardService {
    pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1000);
    pub const ELECTION_TIMEOUT_BASE: u64 = 5000;
    pub const ELECTION_TIMEOUT_JITTER: u64 = 1500;
    pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(2);
    pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

    pub async fn new(peers: Vec<&String>) -> Self {
        let peers: Vec<Peer> = peers
            .iter()
            .cloned()
            .map(|peer| Peer {
                id: "".to_string(),
                address: peer.clone(),
                last_log_index: 0,
            })
            .collect();

        let service = Self {
            cluster_state: Arc::new(Mutex::new(ClusterState {
                id: Uuid::new_v4().to_string(),
                current_term: 0,
                node_state: NodeState::Follower,
                last_heartbeat: Instant::now(),
                voted_for: None,
                peers,
                last_known_leader: None,
                log: vec![Entry {
                    index: 0,
                    term: 0,
                    command: Vec::from("Starting cluster".to_string()),
                }],
                commit_index: 0,
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

    pub async fn new_with_data(peers: Vec<&String>) -> Self {
        let peers: Vec<Peer> = peers
            .iter()
            .cloned()
            .map(|peer| Peer {
                id: "".to_string(),
                address: peer.clone(),
                last_log_index: 0,
            })
            .collect();

        let service = Self {
            cluster_state: Arc::new(Mutex::new(ClusterState {
                id: Uuid::new_v4().to_string(),
                current_term: 4,
                node_state: NodeState::Follower,
                last_heartbeat: Instant::now(),
                voted_for: None,
                peers,
                last_known_leader: None,
                log: vec![
                    Entry {
                        index: 0,
                        term: 0,
                        command: Vec::from("Starting cluster".to_string()),
                    },
                    Entry {
                        index: 1,
                        term: 4,
                        command: Vec::from("foo".to_string()),
                    },
                    Entry {
                        index: 2,
                        term: 4,
                        command: Vec::from("foo".to_string()),
                    },
                ],
                commit_index: 2,
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
            let peers = state.peers.clone();
            let current_term = state.current_term;
            let commit_index = state.commit_index;

            let current_log_index = state.log.len().saturating_sub(1) as u64;
            let current_log_term = state
                .log
                .get(current_log_index as usize)
                .map_or(0, |entry| entry.term);
            drop(state);

            Self::log_with_id(&id, Level::Debug, "Sending heartbeat to peers");

            for peer in peers {
                let response = Self::send_append_entries(
                    &id,
                    current_term,
                    current_log_index,
                    current_log_term,
                    commit_index,
                    &peer,
                    &[],
                )
                .await;

                match response {
                    Ok(response) => {
                        if response.term > current_term {
                            let mut state = service.lock().await;
                            state.current_term = response.term;
                            state.node_state = NodeState::Follower;
                            state.last_known_leader = None;
                            state.voted_for = None;
                            return;
                        }

                        if response.success {
                            Self::log_with_id(
                                &id,
                                Level::Debug,
                                &format!(
                                    "Peer {} responded to AppendEntries with success during heartbeat",
                                    &peer.address
                                )
                            );
                        } else {
                            Self::log_with_id(
                                &id,
                                Level::Error,
                                &format!(
                                    "Peer {} responded to AppendEntries with failure during heartbeat, syncing log",
                                    &peer.address
                                ),
                            );
                            match Self::sync_follower_log(&id, &peer, service.clone()).await {
                                Ok(_) => {
                                    Self::log_with_id(
                                        &id,
                                        Level::Info,
                                        &format!(
                                            "Successfully synced follower log for peer: {}",
                                            &peer.address
                                        ),
                                    );
                                }
                                Err(error) => {
                                    Self::log_with_id(
                                        &id,
                                        Level::Error,
                                        &format!(
                                            "Failed to sync follower log for peer: {} with error: {}",
                                            &peer.address, error
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    Err(_) => {
                        Self::log_with_id(
                            &id,
                            Level::Error,
                            &format!("Failed to send heartbeat to peer: {}", &peer.address),
                        );
                    }
                }
            }
        }
    }

    /**
     * Syncs missing log entries from the leader to the follower
     *
     * Starts by setting the index to send to be the latest log index then loops through the log sending a slice
     * of entries, including the previous entry to the entries already tried until the follower responds with
     * success.
     */
    async fn sync_follower_log(
        id: &str,
        peer: &Peer,
        cluster_state: Arc<Mutex<ClusterState>>,
    ) -> Result<()> {
        let mut state = cluster_state.lock().await;
        if state.log.is_empty() {
            return Ok(());
        }

        let current_log_index = state.log.len().saturating_sub(1) as u64;
        let mut start_index = current_log_index;
        let mut prev_log_index = start_index.saturating_sub(1);
        let mut prev_log_term = state
            .log
            .get(prev_log_index as usize)
            .map_or(0, |entry| entry.term);

        loop {
            let response = Self::send_append_entries(
                id,
                state.current_term,
                prev_log_index,
                prev_log_term,
                state.commit_index,
                peer,
                &state.log[start_index as usize..=current_log_index as usize],
            )
            .await;

            match response {
                Ok(response) => {
                    let peer = state
                        .peers
                        .iter_mut()
                        .find(|p| p.address == peer.address)
                        .unwrap();
                    peer.last_log_index = current_log_index;
                    if response.success {
                        return Ok(());
                    }

                    // if start index is 0, we've reached the beginning of the log and can't go any further
                    if start_index == 0 {
                        return Err(Error::msg("Failed to sync follower log, start index is 0"));
                    }

                    start_index = start_index.saturating_sub(1);
                    prev_log_index = start_index.saturating_sub(1);
                    prev_log_term = state
                        .log
                        .get(prev_log_index as usize)
                        .map_or(0, |entry| entry.term);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn send_append_entries(
        id: &str,
        current_term: u64,
        prev_log_index: u64,
        prev_log_term: u64,
        commit_index: u64,
        peer: &Peer,
        entries: &[Entry],
    ) -> Result<AppendEntriesResponse> {
        // TODO: Reuse client
        let mut client = Self::create_client(id, &peer.address)
            .await
            .with_context(|| format!("Failed to create client for peer: {}", &peer.address))?;

        let request = Request::new(AppendEntriesRequest {
            term: current_term,
            leader_id: id.to_string(),
            prev_log_index,
            prev_log_term,
            leader_commit: commit_index,
            entries: entries.to_vec(),
        });

        let response = timeout(Self::REQUEST_TIMEOUT, client.append_entries(request))
            .await
            .with_context(|| {
                format!(
                    "Timeout occurred during append entries to peer: {}",
                    &peer.address
                )
            })??;

        Ok(response.into_inner())
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

                let last_log_index = cluster_state.log.len().saturating_sub(1) as u64;
                let last_log_term = cluster_state
                    .log
                    .get(last_log_index as usize)
                    .map_or(0, |entry| entry.term);
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

                    let mut client = match Self::create_client(&id, &peer.address).await {
                        Some(value) => value,
                        None => continue,
                    };

                    let request = Request::new(RequestVoteRequest {
                        term: current_term,
                        candidate_id: id.clone(),
                        last_log_index,
                        last_log_term,
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
                                &format!("RequestVote {} form {}", vote_status, &peer.address),
                            );
                            if response.vote_granted {
                                votes += 1;
                            }
                        }
                        Ok(Err(_)) => {
                            Self::log_with_id(
                                &id,
                                Level::Error,
                                &format!("Failed to send request vote to peer: {}", &peer.address),
                            );
                        }
                        Err(_) => {
                            Self::log_with_id(
                                &id,
                                Level::Error,
                                &format!("Request vote to peer: {} timed out", &peer.address),
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

    async fn create_client(id: &str, peer: &String) -> Option<RailyardClient<Channel>> {
        let client: RailyardClient<Channel>;
        let channel = Channel::builder(peer.clone().parse().unwrap())
            .connect_timeout(Self::CONNECTION_TIMEOUT)
            .connect()
            .await;

        match channel {
            Ok(ch) => client = RailyardClient::new(ch),
            Err(_) => {
                Self::log_with_id(
                    id,
                    Level::Error,
                    &format!("Failed to connect to peer: {}", &peer),
                );
                return None;
            }
        }
        Some(client)
    }
}

#[tonic::async_trait]
impl Railyard for RailyardService {
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

        // Log append entry request from leader along with current cluster state
        Self::log_with_id(
            &id,
            Level::Debug,
            &format!(
                "Received AppendEntries from leader with term: {}, prev_log_index: {}, prev_log_term: {}, \
                leader_commit: {}, entries: {:?}, current_term: {}, current_log_index: {}, committed_index: {}",
                request.term,
                request.prev_log_index,
                request.prev_log_term,
                request.leader_commit,
                request.entries,
                state.current_term,
                state.log.len().saturating_sub(1),
                state.commit_index
            ),
        );

        // Reply false if term < currentTerm
        if request.term < state.current_term {
            Self::log_with_id(
                &id,
                Level::Debug,
                &format!(
                    "Received AppendEntries from leader: {:?} with term: {} less than current term: {}",
                    request.leader_id, request.term, state.current_term
                ),
            );
            return Ok(Response::new(AppendEntriesResponse {
                term: state.current_term,
                success: false,
            }));
        }

        // Reply false if log doesn’t contain an entry at prevLogIndex
        if request.prev_log_index > state.log.len().saturating_sub(1) as u64 {
            Self::log_with_id(
                &id,
                Level::Debug,
                &format!(
                    "Received AppendEntries from leader: {:?} with prev_log_index: {} greater than log length: {}",
                    request.leader_id, request.prev_log_index, state.log.len()
                ),
            );
            return Ok(Response::new(AppendEntriesResponse {
                term: state.current_term,
                success: false,
            }));
        }

        // If an existing entry conflicts with a new one (same index but different terms), delete the existing entry
        // from the log and all entries that follow it
        if request.prev_log_index > 0 {
            let prev_log_entry = state.log.get(request.prev_log_index as usize).unwrap();
            if prev_log_entry.term != request.prev_log_term {
                Self::log_with_id(
                    &id,
                    Level::Debug,
                    &format!(
                        "Received AppendEntries from leader: {:?} with prev_log_term: {} not matching log term: {}",
                        request.leader_id, request.prev_log_term, prev_log_entry.term
                    ),
                );
                state.log.truncate(request.prev_log_index as usize);
            }
        }

        // Append any new entries not already in the log
        state.log.extend(request.entries);

        // If leaderCommit > commitIndex, set commitIndex = min(leaderCommit, index of last new entry)
        if request.leader_commit > state.commit_index {
            // TODO: Commit all commands found in entires between commit_index and leader_commit
            state.commit_index = std::cmp::min(request.leader_commit, state.log.len() as u64);
        }

        Ok(Response::new(AppendEntriesResponse {
            term: state.current_term,
            success: true,
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
        let id = state.id.clone();

        if request.term >= state.current_term
            && state
                .voted_for
                .clone()
                .map_or(true, |v| v == request.candidate_id)
            && request.last_log_index >= state.log.len() as u64
        {
            Self::log_with_id(
                &id,
                Level::Debug,
                &format!(
                    "Received RequestVote from candidate: {:?} with term: {} greater than current term: {}, granting vote",
                    request.candidate_id, request.term, state.current_term
                ),
            );
            state.voted_for = Some(request.candidate_id.clone());
            state.current_term = request.term;
            state.node_state = NodeState::Follower;
            state.last_heartbeat = Instant::now();
            state.last_known_leader = Some(request.candidate_id.clone());

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

    async fn get_iterator_index(
        &self,
        _request: Request<GetIteratorIndexRequest>,
    ) -> Result<Response<GetIteratorIndexResponse>, Status> {
        todo!()
    }
}

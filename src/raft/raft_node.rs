use std::sync::{Arc, Mutex};
use std::time::Duration;
use rand::Rng;
use tokio::sync::mpsc;
use crate::raft::raft_state::{NodeState, RaftState};
use crate::grpc::raft_grpc_server::{RaftGrpcRequest, RaftGrpcResponse};
use crate::raft_grpc::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse, TimeoutNowResponse};
use crate::configuration::config::{MembershipConfigurations, NodeConfig};
use crate::data_store::sled_log::LogStore;
use crate::data_store::sled_state::StateStore;
use crate::error::error::RaftError;
use crate::raft::election;
use crate::raft::election::ElectionResult;
use crate::Utils::thread_safe_bool::ThreadSafeBool;

pub struct RaftServer{
    node_id: String,
    node_state: Arc<Mutex<NodeState>>,
    raft_state: Arc<RaftState>,

    log_store: Arc<dyn LogStore>,

    config: NodeConfig,
    membership_configurations: MembershipConfigurations,


    /// Channel to receive incoming Raft RPCs.
    rpc_rx: mpsc::Receiver<(RaftGrpcRequest, mpsc::Sender<RaftGrpcResponse>)>,
    /// Channels to signal stopping server background task.
    shutdown_tx_rx: (mpsc::Sender<()>, mpsc::Receiver<()>),

    running: ThreadSafeBool,
}

impl RaftServer {
    pub async fn new(
        config: NodeConfig,
        membership_configurations: MembershipConfigurations,
        rpc_rx: mpsc::Receiver<(RaftGrpcRequest, mpsc::Sender<RaftGrpcResponse>)>,
        state_store: Box<dyn StateStore>,
        log_store: Arc<dyn LogStore>,
    ) -> Self {
        let raft_state = Arc::new(RaftState::new(state_store));
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let server = Self {
            node_id: config.server_id.clone(),
            node_state: Arc::new(Mutex::new(NodeState::Follower)),
            raft_state,
            log_store,
            config,
            membership_configurations,
            rpc_rx,
            shutdown_tx_rx: (shutdown_tx, shutdown_rx),
            running: ThreadSafeBool::new(false),
        };
        server
    }

    /*
    =================
    RAFT SERVER LOOP
    =================

    States: Follower (default), Candidate (during election), Leader (manages replication).

    Transitions:
    - Follower -> Candidate: Election timeout (no leader heartbeat) → initiates vote request.
    - Candidate -> Leader: Gains majority votes.
    - Candidate -> Follower: Detects existing leader or higher term.
    - Leader -> Follower: Higher term detected (steps down).
    - Any state -> Stopped: Shutdown (e.g., sigterm, removal).

    Diagram:

                   Election Timeout
    Initial      Initiates Election
      |                 .----.
      |                 |    |
      v   Timeout       |    v     Majority Votes
    Follower --------> Candidate ----------> Leader
      ^ ^                 |                      |
      | | Detect Leader/  |                      |
      | | Higher Term     |                      |
      | '-----------------'                      |
      |                                          |
      '------------------ Higher Term Detected --'
                             Stopped
    */

    async fn start(&mut self){
        log::info!("Starting loop start {}", self.node_id);
        self.running.set(true);

        while self.running.get() {
            let state = self.get_state();
            match state {
                Ok(current_state) => {
                    match current_state {
                        Some(NodeState::Follower) => self.run_follower_loop().await,
                        Some(NodeState::Candidate) => self.run_candidate_loop().await,
                        Some(NodeState::Leader) => self.run_leader_loop().await,
                        Some(NodeState::Stopped) => self.stop().await,
                        _ => {},
                    }
                }
                Err(err) => {
                    log::error!("server.loop.end: {:?}", err);
                    self.running.set(false);
                }
            }
        }
    }
    pub async fn run(&mut self){
        if let Ok(Some(state)) = self.get_state() {
            self.become_follower();
            log::info!("server.start.state: {:?}", state);

            self.start().await;
        } else {
            log::error!("Failed to start raft server");
        };
    }

    async fn stop(&mut self){
        // _ = self.fsm_shutdown_tx.send(()).await;
        self.running.set(false);
        log::info!("server.loop.stop");
    }

    async fn run_follower_loop(&mut self){
        log::info!("Run Follower Loop");
        let mut election_timeout = self.random_election_timeout();

        loop {
            let state = self.get_state();
            match state {
                Ok(current_state) => {
                    if current_state != Some(NodeState::Follower) {
                        break;
                    }

                    tokio::select! {
                        _ = tokio::time::sleep(election_timeout) => {
                            log::info!("Election time-out. Node {} become candidate", self.node_id);
                            self.become_candidate();
                            election_timeout = self.random_election_timeout();
                        },

                        rpc_event = self.rpc_rx.recv() => {
                            self.handle_rpc_event(rpc_event).await
                        },

                        _ = self.shutdown_tx_rx.1.recv() => {
                            log::info!("Stopping the follower loop");
                            self.set_state(NodeState::Stopped);
                            return;
                        },
                    }
                }
                Err(e) => log::error!("Unable to get current state: {:?}", e)
            }
        }
    }

    async fn run_candidate_loop(&mut self){
        log::info!("Run Candidate Loop");

        let new_term = self.get_current_term() + 1;
        log::info!("total votes ({:?}) needed for election in this term {:?}", self.quorum_size(), new_term);

        // TODO: Reset heartbeat flag
        self.set_current_term(new_term);
        let nodes = self.membership_configurations.nodes.clone();

        loop {
            let state = self.get_state();
            match state{
                Ok(current_state) => {
                    if current_state != Some(NodeState::Candidate){
                        break;
                    }

                    let request = RequestVoteRequest {
                        term: new_term,
                        candidate_id: self.config.server_id.clone(),
                        last_log_index: self.get_last_log_index().await,
                        last_log_term: self.get_last_log_term().await,
                        commit_index: self.get_commit_index(),
                    };
                    log::info!("Sending RequestVoteRequest {:?}", request);

                    let election_result = election::grant_vote(&nodes, &self.config.server_id, request, self.quorum_size() as usize).await;
                    match election_result {
                        ElectionResult::Won => self.become_leader(),
                        ElectionResult::StepDown => self.become_follower(),
                        ElectionResult::Lost => {
                            log::info!("server.election.lost");
                        }
                    }
                }
                Err(e) => log::error!("Unable to get current state: {:?}", e)
            }
        }
    }

    async fn run_leader_loop(&mut self){
        log::info!("Run Leader Loop");

        self.run_periodic_heartbeat();

        loop{
            let state = self.get_state();
            match state {
                Ok(current_state) => {
                    if current_state != Some(NodeState::Leader){
                        break;
                    }

                    tokio::select! {
                        rpc_event = self.rpc_rx.recv() => {
                            self.handle_rpc_event(rpc_event).await
                        },

                        _ = self.shutdown_tx_rx.1.recv() => {
                            log::info!("Stopping the follower loop");
                            self.set_state(NodeState::Stopped);
                            return;
                        }
                    }
                }
                Err(e) => log::error!("Unable to get current state: {:?}", e)
            }
        }
    }
    async fn handle_rpc_event(&mut self, rpc_event: Option<(RaftGrpcRequest, mpsc::Sender<RaftGrpcResponse>)>) {
        match rpc_event {
            Some((request, response_sender)) => {
                let result = match request {
                    RaftGrpcRequest::AppendEntries(args) => {
                        let response = self.append_entries(args).await;
                        response_sender.send(RaftGrpcResponse::AppendEntries(response)).await
                    },
                    RaftGrpcRequest::RequestVote(args) => {
                        let response = self.request_vote(args).await;
                        response_sender.send(RaftGrpcResponse::RequestVote(response)).await
                    },
                    RaftGrpcRequest::TimeoutNow(_) => {
                        let response = self.timeout_now().await;
                        response_sender.send(RaftGrpcResponse::TimeoutNow(response)).await
                    },
                };
                if let Err(e) = result {
                    log::error!("Unable to process request: {}", e);
                }
                self.random_election_timeout();
            },
            None => {
                log::error!("Channel was closed, restarting listener");
            }
        }
    }

    async fn append_entries(&mut self, args: AppendEntriesRequest) -> AppendEntriesResponse {}
    async fn request_vote(&mut self, args: RequestVoteRequest) -> RequestVoteResponse {}
    async fn timeout_now(&mut self)-> TimeoutNowResponse {
        TimeoutNowResponse {
            term: self.get_current_term(),
            success: true,
        }
    }
    fn run_periodic_heartbeat(&mut self){}
}

impl RaftServer {
    fn random_election_timeout(&mut self) -> Duration {
        rand::rng().random_range(Duration::from_secs(self.config.election_timeout_min)..Duration::from_secs(self.config.election_timeout_max))
    }
    fn get_state(&self) -> Result<Option<NodeState>, RaftError> {
        let locked_state = self.node_state.lock();
        match locked_state {
            Ok(result) => {
                let state = match *result {
                    NodeState::Stopped => NodeState::Stopped,
                    NodeState::Follower => NodeState::Follower,
                    NodeState::Candidate => NodeState::Candidate,
                    NodeState::Leader => NodeState::Leader,
                };
                Ok(Some(state))
            },
            Err(e) => {
                log::error!("unable to unlock node state: {}", e);
                Err(RaftError::UnableToUnlockNodeState)
            }
        }
    }
    fn set_state(&mut self, new_state: NodeState) {
        let mut current_state = self.node_state.lock().unwrap();
        *current_state = new_state;
    }
    fn become_follower(&mut self){
        self.set_state(NodeState::Follower);
        log::info!("Node {} became the follower", self.node_id);
    }
    fn become_leader(&mut self){
        self.set_state(NodeState::Leader);
        log::info!("Node {} became the leader", self.node_id);
    }
    fn become_candidate(&mut self) {
        self.set_state(NodeState::Candidate);
        log::info!("Node {} became the candidate", self.node_id);
    }
    fn quorum_size(&self) -> u64 {
        // TODO: Introduce new configuration for all membership
        return 0;
    }
    fn get_current_term(&self) -> u64 {
        self.raft_state.get_current_term()
    }
    fn set_current_term(&self, value: u64) {
        self.raft_state.set_current_term(value);
    }
    fn get_commit_index(&self) -> u64 {
        self.raft_state.get_commit_index()
    }
    fn set_commit_index(&self, value: u64) {
        self.raft_state.set_commit_index(value);
    }
    fn get_last_applied(&self) -> u64 {
        self.raft_state.get_last_applied()
    }
    fn set_last_applied(&self, value: u64) {
        self.raft_state.set_last_applied(value);
    }
    fn get_votes(&self) -> u64 {
        self.raft_state.get_votes()
    }
    fn set_votes(&self, value: u64) {
        self.raft_state.set_votes(value);
    }
    fn get_voted_for(&self) -> Option<String> {
        self.raft_state.get_voted_for()
    }
    fn set_voted_for(&self, node_id: Option<String>) {
        self.raft_state.set_voted_for(node_id);
    }
    async fn get_last_log_term(&self) -> u64 { self.log_stor    }
    async fn get_last_log_index(&self) -> u64  {
        self.logs.read().await.last_index()
    }
}
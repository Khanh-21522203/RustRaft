use std::sync::Mutex;
use crate::data_store::sled_state::StateStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
    Stopped,
}

pub struct RaftStateKey;

impl RaftStateKey {
    pub const CURRENT_TERM: &'static str = "current_term";
    pub const COMMIT_INDEX: &'static str = "commit_index";
    pub const LAST_APPLIED: &'static str = "last_applied";
    pub const PREV_VOTED_FOR: &'static str = "prev_voted_for";
    pub const PREV_VOTE_TERM: &'static str = "prev_vote_term";
}

pub struct RaftState {
    pub votes_granted: Mutex<u64>,
    pub voted_for: Mutex<Option<String>>,
    pub current_term: Mutex<u64>,
    pub commit_index: Mutex<u64>,
    pub last_applied: Mutex<u64>,

    state_store: Box<dyn StateStore>,
}

impl RaftState {
    pub fn new(state_store: Box<dyn StateStore>) -> Self {
        let commit_index = state_store.get(RaftStateKey::COMMIT_INDEX)
            .ok()
            .flatten()
            .unwrap_or(0);

        let current_term = state_store.get(RaftStateKey::CURRENT_TERM)
            .ok()
            .flatten()
            .unwrap_or(0);

        let last_applied = state_store.get(RaftStateKey::LAST_APPLIED)
            .ok()
            .flatten()
            .unwrap_or(0);



        let raft_state = Self {
            votes_granted: Mutex::new(0),
            voted_for: Mutex::new(None),
            current_term: Mutex::new(current_term),
            commit_index: Mutex::new(commit_index),
            last_applied: Mutex::new(last_applied),
            state_store,
        };

        raft_state
    }

    pub fn get_current_term(&self) -> u64 {
        let current_term = self.current_term.lock().unwrap();
        *current_term
    }

    pub fn set_current_term(&self, value: u64) {
        if let Err(e) = self.state_store.set(RaftStateKey::CURRENT_TERM, value){
            log::error!("unable to persist current term to disk: {:?}", e)
        };

        let mut current_term = self.current_term.lock().unwrap();
        *current_term = value;
    }

    pub fn get_commit_index(&self) -> u64 {
        let commit_index = self.commit_index.lock().unwrap();
        *commit_index
    }

    pub fn set_commit_index(&self, value: u64) {
        if let Err(e) = self.state_store.set(RaftStateKey::COMMIT_INDEX, value){
            log::error!("unable to persist commit index to disk: {:?}", e)
        };

        let mut commit_index = self.commit_index.lock().unwrap();
        *commit_index = value;
    }

    pub fn get_last_applied(&self) -> u64 {
        let last_applied = self.last_applied.lock().unwrap();
        *last_applied
    }
    pub fn set_last_applied(&self, value: u64) {
        if let Err(e) = self.state_store.set(RaftStateKey::LAST_APPLIED, value){
            log::error!("unable to persist last applied to disk: {:?}", e)
        };

        let mut last_applied = self.last_applied.lock().unwrap();
        *last_applied = value;
    }

    pub fn get_votes(&self) -> u64 {
        let votes = self.votes_granted.lock().unwrap();
        *votes
    }
    pub fn set_votes(&self, value: u64) {
        let mut votes = self.votes_granted.lock().unwrap();
        *votes = value;
    }

    pub fn get_voted_for(&self) -> Option<String> {
        let voted_for = self.voted_for.lock().unwrap();
        voted_for.clone()
    }
    pub fn set_voted_for(&self, node_id: Option<String>) {
        if let Err(e) = self.state_store.set_str(RaftStateKey::PREV_VOTED_FOR, node_id.clone().unwrap_or("".to_string())){
            log::error!("unable to persist voted for to disk: {:?}", e)
        };

        let mut voted_for = self.voted_for.lock().unwrap();
        *voted_for = node_id;
    }

    pub fn last_voted_for(&self)->  Option<String> {
        let last_voted_for  = self.state_store.get_str(RaftStateKey::PREV_VOTED_FOR);
        if let Ok(Some(voted_for)) = last_voted_for  {
            if voted_for == "" {
                return None
            }
            return Some(voted_for);
        }
        None
    }
    pub fn last_vote_term(&self) -> u64{
        let last_vote_term  = self.state_store.get(RaftStateKey::PREV_VOTE_TERM);
        if let Ok(Some(idx)) = last_vote_term  {
            return idx;
        }
        0
    }

    pub fn set_vote_term(&self, term: u64) {
        if let Err(e) = self.state_store.set(RaftStateKey::PREV_VOTE_TERM, term){
            log::error!("unable to persist last vote term to disk: {:?}", e)
        };
    }
}
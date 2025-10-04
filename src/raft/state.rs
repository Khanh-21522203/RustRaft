use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
    Stopped,
}

pub struct RaftState {
    pub votes: Mutex<u64>,
    pub voted_for: Mutex<Option<String>>,
    pub current_term: Mutex<u64>,
    pub commit_index: Mutex<u64>,
    pub last_applied: Mutex<u64>,

    // Storage
}

impl RaftState {
    pub fn new() -> Self {
        let raft_state = Self {
            votes: Mutex::new(0),
            voted_for: Mutex::new(None),
            current_term: Mutex::new(0),
            commit_index: Mutex::new(0),
            last_applied: Mutex::new(0),
        };

        raft_state
    }

    pub fn get_current_term(&self) -> u64 {
        let current_term = self.current_term.lock().unwrap();
        *current_term
    }

    pub fn set_current_term(&self, value: u64) {
        // Persistence storage
        let mut current_term = self.current_term.lock().unwrap();
        *current_term = value;
    }

    pub fn get_commit_index(&self) -> u64 {
        let commit_index = self.commit_index.lock().unwrap();
        *commit_index
    }

    pub fn set_commit_index(&self, value: u64) {
        // Persistence storage
        let mut commit_index = self.commit_index.lock().unwrap();
        *commit_index = value;
    }

    pub fn get_last_applied(&self) -> u64 {
        let last_applied = self.last_applied.lock().unwrap();
        *last_applied
    }

    pub fn set_last_applied(&self, value: u64) {
        // Persistence storage
        let mut last_applied = self.last_applied.lock().unwrap();
        *last_applied = value;
    }

    pub fn get_votes(&self) -> u64 {
        let votes = self.votes.lock().unwrap();
        *votes
    }
    pub fn set_votes(&self, value: u64) {
        let mut votes = self.votes.lock().unwrap();
        *votes = value;
    }
    pub fn get_voted_for(&self) -> Option<String> {
        let voted_for = self.voted_for.lock().unwrap();
        voted_for.clone()
    }
    pub fn set_voted_for(&self, node_id: Option<String>) {
        // Persistence storage
        let mut voted_for = self.voted_for.lock().unwrap();
        *voted_for = node_id;
    }

    // pub fn last_voted_for(&self)->  Option<String> {
    //     let last_voted_for  = self.stable.get_str(RaftStateKV::PREV_VOTED_FOR);
    //     if let Ok(Some(voted_for)) = last_voted_for  {
    //         if voted_for == "" {
    //             return None
    //         }
    //         return Some(voted_for);
    //     }
    //     None
    // }
    //
    // pub fn last_vote_term(&self) -> u64{
    //     let last_vote_term  = self.stable.get(RaftStateKV::PREV_VOTE_TERM);
    //     if let Ok(Some(idx)) = last_vote_term  {
    //         return idx;
    //     }
    //     0
    // }
    //
    // pub fn set_vote_term(&self, term: u64) {
    //     if let Err(e) = self.stable.set(RaftStateKV::PREV_VOTE_TERM, term){
    //         error!("unable to persist last vote term to disk: {:?}", e)
    //     };
    // }
}
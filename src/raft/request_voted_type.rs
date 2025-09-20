pub struct RequestVotedArgs {
    pub term: i64,
    pub candidate_id: i64,
    pub last_log_index: i64,
    pub last_log_term: i64,
}

pub struct RequestVotedReply {
    pub term: i64,
    pub vote_granted: bool,
}
use super::types::*;
#[derive(Debug)]
pub struct AppendEntryArgs {
    pub term: i64,
    pub leader_id: i64,

    pub prev_log_index: i64,
    pub prev_log_term: i64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: i64,
}

#[derive(Debug)]
pub struct AppendEntryReply {
    pub term: i64,
    pub success: bool,

    pub conflict_index: i64,
    pub conflict_term: i64,
}
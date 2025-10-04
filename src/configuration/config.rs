use std::sync::Arc;

const DEFAULT_ELECTION_TIMEOUT: u64 = 50;
pub const DEFAULT_ELECTION_TIMEOUT_MIN: u64 = 50;
pub const DEFAULT_ELECTION_TIMEOUT_MAX: u64 = 150;
pub const DEFAULT_HEARTBEAT_INTERVAL: u64 = 10;
pub const DEFAULT_MAX_PAYLOAD_ENTRIES: u64 = 30;
pub const DEFAULT_MAX_LOG_ENTRIES: u64 = 100;
pub const DEFAULT_MAX_APPEND_ENTRIES: usize = 100;

pub struct NodeConfig {
    pub server_id: String,
    pub election_timeout: u64,
    pub election_timeout_min: u64,
    pub election_timeout_max: u64,
    pub heartbeat_interval: u64,
    pub max_log_entries: u64,
    pub max_append_entries: usize,
    pub max_payload_size: u64,
}

impl NodeConfig {
    pub fn new(
        server_id: String,
        election_timeout: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
        heartbeat_interval: Option<u64>,
        max_log_entries: Option<u64>,
        max_append_entries: Option<usize>,
        max_payload_size: Option<u64>,
    ) -> NodeConfig {
        NodeConfig {
            server_id,
            election_timeout: election_timeout.unwrap_or(DEFAULT_ELECTION_TIMEOUT),
            election_timeout_min: election_timeout_min.unwrap_or(DEFAULT_ELECTION_TIMEOUT_MIN),
            election_timeout_max: election_timeout_max.unwrap_or(DEFAULT_ELECTION_TIMEOUT_MAX),
            heartbeat_interval: heartbeat_interval.unwrap_or(DEFAULT_HEARTBEAT_INTERVAL),
            max_log_entries: max_log_entries.unwrap_or(DEFAULT_MAX_LOG_ENTRIES),
            max_append_entries: max_append_entries.unwrap_or(DEFAULT_MAX_APPEND_ENTRIES),
            max_payload_size: max_payload_size.unwrap_or(DEFAULT_MAX_PAYLOAD_ENTRIES),
        }
    }
}



#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub address: String,
}

pub struct MembershipConfigurations {
    pub nodes: Vec<Arc<Node>>,
}
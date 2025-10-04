#[derive(Debug, Clone)]
pub enum RaftError {
    InvalidStorageCapacity,
    ConnectionRefusedError,
    UnableToUnlockNodeState,
    HeartbeatFailure,
    PendingConfiguration,
    LeadershipTransferInProgress,
    LogEntryFailed,
    NotALeader,
    Error(String),
}
use std::sync::{Arc, Mutex};
use super::consensus_module::*;
use super::storage::*;
struct Server {
    lock: Arc<Mutex<()>>,

    server_id: i64,
    peer_ids: Vec<i64>,

    consensus_module: ConsensusModule,
    storage: dyn Storage,
    
}
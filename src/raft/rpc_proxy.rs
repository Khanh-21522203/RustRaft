use std::sync::{Arc};
use tokio::sync::Mutex;
use super::consensus_module::ConsensusModule;
pub struct RPCProxy {
    lock: Arc<Mutex<()>>,
    consensus_module: ConsensusModule,

    // num_calls_before_drop is used to control dropping RPC calls:
    //   -1: means we're not dropping any calls
    //    0: means we're dropping all calls now
    //   >0: means we'll start dropping calls after this number is made
    num_calls_before_drop: i64,
}

impl RPCProxy {
    pub fn new(consensus_module: ConsensusModule) -> Self {
        RPCProxy {
            lock: Arc::new(Mutex::new(())),
            consensus_module,
            num_calls_before_drop: -1,
        }
    }


}
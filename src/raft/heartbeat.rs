use std::sync::{Arc, Mutex};
use std::time::Duration;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use futures::stream::FuturesUnordered;
use tokio_util::sync::CancellationToken;
use crate::configuration::config::Node;
use crate::raft_grpc::{AppendEntriesRequest, AppendEntriesResponse};

pub fn run_periodic_heartbeat(
    nodes: &Vec<Arc<Node>>,
    leader_id: &str,

){
    let heartbeat_cancel = CancellationToken::new();

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(150));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let nodes = nodes.clone();

        loop {
            tokio::select! {
                _ = heartbeat_cancel.cancelled() => {
                    break;
                },

                _ = ticker.tick() => {
                        send_heartbeat(nodes);
                    }
            }
        }
    });

}

pub fn send_heartbeat(
    nodes: Vec<Arc<Node>>,
){
    for node in nodes {

    }
}

pub async fn send_append_entries(
    nodes: &Vec<Arc<Node>>,
    leader_id: &str,
    current_term: u64,
    prev_log_index: u64,
    prev_log_term: u64,
    entries: Vec<LogEntry>,
    leader_commit: u64,
    rpc_timeout: Duration,
    needed: usize
) -> Result<bool, anyhow::Error>{
    let cancel_token = CancellationToken::new();
    let mut futures: FuturesUnordered<BoxFuture<'static, Option<(String, AppendEntriesResponse)>>> = FuturesUnordered::new();

    for node in nodes {
        let child = cancel_token.child_token();
        let leader_id = leader_id.to_string();
        let entries_clone = entries.clone();
        let req_term = current_term;

        let future = async move {
            tokio::select! {
                _ = child.cancelled() => {
                    None
                },

                // result = async {
                    // // skip self -> local append is considered immediate success (leader)
                    // if node.id == leader_id {
                    //     // local ack: simulate leader already appended locally
                    //     Ok(AppendEntriesResponse { term: req_term, success: true, conflict_index: None, conflict_term: None })
                    // } else {
                    //     let req = AppendEntriesRequest {
                    //         term: req_term,
                    //         leader_id: leader_id.clone(),
                    //         prev_log_index,
                    //         prev_log_term,
                    //         entries: entries_clone,
                    //         leader_commit,
                    //     };
                    //     send_append_entries(node.clone(), req, rpc_timeout).await
                    // }
                // } => match result {
                //     Ok(resp) => Some(resp),
                //     Err(e) => {
                //         log::debug!("append_entries RPC error to {}: {:?}", node.address, e);
                //         None
                //     }
                // }
            }
        }.boxed();

        futures.push(future);
    }

    let mut acks = 0usize;
    let mut non_acks = 0usize;

    while let Some(res) = futures.next().await {
        if let Some(resp) = res {
            if resp.1.term > current_term {
                cancel_token.cancel();
                return Err(anyhow::anyhow!("higher term observed: step down"));
            }

            if resp.1.success {
                acks += 1;
            } else {
                non_acks += 1;
            }

            if acks >= needed {
                cancel_token.cancel();
                return Ok(true);
            }

            if non_acks > (nodes.len() - needed) {
                cancel_token.cancel();
                return Ok(false);
            }
        }
    }

    // If we exit loop without quorum, it's a failure
    Ok(false)
}
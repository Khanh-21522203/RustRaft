use std::sync::Arc;
use futures::{stream::{FuturesUnordered, StreamExt}, future::BoxFuture, FutureExt};
use tokio_util::sync::CancellationToken;
use crate::configuration::config::Node;
use crate::raft_grpc::{RequestVoteRequest, RequestVoteResponse};

pub enum ElectionResult {
    Won,
    Lost,
    StepDown,
}

pub async fn grant_vote(
    nodes: &Vec<Arc<Node>>,
    server_id: &str,
    req: RequestVoteRequest,
    needed_votes: usize,
) -> ElectionResult {
    // tokio::sync::OnceCell
    let req = Arc::new(req);
    let cancel_token = CancellationToken::new();  // Tạo token để cancel

    let mut futures: FuturesUnordered<BoxFuture<'static, Option<(String, RequestVoteResponse)>>> = FuturesUnordered::new();

    for node in nodes {
        let req_clone = req.clone();
        let node_clone = node.clone();
        let server_id_clone = server_id.clone();
        let child_token = cancel_token.child_token();

        let future = async move {
            tokio::select! {
                _ = child_token.cancelled() => {
                    return None;
                },

                res = async {
                    if node_clone.id.to_string() == server_id_clone.to_string() {
                        Some((node_clone.id.to_string(), RequestVoteResponse { term: req_clone.term, vote_granted: true }))
                    } else {
                        match node::send_vote_request(&node_clone, (*req_clone).clone()).await {
                            Ok(response) => Some((node_clone.id.clone(), response)),
                            Err(e) => {
                                log::error!("unable to send vote request to {}: {}", node_clone.address, e);
                                None
                            }
                        }
                    }
                } => res
            }
        }.boxed();

        futures.push(future);
    }

    let mut granted_count = 0usize;
    let mut ungranted_count = 0usize;

    while let Some(res) = futures.next().await {
        if let Some(vote) = res {
            if vote.1.term > req.term {
                return ElectionResult::StepDown;
            }

            if vote.1.vote_granted {
                granted_count += 1;
            } else {
                ungranted_count += 1;
            }

            if granted_count >= needed_votes {
                cancel_token.cancel();
                return ElectionResult::Won;
            }

            if ungranted_count >= needed_votes {
                cancel_token.cancel();
                return ElectionResult::Lost;
            }
        }
    }
    ElectionResult::Lost
}

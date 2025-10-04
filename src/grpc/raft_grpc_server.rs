use std::time::Duration;
use tokio::sync::mpsc;
use async_trait::async_trait;
use log;
use tokio::time::sleep;
use tonic::{Request, Response, Status};
use tonic::transport::Server;
use crate::utils::format_server_addr;
use crate::raft_grpc::{AppendEntriesRequest, RequestVoteRequest, TimeoutNowRequest, raft_grpc_transport_server};
use crate::raft_grpc::{AppendEntriesResponse, RequestVoteResponse, TimeoutNowResponse};

pub enum RaftGrpcRequest {
    AppendEntries(AppendEntriesRequest),
    RequestVote(RequestVoteRequest),
    TimeoutNow(TimeoutNowRequest),
}

pub enum RaftGrpcResponse {
    AppendEntries(AppendEntriesResponse),
    RequestVote(RequestVoteResponse),
    TimeoutNow(TimeoutNowResponse),
}

pub struct RaftGrpcServer {
    rpc_send_channel: mpsc::Sender<(RaftGrpcRequest, mpsc::Sender<RaftGrpcResponse>)>,
}

impl RaftGrpcServer {
    pub fn new(rpc_send_channel: mpsc::Sender<(RaftGrpcRequest, mpsc::Sender<RaftGrpcResponse>)>) -> Self {
        RaftGrpcServer { rpc_send_channel }
    }
    pub async fn run(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let server_addr = format_server_addr(addr).parse()?;

        // Just spawn the server task and don't wait on it
        tokio::spawn(async move {
            Server::builder()
                .add_service(raft_grpc_transport_server::RaftGrpcTransportServer::new(self))
                .serve(server_addr)
                .await.expect("unable to start up grpc server");
        });
        sleep(Duration::from_millis(100)).await;

        log::info!("Server is running on {}", addr);
        Ok(())
    }
}

#[async_trait]
impl raft_grpc_transport_server::RaftGrpcTransport for RaftGrpcServer {
    async fn append_entries(&self, request: Request<AppendEntriesRequest>) -> Result<Response<AppendEntriesResponse>, Status> {
        let (response_sender, mut response_receiver) = mpsc::channel(1);
        match self.rpc_send_channel.send((RaftGrpcRequest::AppendEntries(request.into_inner()), response_sender)).await {
            Ok(_) => {
                match response_receiver.recv().await {
                    Some(RaftGrpcResponse::AppendEntries(res)) => Ok(Response::new(res)),
                    _ => Err(Status::internal("Received unexpected response type")),
                }
            },
            Err(e) =>  {
                log::error!("Unable to process append entries request: {}", e);
                Err(Status::internal(format!("Unable to process append entries request: {}", e)))
            }
        }
    }
    async fn request_vote(&self, request: Request<RequestVoteRequest>) -> Result<Response<RequestVoteResponse>, Status> {
        let (response_sender, mut response_receiver) = mpsc::channel(1);
        match self.rpc_send_channel.send((RaftGrpcRequest::RequestVote(request.into_inner()), response_sender)).await{
            Ok(_) => {
                match response_receiver.recv().await {
                    Some(RaftGrpcResponse::RequestVote(res)) => Ok(Response::new(res)),
                    _ => Err(Status::internal("Unexpected response type"))
                }
            },
            Err(e) =>  {
                log::error!("Unable to process request vote request: {}", e);
                Err(Status::internal(format!("Unable to process request vote request: {}", e)))
            }
        }
    }
    async fn timeout_now(&self, request: Request<TimeoutNowRequest>) -> Result<Response<TimeoutNowResponse>, Status> {
        let (response_sender, mut response_receiver) = mpsc::channel(1);
        match self.rpc_send_channel.send((RaftGrpcRequest::TimeoutNow(request.into_inner()), response_sender)).await{
            Ok(_) => {
                match response_receiver.recv().await {
                    Some(RaftGrpcResponse::TimeoutNow(res)) => Ok(Response::new(res)),
                    _ => Err(Status::internal("Unexpected response type"))
                }
            },
            Err(e) =>  {
                log::error!("Unable to process timeout now request: {}", e);
                Err(Status::internal(format!("Unable to process timeout now  request: {}", e)))
            }
        }
    }
}
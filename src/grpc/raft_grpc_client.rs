use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};
use crate::raft_grpc::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest,
                                          RequestVoteResponse, TimeoutNowRequest, TimeoutNowResponse};
use crate::raft_grpc::raft_grpc_transport_client::RaftGrpcTransportClient;
use crate::utils::format_endpoint_addr;

pub struct RaftGrpcClient {
    client: Arc<Mutex<RaftGrpcTransportClient<Channel>>>
}

#[async_trait]
pub trait RaftGrpcClientTrait {
    async fn append_entries(&self, request: AppendEntriesRequest) -> Result<AppendEntriesResponse, tonic::Status>;
    async fn request_vote(&self, request: RequestVoteRequest) -> Result<RequestVoteResponse, tonic::Status>;
    async fn timeout_now(&self, request: TimeoutNowRequest) -> Result<TimeoutNowResponse, tonic::Status>;
}

impl RaftGrpcClient{
    pub async fn new(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let endpoint_addr = format_endpoint_addr(addr);
        let endpoint = Endpoint::from_shared(endpoint_addr)?;
        let channel = endpoint.connect().await?;
        let client: RaftGrpcTransportClient<Channel> = RaftGrpcTransportClient::new(channel);
        Ok(RaftGrpcClient { client: Arc::new(Mutex::new(client)) })
    }
}

#[async_trait]
impl RaftGrpcClientTrait for RaftGrpcClient {
    async fn append_entries(&self, request: AppendEntriesRequest) -> Result<AppendEntriesResponse, tonic::Status> {
        let mut client = self.client.lock().await;
        let response = client.append_entries(request).await?;
        Ok(response.into_inner())
    }
    async fn request_vote(&self, request: RequestVoteRequest) -> Result<RequestVoteResponse, tonic::Status> {
        let mut client = self.client.lock().await;
        let response = client.request_vote(request).await?;
        Ok(response.into_inner())
    }
    async fn timeout_now(&self, request: TimeoutNowRequest) -> Result<TimeoutNowResponse, tonic::Status> {
        let mut client = self.client.lock().await;
        let response = client.timeout_now(request).await?;
        Ok(response.into_inner())
    }
}
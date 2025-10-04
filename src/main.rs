mod raft;
mod data_store;
mod grpc;
mod utils;
mod configuration;
mod error;
mod Utils;

pub mod raft_grpc {
    include!("proto/raft_grpc.rs");
}

fn main() {
    println!("Hello, world!");
}

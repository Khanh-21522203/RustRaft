use std::sync::{Arc};
use async_trait::async_trait;
use bincode::{Decode, Encode};
use bincode::config::standard;
use bincode::serde::decode_from_slice;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::data_store::log_entries::{LogEntry, LogEntryType};
use crate::data_store::sled_data::SledDataStore;

#[async_trait]
pub trait FSM: Send + Sync  + 'static {
    async fn apply(&mut self, log: &LogEntry) -> Box<dyn std::any::Any>;
}


struct StateMachine {
    database: Arc<RwLock<SledDataStore>>
}

#[derive(Serialize, Deserialize, Debug, Encode, Decode)]
enum StateMachineCommand {
    Add { key: String, value: String },
    Remove { key: String },
}

impl StateMachine {
    fn new(database: Arc<RwLock<SledDataStore>>) -> StateMachine {
        StateMachine{
            database
        }
    }
}

#[async_trait]
impl FSM for StateMachine {
    async fn apply(&mut self, log_entry: &LogEntry) -> Box<dyn std::any::Any> {
        info!("{:?}", log_entry);

        let log_entry_type = log_entry.log_entry_type;
        match log_entry_type {
            LogEntryType::LogCommand => {
                let (command, _): (StateMachineCommand, usize) =
                    decode_from_slice(&log_entry.data, standard()).expect("Failed to deserialize command");
                match command {
                    StateMachineCommand::Add { key, value } => {
                        self.database.write().await.add(&key, &value).expect("failed to add data");
                    }
                    StateMachineCommand::Remove { key } => {
                        self.database.write().await.delete(&key).expect("failed to remove data");
                    }
                }
            }
            LogEntryType::LogConfCommand => {}
            LogEntryType::LogNoOp => {}
        }
        Box::new(())
    }
}
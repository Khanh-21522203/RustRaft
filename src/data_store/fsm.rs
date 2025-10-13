use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use log::info;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait FSM: Send + Sync  + 'static {
    async fn apply(&mut self, log: &LogEntry) -> Box<dyn std::any::Any>;
}

struct MyDataBase{

}

struct StateMachine {
    database: Arc<RwLock<MyDataBase>>
}

#[derive(Serialize, Deserialize, Debug)]
enum StateMachineCommand {
    Add { key: String, value: String },
    Remove { key: String },
}

impl StateMachine {
    fn new(database: Arc<RwLock<MyDataBase>>) -> StateMachine {
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
                let command: StateMachineCommand = bincode::deserialize(&log_entry.data).expect("Failed to deserialize command");
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
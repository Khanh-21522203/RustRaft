use sled::{Config, Db, IVec};
use crate::error::store_error::StoreError;

pub trait LogStore: Send + Sync {
    fn get_log(&self, idx: u64) -> Result<Option<LogEntry>, StoreError>;
    fn store_log(&mut self, log: &LogEntry) -> Result<(), StoreError>;
    fn store_logs(&mut self, logs: &[LogEntry]) -> Result<(), StoreError>;
    fn first_index(&self) -> u64;
    fn last_index(&self) -> u64;
    fn delete_range(&mut self, min_idx: u64, max_idx: u64)->  Result<(), StoreError>;
    fn get_logs_from_range(&self, min_idx: u64, max_idx: u64)-> Result<Vec<LogEntry>, StoreError>;
}

struct SledLogStore {
    db: Db
}

impl SledLogStore {
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let config = Config::new()
            .path(path)
            .cache_capacity(1_000_000)
            .mode(sled::Mode::LowSpace);

        let db = config.open()?;
        Ok(Self { db })
    }

    fn key_to_bytes(&self, key: u64) -> IVec {
        IVec::from(&key.to_be_bytes())
    }
}

impl LogStore for SledLogStore {
    /// Retrieves a log entry from the store based on the given index.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the log entry to retrieve.
    ///
    /// # Returns
    ///
    /// A Result containing the log entry if found, or a StoreError if an error occurs.
    fn get_log(&self, idx: u64) -> Result<Option<LogEntry>, StoreError> {
        let key_bytes = self.key_to_bytes(idx);
        match self.db.get(&key_bytes) {
            Ok(Some(ivec)) => Ok(LogEntry::from_bytes(&ivec)),
            Ok(None) => Ok(None),
            Err(e) => Err(StoreError::Error(e.to_string())),
        }
    }

    /// Retrieves a range of log entries from the store.
    ///
    /// # Arguments
    ///
    /// * `min_idx` - The minimum index of the log entries to retrieve.
    /// * `max_idx` - The maximum index of the log entries to retrieve.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of log entries within the specified range, or a StoreError if an error occurs.
    fn get_logs_from_range(&self, min_idx: u64, max_idx: u64) -> Result<Vec<LogEntry>, StoreError> {
        let mut logs = Vec::new();
        for idx in min_idx..=max_idx {
            let key_bytes = self.key_to_bytes(idx);
            match self.db.get(&key_bytes) {
                Ok(Some(ivec)) => {
                    let log_entry = LogEntry::from_bytes(&ivec).ok_or_else(|| {
                        StoreError::Error(format!("Failed to deserialize log entry at index {}", idx))
                    })?;
                    logs.push(log_entry);
                }
                Ok(None) => {}
                Err(e) => return Err(StoreError::Error(e.to_string())),
            }
        }

        Ok(logs)
    }

    /// Stores a single log entry in the store.
    ///
    /// # Arguments
    ///
    /// * `log` - The log entry to store.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a StoreError if an error occurs.
    fn store_log(&mut self, log: &LogEntry) -> Result<(), StoreError> {
        let bytes = log.to_bytes();
        let key_bytes = self.key_to_bytes(log.index);
        self.db
            .insert(key_bytes, bytes)
            .map_err(|e| StoreError::InsertionError(e.to_string()))?;

        self.db.flush().map_err(|e| StoreError::Error(e.to_string()))?;
        Ok(())
    }

    /// Stores multiple log entries in the store as a batch operation.
    ///
    /// # Arguments
    ///
    /// * `logs` - A slice of log entries to store.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a StoreError if an error occurs.
    fn store_logs(&mut self, logs: &[LogEntry]) -> Result<(), StoreError> {
        let mut batch = sled::Batch::default();

        for log in logs {
            let key_bytes = self.key_to_bytes(log.index);
            batch.insert(key_bytes, log.to_bytes());
        }

        self.db
            .apply_batch(batch)
            .map_err(|e| StoreError::InsertionError(e.to_string()))?;

        self.db.flush().map_err(|e| StoreError::Error(e.to_string()))?;

        Ok(())
    }

    /// Retrieves the index of the first log entry in the store.
    fn first_index(&self) -> u64 {
        self.db
            .iter()
            .keys()
            .next()
            .and_then(|result| result.ok())
            .and_then(|ivec| {
                let bytes: [u8; 8] = ivec.as_ref().try_into().ok()?;
                Some(u64::from_be_bytes(bytes))
            })
            .unwrap_or_default()
    }

    /// Retrieves the index of the last log entry in the store.
    fn last_index(&self) -> u64 {
        self.db
            .iter()
            .keys()
            .last()
            .and_then(|result| result.ok())
            .and_then(|ivec| {
                let bytes: [u8; 8] = ivec.as_ref().try_into().ok()?;
                Some(u64::from_be_bytes(bytes))
            })
            .unwrap_or_default()
    }

    /// Deletes a range of log entries from the store.
    ///
    /// # Arguments
    ///
    /// * `min_idx` - The minimum index of the log entries to delete.
    /// * `max_idx` - The maximum index of the log entries to delete.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a StoreError if an error occurs.
    fn delete_range(&mut self, min_idx: u64, max_idx: u64) -> Result<(), StoreError> {
        for idx in min_idx..=max_idx {
            let key_bytes = self.key_to_bytes(idx);
            self.db.remove(key_bytes).map_err(|e| StoreError::Error(e.to_string()))?;
        }

        self.db.flush().map_err(|e| StoreError::Error(e.to_string()))?;
        Ok(())
    }
}

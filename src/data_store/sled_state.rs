use sled::{Config, Db};
use crate::error::store_error::StoreError;
use serde::{de::DeserializeOwned, Serialize};
use bincode::serde::decode_from_slice;
use bincode::config;

pub trait StateStore: Send + Sync {
    fn set(&self, key: &str, value: u64) -> Result<(), StoreError>;
    fn get(&self, key: &str) -> Result<Option<u64>, StoreError>;
    fn set_str(&self, key: &str, value: String) -> Result<(), StoreError>;
    fn get_str(&self, key: &str) -> Result<Option<String>, StoreError>;
}

pub struct SledStateStore {
    db: Db
}

impl SledStateStore {
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let config = Config::new()
            .path(path)
            .cache_capacity(1_000_000)
            .mode(sled::Mode::LowSpace);

        let db = config.open()?;
        Ok(Self { db })
    }

    fn get<V>(&self, key: &str) -> Result<Option<V>, StoreError>
    where V: DeserializeOwned{
        let key_bytes = key.as_bytes();
        let result = self.db.get(&key_bytes).map_err(|e| StoreError::Error(e.to_string()))?;

        match result {
            Some(value) => {
                let (deserialized_value, _bytes_read) =
                    decode_from_slice::<V, _>(value.as_ref(), config::standard())
                        .map_err(|e| StoreError::Error(e.to_string()))?;

                Ok(Some(deserialized_value))
            }
            None => Ok(None),
        }
    }

    fn set<V>(&self, key: &str, value: &V) -> Result<(), StoreError>
    where V: Serialize {
        let key_bytes = key.as_bytes();
        let value_bytes = bincode::serde::encode_to_vec(value, config::standard())
            .map_err(|e| StoreError::Error(e.to_string()))?;

        self.db
            .insert(&key_bytes, value_bytes)
            .map_err(|e| StoreError::Error(e.to_string()))?;

        self.db.flush().map_err(|e| StoreError::Error(e.to_string()))?;
        Ok(())
    }
}

impl StateStore for SledStateStore {
    fn set(&self, key: &str, value: u64) -> Result<(), StoreError>{
        self.set(key, &value)
    }
    fn  get(&self, key: &str) -> Result<Option<u64>, StoreError>{
        self.get(key)
    }
    fn set_str(&self, key: &str, value: String) -> Result<(), StoreError>{
        self.set(key, &value)
    }
    fn  get_str(&self, key: &str) -> Result<Option<String>, StoreError>{
        self.get(key)
    }
}
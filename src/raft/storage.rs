use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Storage is an interface implemented by stable storage providers.
pub trait Storage: Send + Sync {
    fn set(&self, key: String, value: Vec<u8>);
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn has_data(&self) -> bool;
}

/// MapStorage is a simple in-memory implementation of Storage for testing.
pub struct MapStorage {
    m: Mutex<HashMap<String, Vec<u8>>>,
}

impl MapStorage {
    pub fn new() -> Self {
        MapStorage {
            m: Mutex::new(HashMap::new()),
        }
    }
}

impl Storage for MapStorage {
    fn set(&self, key: String, value: Vec<u8>) {
        let mut map = self.m.lock().unwrap();
        map.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<Vec<u8>> {
        let map = self.m.lock().unwrap();
        map.get(key).cloned()
    }

    fn has_data(&self) -> bool {
        let map = self.m.lock().unwrap();
        !map.is_empty()
    }
}
use std::sync::RwLock;

pub struct ThreadSafeBool {
    value: RwLock<bool>,
}

impl ThreadSafeBool {
    pub fn new(initial_value: bool) -> Self {
        ThreadSafeBool {
            value: RwLock::new(initial_value),
        }
    }

    pub fn get(&self) -> bool {
        *self.value.read().unwrap()
    }

    pub fn set(&self, value: bool) {
        *self.value.write().unwrap() = value;
    }
}
// use std::collections::HashMap;
// use std::sync::{Arc, Mutex};
// 
// pub struct DataStore {
//     lock: Arc<Mutex<()>>,
//     data: HashMap<String, String>,
// }
// 
// impl DataStore {
//     fn new() -> Self {
//         DataStore {
//             lock: Arc::new(Mutex::new(())),
//             data: HashMap::new(),
//         }
//     }
// 
//     pub fn get(&self, key: &str) -> (String, bool) {
//         let lock = self.data.lock().unwrap();
//         match map.get(key) {
//             Some(v) => (v.clone(), true),
//             None => (String::new(), false),
//         }
//     }
// 
//     pub fn put(&self, key: String, value: String) -> (String, bool) {
//         let mut lock = self.data.lock().unwrap();
//         match map.insert(key, value) {
//             Some(prev) => (prev, true),
//             None => (String::new(), false),
//         }
//     }
// 
//     pub fn append(&self, key: String, value: String) -> (String, bool) {
//         let mut lock = self.data.lock().unwrap();
//         if let Some(prev) = map.get_mut(&key) {
//             let old = prev.clone();
//             prev.push_str(&value);
//             (old, true)
//         } else {
//             map.insert(key, value);
//             (String::new(), false)
//         }
//     }
// 
//     pub fn cas(&self, key: String, compare: String, value: String) -> (String, bool) {
//         let mut lock = self.data.lock().unwrap();
//         if let Some(prev) = map.get_mut(&key) {
//             let old = prev.clone();
//             let existed = true;
//             if old == compare {
//                 *prev = value;
//             }
//             (old, existed)
//         } else {
//             (String::new(), false)
//         }
//     }
// }
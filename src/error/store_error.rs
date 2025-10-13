#[derive(Debug, Clone)]
pub enum StoreError {
    InsertionError(String),
    Error(String),
}
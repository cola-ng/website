use thiserror::Error;

#[derive(Debug, Error)]
pub enum PoolError {
    #[error("database acquisition failed: {0}")]
    Queue(String),
}

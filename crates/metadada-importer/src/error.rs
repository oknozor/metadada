#[derive(Debug, thiserror::Error)]

pub enum ReplicationError {
    #[error("Next replication packet not found")]
    NotFound,
    #[error("Http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Database error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Send error: {0}")]
    Send(#[from] tokio::sync::mpsc::error::SendError<()>),
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

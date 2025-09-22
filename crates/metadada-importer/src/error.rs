#[derive(Debug, thiserror::Error)]

pub enum ReplicationError {
    #[error("Http status error: {0}")]
    HttpStatusError(u16),
    #[error("Http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Database error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    CSVError(#[from] csv::Error),
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Send error: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<()>),
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

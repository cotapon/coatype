use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("api returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

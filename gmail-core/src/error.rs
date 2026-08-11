//! Error types for gmail-core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GmailError {
    #[error("Authentication error: {0}")]
    Auth(Box<dyn std::error::Error + Send + Sync>),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP/3 error: {0}")]
    Http3(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("rkyv serialization error: {0}")]
    Rkyv(#[from] rkyv::rancor::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Token error: {0}")]
    Token(String),

    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Streaming error: {0}")]
    Streaming(String),

    #[error("Batch operation failed: {failed}/{total} operations failed")]
    BatchPartial { failed: usize, total: usize },

    #[error("Dictionary not found: {0}")]
    DictionaryNotFound(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("DNS resolution error: {0}")]
    Dns(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl GmailError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            GmailError::Http(e) if e.is_timeout() || e.is_connect() || e.is_request()
        ) || matches!(
            self,
            GmailError::RateLimited { .. } |
            GmailError::Timeout(_)
        ) || matches!(
            self,
            GmailError::Api { status, .. } if *status >= 500 || *status == 429
        )
    }

    /// Get HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            GmailError::Api { status, .. } => Some(*status),
            GmailError::Http(e) => e.status().map(|s| s.as_u16()),
            GmailError::RateLimited { .. } => Some(429),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, GmailError>;
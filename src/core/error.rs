//! Error types for grr-core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GrrError {
    #[error("Authentication error: {0}")]
    Auth(Box<dyn std::error::Error + Send + Sync>),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

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

impl GrrError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            GrrError::Http(e) if e.is_timeout() || e.is_connect() || e.is_request()
        ) || matches!(self, GrrError::RateLimited { .. } | GrrError::Timeout(_))
            || matches!(
                self,
                GrrError::Api { status, .. } if *status >= 500 || *status == 429
            )
    }

    /// Get HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            GrrError::Api { status, .. } => Some(*status),
            GrrError::Http(e) => e.status().map(|s| s.as_u16()),
            GrrError::RateLimited { .. } => Some(429),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, GrrError>;

pub(crate) fn api_error(status: u16, body: &str) -> GrrError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .map(|value| json_error_detail(&value))
        .filter(|message| message != "unknown error")
        .unwrap_or_else(|| {
            let body = body.trim();
            if body.is_empty() {
                "HTTP request failed".to_string()
            } else {
                body.to_string()
            }
        });
    GrrError::Api { status, message }
}

pub(crate) fn json_error_detail(body: &serde_json::Value) -> String {
    let reason = body
        .get("error")
        .and_then(|value| {
            value
                .as_str()
                .or_else(|| value.get("message").and_then(serde_json::Value::as_str))
        })
        .or_else(|| body.get("message").and_then(serde_json::Value::as_str))
        .unwrap_or("unknown error");
    let description = body
        .get("error_description")
        .and_then(serde_json::Value::as_str)
        .or_else(|| body.get("description").and_then(serde_json::Value::as_str))
        .unwrap_or("");
    if description.is_empty() || description == reason {
        reason.to_string()
    } else {
        format!("{reason}: {description}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_google_api_error_messages() {
        let error = api_error(
            400,
            r#"{"error":{"code":400,"message":"invalid query","status":"INVALID_ARGUMENT"}}"#,
        );
        assert!(
            matches!(error, GrrError::Api { status: 400, message } if message == "invalid query")
        );
    }

    #[test]
    fn extracts_oauth_error_details() {
        let value = json!({"error":"invalid_grant", "error_description":"expired"});
        assert_eq!(json_error_detail(&value), "invalid_grant: expired");
    }
}

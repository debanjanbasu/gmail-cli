use grr_core::error::GmailError;
use reqwest::StatusCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_429_returns_rate_limited_with_retry_after() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "5"))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/test", server.uri()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers().get("retry-after").unwrap(), "5");
}

#[tokio::test]
async fn test_rate_limited_error_has_retry_after() {
    let err = GmailError::RateLimited {
        retry_after_secs: 30,
    };
    assert_eq!(err.to_string(), "Rate limited: retry after 30s");
    assert!(err.is_retryable());
    assert_eq!(err.status_code(), Some(429));
}

#[tokio::test]
async fn test_5xx_errors_are_retryable() {
    let err = GmailError::Api {
        status: 500,
        message: "Server error".into(),
    };
    assert!(err.is_retryable());
    assert_eq!(err.status_code(), Some(500));
}

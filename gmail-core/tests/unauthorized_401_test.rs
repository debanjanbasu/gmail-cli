//! Offline regression tests for 401 handling in the retry executor.
//!
//! Regression: a 401 used to trigger `revoke()` + retry inside
//! `execute_with_retry`. That was doubly broken — the retry re-sent the same
//! stale bearer token (fetched once above the loop), and `revoke()` emptied
//! storage so any subsequent call fell into the implicit interactive OAuth
//! flow mid-request. The fix returns a fast, actionable `Auth` error.

use std::time::Duration;

use gmail_core::{
    GmailAuth, GmailClient, GmailClientBuilder, GmailConfig, GmailError, PerformanceConfig,
    TokenStorage,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn unauthorized_401_fails_fast_without_retrying_stale_token() {
    let server = MockServer::start().await;
    // Exactly one request may ever hit the API for a 401: retrying cannot
    // succeed because the same stale token would be re-sent.
    Mock::given(method("GET"))
        .and(path("/users/me/messages/m-401"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": { "code": 401, "message": "Request had invalid authentication credentials.", "status": "UNAUTHENTICATED" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let result = tokio::time::timeout(Duration::from_secs(5), client.get_message("m-401", None))
        .await
        .expect("401 must fail fast, not burn retry backoff cycles");

    let err = result.expect_err("a 401 must yield an error");
    assert!(
        matches!(err, GmailError::Auth(_)),
        "expected Auth error, got: {err:?}"
    );
    let message = err.to_string();
    assert!(
        message.contains("gmail auth login"),
        "remediation guidance missing: {message}"
    );

    // No retry storm against the API with a dead credential.
    let received = server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "401 must not be retried with the same stale token"
    );
}

/// Client pointed at the mock server with a valid in-memory token and a
/// tempdir token path, so neither OAuth flows nor (pre-fix) `revoke()` calls
/// can touch the user's real cached credential.
async fn test_client(base: &str) -> GmailClient {
    let config = GmailConfig {
        performance: PerformanceConfig {
            enable_http3: false,
            enable_http2: false,
            ..PerformanceConfig::default()
        },
        ..GmailConfig::default()
    };
    let storage = TokenStorage {
        access_token: "test-token".into(),
        refresh_token: None,
        expires_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
        token_type: "Bearer".into(),
        scope: "test-scope".into(),
    };
    let dir = tempfile::tempdir().unwrap();
    let auth = GmailAuth::with_token(config.oauth.clone(), storage)
        .await
        .unwrap()
        .with_token_path(dir.path().join("token.json"));
    GmailClientBuilder::new(config)
        .auth(auth)
        .base_url(base.parse().unwrap())
        .upload_base_url(format!("{}/", base).parse().unwrap())
        .build()
        .await
        .unwrap()
}

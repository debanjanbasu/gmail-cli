//! Offline regression tests for refresh-token failure handling.
//!
//! Regression: a rejected refresh token (Google 400 `invalid_grant`) used to
//! silently fall through into the implicit interactive OAuth flow, which binds
//! port 3434 mid-API-call and hangs callers for the full callback timeout.
//! The fix makes `get_access_token` return a fast, actionable `Auth` error
//! carrying Google's underlying reason; the implicit flow is reserved for the
//! fresh-install / explicit-login case (no stored token at all).

use std::time::Duration;

use gmail_core::{GmailAuth, GmailConfig, GmailError, TokenStorage};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn expired_storage_with_refresh_token() -> TokenStorage {
    TokenStorage {
        access_token: "stale-access-token".into(),
        refresh_token: Some("stale-refresh-token".into()),
        expires_at: 1, // long expired
        token_type: "Bearer".into(),
        scope: "test-scope".into(),
    }
}

#[tokio::test]
async fn rejected_refresh_token_returns_auth_error_without_oauth_flow() {
    let server = MockServer::start().await;
    let guard = Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Token has been expired or revoked."
        })))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let config = GmailConfig::default();
    let auth = GmailAuth::with_token(config.oauth.clone(), expired_storage_with_refresh_token())
        .await
        .unwrap()
        .with_token_endpoint(format!("{}/token", server.uri()));

    let result = tokio::time::timeout(Duration::from_secs(5), auth.get_access_token())
        .await
        .expect("get_access_token must fail fast, not wait on an OAuth callback timeout");

    let err = result.expect_err("a rejected refresh token must yield an error");
    assert!(
        matches!(err, GmailError::Auth(_)),
        "unexpected error: {err:?}"
    );
    let message = err.to_string();
    assert!(
        message.contains("invalid_grant"),
        "underlying reason not surfaced: {message}"
    );
    assert!(
        message.contains("gmail auth login"),
        "remediation guidance missing: {message}"
    );

    // Panics unless the token endpoint saw exactly one request, i.e. no
    // second attempt and no callback-server detour happened.
    drop(guard);
}

#[tokio::test]
async fn successful_refresh_still_updates_the_access_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "fresh-access-token",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let config = GmailConfig::default();
    let auth = GmailAuth::with_token(config.oauth.clone(), expired_storage_with_refresh_token())
        .await
        .unwrap()
        .with_token_endpoint(format!("{}/token", server.uri()));

    let token = tokio::time::timeout(Duration::from_secs(5), auth.get_access_token())
        .await
        .expect("refresh must complete promptly")
        .unwrap();
    assert_eq!(token, "fresh-access-token");

    // The happy path persists the refreshed token to the cache dir; clean up
    // so the test never leaves a fake credential on the machine.
    if let Some(cache) = dirs::cache_dir() {
        let _ = std::fs::remove_file(cache.join("gmail-opencode").join("token.json"));
    }
}

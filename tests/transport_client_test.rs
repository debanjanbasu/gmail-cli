#![cfg(feature = "gmail")]
//! Client-construction transport tests for the Gmail client over the
//! shared HTTP core.

use grr_cli::core::{GoogleAuth, GrrConfig};
use grr_cli::gmail::GmailClientBuilder;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn base_url_override_builds_without_probing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/profile"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = GrrConfig::default();

    let auth = GoogleAuth::new(config.oauth.clone()).await.unwrap();
    let client = GmailClientBuilder::new()
        .auth(auth)
        .base_url(server.uri().parse().unwrap())
        .build()
        .await
        .unwrap();

    let info = client.transport_info();
    assert_eq!(info.negotiated_version, "not-probed");
    assert!(info.http3_requested);
    assert!(!info.http3_effective);
    assert!(!info.fell_back);
}

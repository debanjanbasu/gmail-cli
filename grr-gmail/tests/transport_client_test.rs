//! Client-construction transport tests for the Gmail client over the
//! shared HTTP core.

use grr_core::{GoogleAuth, GrrConfig};
use grr_gmail::GmailClientBuilder;
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

    // HTTP/3 is compile-time (the `http3` feature). In a workspace run the
    // CLI's feature unification turns it on; assert against the
    // grr-core-visible state, not this test crate's cfg.
    // A base_url override skips the probe entirely.
    let info = client.transport_info();
    assert_eq!(info.negotiated_version, "not-probed");
    assert_eq!(info.http3_requested, grr_core::http::HTTP3_COMPILED);
    assert!(!info.http3_effective);
    assert!(!info.fell_back);
}

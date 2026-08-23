use gmail_core::client::{TransportMode, apply_transport_version, resolve_transport_mode};
use gmail_core::config::PerformanceConfig;
use gmail_core::{GmailAuth, GmailClientBuilder, GmailConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn h3_requested_with_feature_enabled_wins_over_h2() {
    assert_eq!(
        resolve_transport_mode(true, true, true),
        TransportMode::Http3PriorKnowledge
    );
}

#[test]
fn h3_requested_without_feature_falls_to_h2() {
    assert_eq!(
        resolve_transport_mode(true, false, true),
        TransportMode::Http2PriorKnowledge
    );
}

#[test]
fn neither_enabled_is_alpn_default() {
    assert_eq!(
        resolve_transport_mode(false, true, false),
        TransportMode::AlpnDefault
    );
}

#[test]
fn h2_only_is_prior_knowledge() {
    assert_eq!(
        resolve_transport_mode(false, true, true),
        TransportMode::Http2PriorKnowledge
    );
}

#[test]
fn apply_transport_version_forces_h3_when_effective() {
    let builder = reqwest::Client::new().get("https://example.com/users/me/profile");
    let request = apply_transport_version(builder, true).build().unwrap();
    assert_eq!(request.version(), reqwest::Version::HTTP_3);
}

#[test]
fn apply_transport_version_is_noop_without_h3() {
    let builder = reqwest::Client::new().get("https://example.com/users/me/profile");
    let request = apply_transport_version(builder, false).build().unwrap();
    assert_eq!(request.version(), reqwest::Version::HTTP_11);
}

#[tokio::test]
async fn base_url_override_builds_without_probing_when_h3_disabled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/profile"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = GmailConfig {
        performance: PerformanceConfig {
            enable_http3: false,
            ..PerformanceConfig::default()
        },
        ..GmailConfig::default()
    };

    let auth = GmailAuth::new(config.oauth.clone()).await.unwrap();
    let client = GmailClientBuilder::new(config)
        .auth(auth)
        .base_url(server.uri().parse().unwrap())
        .build()
        .await
        .unwrap();

    let info = client.transport_info();
    assert_eq!(info.negotiated_version, "not-probed");
    assert!(!info.http3_requested);
    // http3_effective=false must leave requests unversioned (no h3 override).
    assert!(!info.http3_effective);
    assert!(!info.fell_back);
}

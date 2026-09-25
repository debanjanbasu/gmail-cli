//! Transport selection unit tests for the shared HTTP core.

use grr_cli::core::http::{TransportMode, apply_transport_version, resolve_transport_mode};

#[test]
fn h3_requested_with_feature_enabled_wins_over_h2() {
    assert_eq!(
        resolve_transport_mode(true, true, true),
        TransportMode::Http3PriorKnowledge
    );
}

#[test]
fn h3_without_feature_falls_back_to_h2() {
    assert_eq!(
        resolve_transport_mode(true, false, true),
        TransportMode::Http2PriorKnowledge
    );
}

#[test]
fn neither_requested_yields_alpn_default() {
    assert_eq!(
        resolve_transport_mode(false, false, false),
        TransportMode::AlpnDefault
    );
}

#[test]
fn apply_transport_version_pins_h3_when_effective() {
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

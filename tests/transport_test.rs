//! Transport selection unit tests for the shared HTTP core.

use grr_cli::core::http::{
    TransportInfo, TransportMode, apply_transport_version, resolve_transport_mode,
};

#[test]
fn h3_mode_is_selected_for_the_unconditional_transport() {
    assert_eq!(
        resolve_transport_mode(true),
        TransportMode::Http3PriorKnowledge
    );
}

#[test]
fn h2_mode_is_used_only_for_runtime_fallback() {
    assert_eq!(
        resolve_transport_mode(false),
        TransportMode::Http2PriorKnowledge
    );
}

#[test]
fn default_transport_info_requests_http3() {
    let info = TransportInfo::default();
    assert!(info.http3_requested);
    assert!(!info.http3_effective);
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

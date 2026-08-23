use gmail_core::client::{resolve_transport_mode, TransportMode};

#[test]
fn h3_requested_with_feature_enabled_wins_over_h2() {
    assert_eq!(resolve_transport_mode(true, true, true), TransportMode::Http3PriorKnowledge);
}

#[test]
fn h3_requested_without_feature_falls_to_h2() {
    assert_eq!(resolve_transport_mode(true, false, true), TransportMode::Http2PriorKnowledge);
}

#[test]
fn neither_enabled_is_alpn_default() {
    assert_eq!(resolve_transport_mode(false, true, false), TransportMode::AlpnDefault);
}

#[test]
fn h2_only_is_prior_knowledge() {
    assert_eq!(resolve_transport_mode(false, true, true), TransportMode::Http2PriorKnowledge);
}

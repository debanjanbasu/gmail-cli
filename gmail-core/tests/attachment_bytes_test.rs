use base64::Engine;
use gmail_core::{
    GmailAuth, GmailClient, GmailClientBuilder, GmailConfig, PerformanceConfig, TokenStorage,
};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PAYLOAD: &[u8] = b"PDF-ish attachment body \x00\x01\x02 binary safe";
const RAW_RFC822: &str =
    "From: a@example.com\r\nTo: b@example.com\r\nSubject: hi\r\n\r\nbody \x00\x01 here";

fn b64(input: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

#[tokio::test]
async fn get_attachment_decodes_in_single_allocation() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"messages/.*/attachments/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "size": PAYLOAD.len(),
            "data": b64(PAYLOAD),
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let got = client.get_attachment_bytes("msg1", "att1").await.unwrap();
    assert_eq!(&got[..], PAYLOAD);
}

#[tokio::test]
async fn download_attachment_to_writes_exact_bytes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"messages/.*/attachments/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "size": PAYLOAD.len(),
            "data": b64(PAYLOAD),
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("a.bin");
    let n = client
        .download_attachment_to("msg1", "att1", &out)
        .await
        .unwrap();
    assert_eq!(n as usize, PAYLOAD.len());
    assert_eq!(std::fs::read(&out).unwrap(), PAYLOAD);
}

#[tokio::test]
async fn get_message_raw_bytes_decodes_exactly() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"messages/raw1$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "raw1",
            "raw": b64(RAW_RFC822.as_bytes()),
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let got = client.get_message_raw_bytes("raw1").await.unwrap();
    assert_eq!(&got[..], RAW_RFC822.as_bytes());
}

/// Build a client pointed at the mock server with a pre-authed, in-memory
/// token so no OAuth flow or token-endpoint round-trip ever happens.
/// Both h3 and h2 prior-knowledge are disabled: wiremock speaks HTTP/1.1.
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
    let auth = GmailAuth::with_token(config.oauth.clone(), storage)
        .await
        .unwrap();
    GmailClientBuilder::new(config)
        .auth(auth)
        .base_url(base.parse().unwrap())
        .build()
        .await
        .unwrap()
}

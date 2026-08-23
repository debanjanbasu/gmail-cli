use base64::Engine;
use gmail_core::client::{StreamAttachment, mime_message_stream};
use gmail_core::{GmailAuth, GmailClient, GmailClientBuilder, GmailConfig, PerformanceConfig, TokenStorage};
use tokio_util::io::ReaderStream;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Split the recorded body on `\r\n\r\n` boundaries, take the last non-empty
/// segment, and drop any trailing boundary-marker lines so only base64 chars remain.
fn extract_last_base64_segment(body: &str) -> String {
    body.split("\r\n\r\n")
        .filter(|segment| {
            segment
                .lines()
                .any(|line| !line.is_empty() && !line.starts_with("--"))
        })
        .last()
        .map(|segment| {
            segment
                .lines()
                .filter(|line| !line.starts_with("--"))
                .collect::<Vec<&str>>()
                .join("")
        })
        .unwrap_or_default()
}

fn message_json(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "threadId": id,
        "labelIds": [],
        "snippet": null,
        "historyId": null,
        "internalDate": null,
        "payload": null,
        "sizeEstimate": 0,
        "raw": null
    })
}

#[tokio::test]
async fn mime_stream_assembles_headers_body_and_attachments() {
    let server = MockServer::start().await;
    let expected_file = "streaming attachment content";
    Mock::given(method("POST"))
        .and(path_regex(r"users/me/messages/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(message_json("m1")))
        .expect(1)
        .mount(&server)
        .await;

    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), expected_file).unwrap();

    let client = test_client(&server.uri()).await;
    let atts = vec![StreamAttachment {
        path: file.path().to_path_buf(),
        filename: "data.txt".into(),
        mime_type: "text/plain".into(),
    }];
    let msg = client
        .send_mime_stream(mime_message_stream("a@b.c", "Subj", "hello", atts, None), None)
        .await
        .unwrap();
    assert_eq!(msg.id.as_str(), "m1");

    let recorded = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        recorded.headers.get("content-type").and_then(|v| v.to_str().ok()),
        Some("message/rfc822")
    );
    let body = String::from_utf8(recorded.body.clone()).unwrap();
    assert!(body.starts_with("To: a@b.c\r\n"));
    assert!(body.contains("Content-Type: multipart/mixed"));
    assert!(body.contains("hello"));
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(extract_last_base64_segment(&body))
        .unwrap();
    assert_eq!(decoded, expected_file.as_bytes());
}

#[tokio::test]
async fn large_attachment_chunks_across_multiple_frames_without_corruption() {
    let server = MockServer::start().await;
    // > CHUNK (192 KiB) so multiple base64 frames are emitted; length chosen
    // so the tail chunk exercises padding (len % 3 != 0 overall).
    let expected: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    assert!(expected.len() > 192 * 1024);

    Mock::given(method("POST"))
        .and(path_regex(r"users/me/messages/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(message_json("m2")))
        .expect(1)
        .mount(&server)
        .await;

    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &expected).unwrap();

    let client = test_client(&server.uri()).await;
    let atts = vec![StreamAttachment {
        path: file.path().to_path_buf(),
        filename: "big.bin".into(),
        mime_type: "application/octet-stream".into(),
    }];
    let msg = client
        .send_mime_stream(mime_message_stream("a@b.c", "Subj", "hi", atts, None), None)
        .await
        .unwrap();
    assert_eq!(msg.id.as_str(), "m2");

    let recorded = &server.received_requests().await.unwrap()[0];
    let body = String::from_utf8(recorded.body.clone()).unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(extract_last_base64_segment(&body))
        .unwrap();
    assert_eq!(decoded, expected);
}

#[tokio::test]
async fn import_streams_rfc822_to_media_endpoint() {
    let server = MockServer::start().await;
    let rfc822 = concat!(
        "From: x@example.com\r\n",
        "To: y@example.com\r\n",
        "Subject: streamed import\r\n",
        "\r\n",
        "line one\r\nline two\x00\x01binary tail"
    );
    Mock::given(method("POST"))
        .and(path_regex(r"users/me/messages/import"))
        .respond_with(ResponseTemplate::new(200).set_body_json(message_json("m3")))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("import.eml");
    std::fs::write(&path, rfc822).unwrap();

    let client = test_client(&server.uri()).await;
    let file = tokio::fs::File::open(&path).await.unwrap();
    let msg = client.import_stream(ReaderStream::new(file), true).await.unwrap();
    assert_eq!(msg.id.as_str(), "m3");

    let recorded = &server.received_requests().await.unwrap()[0];
    assert_eq!(recorded.body, rfc822.as_bytes());
    let pairs: Vec<(String, String)> = recorded
        .url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    assert!(pairs.contains(&("uploadType".into(), "media".into())));
    assert!(pairs.contains(&("internalDateSource".into(), "dateHeader".into())));
    assert!(pairs.contains(&("deleted".into(), "true".into())));
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
        .upload_base_url(format!("{}/", base).parse().unwrap())
        .build()
        .await
        .unwrap()
}

//! Live end-to-end verification against the real Gmail API.
//!
//! [`live_full_pipeline`] exercises the full data plane — search, attachment
//! download, streaming import, trash — and records the transport negotiated
//! during client construction. It is doubly gated: marked `#[ignore]` *and*
//! skipped unless `LIVE_E2E=1`, so ordinary `cargo test` never hits the
//! network.
//!
//! Run with:
//! `LIVE_E2E=1 cargo test -p gmail-core --test live_e2e -- --ignored --nocapture`
//!
//! Credentials: config via `ConfigLoader` (honours `GMAIL_CONFIG_PATH`),
//! token from `~/.gmail-opencode/token.json` (honours `GMAIL_TOKEN_PATH`;
//! note `GmailAuth::new` reads the cache-dir copy instead). Set
//! `LIVE_E2E_HTTP3=1` to force `enable_http3` for this run so the QUIC probe
//! (and any fallback) is exercised regardless of the on-disk config.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gmail_core::client::{TransportInfo, mime_message_stream};
use gmail_core::models::{Message, MessageBody, MessagePayload, MessageRef};
use gmail_core::{ConfigLoader, GmailAuth, GmailClient, GmailClientBuilder, TokenStorage};

const LIVE_E2E_ENV: &str = "LIVE_E2E";
const FORCE_HTTP3_ENV: &str = "LIVE_E2E_HTTP3";
const TOKEN_PATH_ENV: &str = "GMAIL_TOKEN_PATH";
const INBOX_QUERY: &str = "in:inbox";
const ATTACHMENT_QUERY: &str = "in:inbox has:attachment";
const SEARCH_CANDIDATES: usize = 5;

/// An attachment discovered while walking message metadata.
struct SelectedAttachment {
    attachment_id: String,
    filename: String,
    size: u64,
}

/// The live leg runs only when the env var is exactly `"1"`.
fn env_flag_is_one(value: Option<&str>) -> bool {
    value.is_some_and(|v| v == "1")
}

/// Depth-first walk of a MIME tree collecting parts that reference an
/// attachment body.
fn collect_attachments(payload: &MessagePayload, out: &mut Vec<SelectedAttachment>) {
    if let Some(attachment_id) = payload.body.attachment_id.as_ref()
        && let Some(filename) = payload.filename.as_ref()
    {
        out.push(SelectedAttachment {
            attachment_id: attachment_id.clone(),
            filename: filename.clone(),
            size: payload.body.size,
        });
    }
    for part in payload.parts.iter().flatten() {
        collect_attachments(part, out);
    }
}

/// Pick the first metadata response that carries an attachment.
fn select_message_with_attachment(messages: &[Message]) -> Option<(&Message, SelectedAttachment)> {
    messages.iter().find_map(|message| {
        let mut found = Vec::new();
        if let Some(payload) = &message.payload {
            collect_attachments(payload, &mut found);
        }
        found.into_iter().next().map(|attachment| (message, attachment))
    })
}

fn format_transport_summary(info: &TransportInfo) -> String {
    format!(
        "negotiated={} http3_requested={} http3_effective={} fell_back={}",
        info.negotiated_version,
        info.http3_requested,
        info.http3_effective,
        info.fell_back,
    )
}

/// Resolve the token path used by this machine's credential setup.
fn token_path() -> PathBuf {
    std::env::var_os(TOKEN_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".gmail-opencode")
                .join("token.json")
        })
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

async fn fetch_metadata(client: &GmailClient, refs: &[MessageRef]) -> Vec<Message> {
    let mut messages = Vec::with_capacity(refs.len());
    for message_ref in refs {
        match client.get_message_metadata(&message_ref.id).await {
            Ok(message) => messages.push(message),
            Err(err) => eprintln!("[metadata] {}: {err}", message_ref.id),
        }
    }
    messages
}

#[cfg(test)]
mod pure_logic_tests {
    use super::*;

    fn leaf_part(mime_type: &str) -> MessagePayload {
        MessagePayload {
            part_id: None,
            mime_type: mime_type.to_string(),
            filename: None,
            headers: Vec::new(),
            body: MessageBody {
                attachment_id: None,
                size: 0,
                data: None,
            },
            parts: None,
        }
    }

    fn attachment_part(id: &str, filename: &str, size: u64) -> MessagePayload {
        MessagePayload {
            filename: Some(filename.to_string()),
            body: MessageBody {
                attachment_id: Some(id.to_string()),
                size,
                data: None,
            },
            ..leaf_part("application/octet-stream")
        }
    }

    fn multipart(parts: Vec<MessagePayload>) -> MessagePayload {
        MessagePayload {
            parts: Some(parts),
            ..leaf_part("multipart/mixed")
        }
    }

    fn message_with_payload(id: &str, payload: MessagePayload) -> Message {
        Message {
            id: id.to_string(),
            thread_id: format!("thread-{id}"),
            label_ids: Vec::new(),
            snippet: None,
            history_id: None,
            internal_date: None,
            payload: Some(payload),
            size_estimate: None,
            raw: None,
        }
    }

    #[test]
    fn env_gate_requires_exact_one() {
        assert!(env_flag_is_one(Some("1")));
        assert!(!env_flag_is_one(None));
        assert!(!env_flag_is_one(Some("")));
        assert!(!env_flag_is_one(Some("0")));
        assert!(!env_flag_is_one(Some("true")));
        assert!(!env_flag_is_one(Some(" 1")));
    }

    #[test]
    fn finds_attachment_in_nested_parts() {
        let message = message_with_payload(
            "m1",
            multipart(vec![leaf_part("text/plain"), attachment_part("att1", "a.pdf", 42)]),
        );

        let (_, selected) = select_message_with_attachment(&[message]).expect("attachment expected");
        assert_eq!(selected.attachment_id, "att1");
        assert_eq!(selected.filename, "a.pdf");
        assert_eq!(selected.size, 42);
    }

    #[test]
    fn returns_none_without_attachment_parts() {
        let message = message_with_payload("m1", multipart(vec![leaf_part("text/plain")]));

        assert!(select_message_with_attachment(&[message]).is_none());
    }

    #[test]
    fn skips_messages_until_one_has_attachment() {
        let messages = vec![
            message_with_payload("m1", multipart(vec![leaf_part("text/plain")])),
            message_with_payload("m2", multipart(vec![attachment_part("att2", "b.bin", 7)])),
        ];

        let (message, selected) =
            select_message_with_attachment(&messages).expect("second message expected");
        assert_eq!(message.id, "m2");
        assert_eq!(selected.attachment_id, "att2");
    }

    #[test]
    fn transport_summary_reports_negotiated_version_and_flags() {
        let info = TransportInfo {
            negotiated_version: "HTTP/3".to_string(),
            http3_requested: true,
            http3_effective: true,
            fell_back: false,
        };

        let summary = format_transport_summary(&info);
        assert!(summary.contains("negotiated=HTTP/3"));
        assert!(summary.contains("http3_requested=true"));
        assert!(summary.contains("http3_effective=true"));
        assert!(summary.contains("fell_back=false"));
    }

    #[test]
    fn transport_summary_flags_fallback() {
        let info = TransportInfo {
            negotiated_version: "HTTP/2".to_string(),
            http3_requested: true,
            http3_effective: false,
            fell_back: true,
        };

        let summary = format_transport_summary(&info);
        assert!(summary.contains("negotiated=HTTP/2"));
        assert!(summary.contains("fell_back=true"));
    }

    #[test]
    fn timestamp_millis_is_monotonic_across_calls() {
        let first = timestamp_millis();
        let second = timestamp_millis();
        assert!(second >= first);
    }
}

/// Full live pipeline: search → attachment download → streaming import →
/// trash, reporting the negotiated transport protocol along the way.
#[tokio::test]
#[ignore = "hits the real Gmail API; requires LIVE_E2E=1 plus valid credentials"]
async fn live_full_pipeline() {
    if !env_flag_is_one(std::env::var(LIVE_E2E_ENV).ok().as_deref()) {
        println!("[live-e2e] SKIP: set {LIVE_E2E_ENV}=1 to run against the real API");
        return;
    }

    let mut config = ConfigLoader::load().await.expect("load real config");
    if env_flag_is_one(std::env::var(FORCE_HTTP3_ENV).ok().as_deref()) {
        println!("[transport] {FORCE_HTTP3_ENV}=1: forcing enable_http3 for this run");
        config.performance.enable_http3 = true;
    }

    let token_file = token_path();
    let raw = tokio::fs::read_to_string(&token_file)
        .await
        .unwrap_or_else(|err| panic!("read token file {}: {err}", token_file.display()));
    let storage: TokenStorage =
        serde_json::from_str(&raw).unwrap_or_else(|err| panic!("parse token file: {err}"));
    let refreshable = storage.refresh_token.as_deref().is_some_and(|t| !t.is_empty());
    assert!(
        !storage.is_expired() || refreshable,
        "token at {} is expired without a refresh token; rerun the OAuth flow first",
        token_file.display()
    );

    let auth = GmailAuth::with_token(config.oauth.clone(), storage)
        .await
        .expect("build auth handle");
    let client = GmailClientBuilder::new(config)
        .auth(auth)
        .build()
        .await
        .expect("build client (transport probe fires here)");

    // Fallback is warn-only per spec: recorded loudly, never silently claimed.
    let transport = client.transport_info();
    println!("[transport] {}", format_transport_summary(transport));
    if transport.http3_requested && transport.fell_back {
        println!(
            "[transport] WARN: HTTP/3 requested but fell back to {}; quinn/Gmail compatibility issue suspected",
            transport.negotiated_version
        );
    }

    let inbox_refs = client
        .search(INBOX_QUERY, SEARCH_CANDIDATES)
        .await
        .expect("search in:inbox");
    assert!(!inbox_refs.is_empty(), "expected at least one inbox message");
    println!("[search] {} result(s) for '{INBOX_QUERY}'", inbox_refs.len());

    // Attachment leg: prefer an inbox hit carrying an attachment; widen the
    // query once before declaring the leg skipped. Metadata vectors outlive
    // `selected`, which borrows from them.
    let inbox_metadata = fetch_metadata(&client, &inbox_refs).await;
    let mut selected = select_message_with_attachment(&inbox_metadata);

    let attachment_metadata = if selected.is_none() {
        let attachment_refs = client
            .search(ATTACHMENT_QUERY, SEARCH_CANDIDATES)
            .await
            .expect("search in:inbox has:attachment");
        fetch_metadata(&client, &attachment_refs).await
    } else {
        Vec::new()
    };
    if selected.is_none() {
        selected = select_message_with_attachment(&attachment_metadata);
    }

    match selected {
        Some((message, attachment)) => {
            let target = tempfile::Builder::new()
                .prefix("gmail-e2e-attachment")
                .tempfile()
                .expect("create temp file");
            let written = client
                .download_attachment_to(&message.id, &attachment.attachment_id, target.path())
                .await
                .expect("download attachment to disk");
            assert!(written > 0, "downloaded attachment must not be empty");
            let on_disk = tokio::fs::metadata(target.path())
                .await
                .expect("stat downloaded file");
            assert_eq!(on_disk.len(), written, "file size must match bytes fetched");
            println!(
                "[attachment] {:?} ({} bytes fetched, {} declared) -> {}",
                attachment.filename,
                written,
                attachment.size,
                target.path().display()
            );
        }
        None => println!("[attachment] skipped: no candidate message exposes an attachment"),
    }

    let profile = client.get_profile().await.expect("fetch profile");
    let subject = format!("gmail-opencode live-e2e {}", timestamp_millis());
    let stream = mime_message_stream(
        &profile.email_address,
        &subject,
        "live e2e verification body (no attachments)",
        Vec::new(),
        None,
    );
    let imported = client.import_stream(stream, false).await.expect("import stream");
    assert!(!imported.id.is_empty(), "imported message must have an id");
    println!("[import] message {} subject={subject:?}", imported.id);

    let trashed = client.trash_message(&imported.id).await.expect("trash imported message");
    assert!(
        trashed.label_ids.iter().any(|label| label == "TRASH"),
        "expected TRASH label after trashing, got {:?}",
        trashed.label_ids
    );
    println!("[trash] message {} moved to TRASH", imported.id);

    println!(
        "[live-e2e] PASS negotiated={}",
        client.transport_info().negotiated_version
    );
}

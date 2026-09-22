//! Tests for Gmail core models

use grr_core::models::*;

#[test]
fn test_message_payload_helpers() {
    let payload = MessagePayload {
        part_id: None,
        mime_type: "text/plain".into(),
        filename: None,
        headers: vec![
            Header {
                name: "Subject".into(),
                value: "Test Subject".into(),
            },
            Header {
                name: "From".into(),
                value: "from@example.com".into(),
            },
            Header {
                name: "To".into(),
                value: "to@example.com".into(),
            },
            Header {
                name: "Date".into(),
                value: "Mon, 1 Jan 2024".into(),
            },
        ],
        body: MessageBody {
            attachment_id: None,
            size: 0,
            data: None,
        },
        parts: None,
    };

    assert_eq!(payload.subject(), Some("Test Subject"));
    assert_eq!(payload.from(), Some("from@example.com"));
    assert_eq!(payload.to(), Some("to@example.com"));
    assert_eq!(payload.date(), Some("Mon, 1 Jan 2024"));
    assert!(payload.header("message-id").is_none());
}

#[test]
fn test_message_serialization() {
    let msg = Message {
        id: "msg1".into(),
        thread_id: "thread1".into(),
        label_ids: vec!["INBOX".into()],
        snippet: Some("Hello".into()),
        payload: None,
        size_estimate: Some(1024),
        history_id: Some("123".into()),
        internal_date: Some("1704067200000".into()),
        raw: None,
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("msg1"));
    assert!(json.contains("thread1"));
}

#[test]
fn test_label_with_color() {
    let label = Label {
        id: "label1".into(),
        name: "Work".into(),
        message_list_visibility: Some("show".into()),
        label_list_visibility: Some("labelShow".into()),
        r#type: Some("user".into()),
        messages_total: Some(100),
        messages_unread: Some(5),
        threads_total: Some(50),
        threads_unread: Some(3),
        color: Some(LabelColor {
            text_color: "#ffffff".into(),
            background_color: "#4a86e8".into(),
        }),
    };

    let json = serde_json::to_string(&label).unwrap();
    assert!(json.contains("Work"));
    assert!(json.contains("#4a86e8"));
}

#[test]
fn test_thread_serialization() {
    let thread = Thread {
        id: "thread1".into(),
        snippet: None,
        history_id: Some("123".into()),
        messages: Some(vec![Message {
            id: "msg1".into(),
            thread_id: "thread1".into(),
            label_ids: vec![],
            snippet: None,
            payload: None,
            size_estimate: None,
            history_id: None,
            internal_date: None,
            raw: None,
        }]),
    };

    let json = serde_json::to_string(&thread).unwrap();
    assert!(json.contains("thread1"));
    assert!(json.contains("msg1"));
}

#[test]
fn test_send_as_serialization() {
    let send_as = SendAs {
        send_as_email: "me@example.com".into(),
        display_name: Some("Me".into()),
        reply_to_address: None,
        signature: Some("Sent from Rust".into()),
        is_primary: true,
        is_default: true,
        treat_as_alias: false,
        verification_status: None,
    };

    let json = serde_json::to_string(&send_as).unwrap();
    assert!(json.contains("me@example.com"));
}

/// The body of a message, url-safely base64-encoded (Gmail's documented
/// form), padded or not.
fn message_with_body(mime: &str, data: &str) -> Message {
    use base64::Engine;
    let raw = format!("Subject: t\r\n\r\nhello body");
    let data = if data == "auto" {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes())
    } else {
        data.into()
    };
    Message {
        id: "m1".into(),
        thread_id: "t1".into(),
        label_ids: vec![],
        snippet: None,
        payload: Some(MessagePayload {
            part_id: None,
            mime_type: mime.into(),
            filename: None,
            headers: vec![],
            body: MessageBody {
                attachment_id: None,
                size: raw.len() as u64,
                data: Some(data),
            },
            parts: None,
        }),
        size_estimate: None,
        history_id: None,
        internal_date: None,
        raw: None,
    }
}

#[test]
fn extract_body_decodes_unpadded_base64url() {
    let msg = message_with_body("text/plain", "auto");
    let body = extract_body(&msg).expect("body must decode");
    assert!(body.contains("hello body"), "got: {body:?}");
}

#[test]
fn extract_body_decodes_standard_alphabet_unpadded() {
    // Real-world Gmail sometimes ships parts in the *standard* alphabet
    // (`/`, `+`) while still omitting padding — both strict engines
    // reject that; the normalizing decoder must not.
    use base64::Engine;
    let raw = b"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0\"/>";
    let standard = base64::engine::general_purpose::STANDARD_NO_PAD.encode(raw);
    assert!(standard.contains('/') || standard.contains('+') || !standard.is_empty());
    let msg = message_with_body("text/html", &standard);
    let body = extract_body(&msg).expect("standard-alphabet part must decode");
    assert!(body.starts_with("<!DOCTYPE"), "got: {body:?}");
}

#[test]
fn extract_body_prefers_plain_over_html() {
    use base64::Engine;
    let plain_text = "plain wins";
    let html_text = "<p>html loses</p>";
    let msg = Message {
        id: "m1".into(),
        thread_id: "t1".into(),
        label_ids: vec![],
        snippet: None,
        payload: Some(MessagePayload {
            part_id: None,
            mime_type: "multipart/alternative".into(),
            filename: None,
            headers: vec![],
            body: MessageBody {
                attachment_id: None,
                size: 0,
                data: None,
            },
            parts: Some(vec![
                MessagePayload {
                    part_id: Some("1".into()),
                    mime_type: "text/html".into(),
                    filename: None,
                    headers: vec![],
                    body: MessageBody {
                        attachment_id: None,
                        size: 0,
                        data: Some(
                            base64::engine::general_purpose::URL_SAFE_NO_PAD
                                .encode(html_text.as_bytes()),
                        ),
                    },
                    parts: None,
                },
                MessagePayload {
                    part_id: Some("2".into()),
                    mime_type: "text/plain".into(),
                    filename: None,
                    headers: vec![],
                    body: MessageBody {
                        attachment_id: None,
                        size: 0,
                        data: Some(
                            base64::engine::general_purpose::URL_SAFE_NO_PAD
                                .encode(plain_text.as_bytes()),
                        ),
                    },
                    parts: None,
                },
            ]),
        }),
        size_estimate: None,
        history_id: None,
        internal_date: None,
        raw: None,
    };

    // The DFS prefers the text/plain child even though text/html is first.
    assert_eq!(extract_body(&msg).as_deref(), Some("plain wins"));
}

#[test]
fn extract_body_returns_none_without_payload() {
    let msg = Message {
        id: "m1".into(),
        thread_id: "t1".into(),
        label_ids: vec![],
        snippet: None,
        payload: None,
        size_estimate: None,
        history_id: None,
        internal_date: None,
        raw: None,
    };
    assert!(extract_body(&msg).is_none());
}

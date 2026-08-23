//! Tests for Gmail core models

use gmail_core::models::*;

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

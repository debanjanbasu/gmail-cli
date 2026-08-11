//! Tests for Gmail core models

use gmail_core::models::*;
use serde_json;

#[test]
fn test_email_message_serialization() {
    let msg = EmailMessage {
        id: "msg1".into(),
        thread_id: "thread1".into(),
        subject: Some("Test".into()),
        from: Some("from@example.com".into()),
        to: Some("to@example.com".into()),
        date: Some("Mon, 1 Jan 2024".into()),
        snippet: Some("Hello".into()),
        labels: Some(vec!["INBOX".into()]),
        size_estimate: Some(1024),
        internal_date: Some("1704067200000".into()),
    };
    
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("msg1"));
    assert!(json.contains("thread1"));
}

#[test]
fn test_thread_message_with_attachments() {
    let msg = ThreadMessage {
        id: "msg1".into(),
        thread_id: "thread1".into(),
        subject: Some("Test".into()),
        from: Some("from@example.com".into()),
        to: Some("to@example.com".into()),
        date: Some("Mon, 1 Jan 2024".into()),
        body: Some("Body text".into()),
        labels: Some(vec!["INBOX".into()]),
        size_estimate: Some(2048),
        internal_date: Some("1704067200000".into()),
        attachments: Some(vec![AttachmentInfo {
            attachment_id: "att1".into(),
            filename: "test.pdf".into(),
            mime_type: "application/pdf".into(),
            size: 1024,
        }]),
    };
    
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("test.pdf"));
    assert!(json.contains("application/pdf"));
}

#[test]
fn test_label_info_with_color() {
    let label = LabelInfo {
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
fn test_history_record() {
    let history = HistoryRecord {
        id: "hist1".into(),
        messages: Some(vec![EmailMessage {
            id: "msg1".into(),
            thread_id: "thread1".into(),
            subject: None,
            from: None,
            to: None,
            date: None,
            snippet: None,
            labels: None,
            size_estimate: None,
            internal_date: None,
        }]),
        labels_added: Some(vec!["IMPORTANT".into()]),
        labels_removed: Some(vec!["UNREAD".into()]),
    };
    
    let json = serde_json::to_string(&history).unwrap();
    assert!(json.contains("IMPORTANT"));
    assert!(json.contains("UNREAD"));
}
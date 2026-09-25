#![cfg(feature = "gmail")]
use grr_cli::core::{GoogleAuth, GrrConfig, GrrError, TokenStorage};
use grr_cli::gmail::{
    AutoForwarding, Filter, FilterAction, FilterCriteria, ForwardingAddress, GmailClient,
    GmailClientBuilder, ImapSettings, PopSettings, UpdateLabelOptions, UpdateSendAsOptions,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn filter_json(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "criteria": {
            "from": "sender@example.com",
            "to": "me@example.com",
            "subject": "receipt",
            "query": "has:attachment",
            "hasAttachment": true,
            "excludeChats": true,
            "size": 1024,
            "sizeComparison": "larger"
        },
        "action": {
            "addLabelIds": ["Receipts"],
            "removeLabelIds": ["INBOX"],
            "forward": "archive@example.com"
        }
    })
}

fn request_body(request: &Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("request body must be JSON")
}

async fn test_client(base: &str) -> GmailClient {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after the Unix epoch")
        .as_secs();
    let auth = GoogleAuth::with_token(
        GrrConfig::default().oauth,
        TokenStorage {
            access_token: "test-token".into(),
            refresh_token: None,
            expires_at: now + 3600,
            token_type: "Bearer".into(),
            scope: "test-scope".into(),
        },
    )
    .await
    .expect("auth handle");
    GmailClientBuilder::new()
        .auth(auth)
        .base_url(base.parse().expect("base URL"))
        .upload_base_url(format!("{base}/").parse().expect("upload base URL"))
        .build()
        .await
        .expect("Gmail client")
}

#[tokio::test]
async fn filters_use_documented_create_list_get_and_delete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/settings/filters"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "filter": [filter_json("filter-list")]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/me/settings/filters/filter-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(filter_json("filter-1")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/users/me/settings/filters"))
        .respond_with(ResponseTemplate::new(200).set_body_json(filter_json("filter-created")))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/users/me/settings/filters/filter-1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let listed = client.list_filters().await.expect("list filters");
    let fetched = client.get_filter("filter-1").await.expect("get filter");
    let created = client
        .create_filter(Filter {
            criteria: FilterCriteria {
                from: Some("sender@example.com".into()),
                to: Some("me@example.com".into()),
                subject: Some("receipt".into()),
                query: Some("has:attachment".into()),
                has_attachment: Some(true),
                exclude_chats: Some(true),
                size: Some(1024),
                size_comparison: Some("larger".into()),
                negated_query: None,
            },
            action: FilterAction {
                add_label_ids: Some(vec!["Receipts".into()]),
                remove_label_ids: Some(vec!["INBOX".into()]),
                forward: Some("archive@example.com".into()),
            },
            ..Filter::default()
        })
        .await
        .expect("create filter");
    client
        .delete_filter("filter-1")
        .await
        .expect("delete filter");

    assert_eq!(listed.len(), 1);
    assert_eq!(fetched.id, "filter-1");
    assert_eq!(created.id, "filter-created");

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[0].url.path(), "/users/me/settings/filters");
    assert_eq!(requests[1].method.as_str(), "GET");
    assert_eq!(
        requests[1].url.path(),
        "/users/me/settings/filters/filter-1"
    );
    assert_eq!(requests[2].method.as_str(), "POST");
    assert_eq!(
        request_body(&requests[2]),
        serde_json::json!({
            "criteria": {
                "from": "sender@example.com",
                "to": "me@example.com",
                "subject": "receipt",
                "query": "has:attachment",
                "hasAttachment": true,
                "excludeChats": true,
                "size": 1024,
                "sizeComparison": "larger"
            },
            "action": {
                "addLabelIds": ["Receipts"],
                "removeLabelIds": ["INBOX"],
                "forward": "archive@example.com"
            }
        })
    );
    assert_eq!(requests[3].method.as_str(), "DELETE");
    assert_eq!(
        requests[3].url.path(),
        "/users/me/settings/filters/filter-1"
    );
}

#[tokio::test]
async fn forwarding_addresses_use_requested_contract() {
    let server = MockServer::start().await;
    let address = serde_json::json!({
        "forwardingEmail": "forward@example.com",
        "verificationStatus": "pending"
    });
    Mock::given(method("GET"))
        .and(path("/users/me/settings/forwardingAddresses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "forwardingAddresses": [address]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/users/me/settings/forwardingAddresses/forward@example.com",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(address.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/users/me/settings/forwardingAddresses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(address.clone()))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(
            "/users/me/settings/forwardingAddresses/forward@example.com",
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let listed = client
        .list_forwarding_addresses()
        .await
        .expect("list forwarding addresses");
    let fetched = client
        .get_forwarding_address("forward@example.com")
        .await
        .expect("get forwarding address");
    let created = client
        .create_forwarding_address(ForwardingAddress {
            forwarding_email: "forward@example.com".into(),
            verification_status: None,
        })
        .await
        .expect("create forwarding address");
    client
        .delete_forwarding_address("forward@example.com")
        .await
        .expect("delete forwarding address");

    assert_eq!(listed[0].forwarding_email, "forward@example.com");
    assert_eq!(fetched.verification_status.as_deref(), Some("pending"));
    assert_eq!(created.forwarding_email, "forward@example.com");

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[1].method.as_str(), "GET");
    assert_eq!(requests[2].method.as_str(), "POST");
    assert_eq!(
        request_body(&requests[2]),
        serde_json::json!({
            "forwardingEmail": "forward@example.com"
        })
    );
    assert_eq!(requests[3].method.as_str(), "DELETE");
}

#[tokio::test]
async fn auto_forwarding_get_and_put_use_requested_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/settings/autoForwarding"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enabled": true,
            "emailAddress": "forward@example.com",
            "disposition": "archive"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/users/me/settings/autoForwarding"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enabled": true,
            "emailAddress": "forward@example.com",
            "disposition": "trash"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let settings = client
        .get_auto_forwarding()
        .await
        .expect("get auto-forwarding");
    let updated = client
        .update_auto_forwarding(AutoForwarding {
            enabled: true,
            email_address: "forward@example.com".into(),
            disposition: Some("trash".into()),
        })
        .await
        .expect("update auto-forwarding");

    assert!(settings.enabled);
    assert_eq!(settings.disposition.as_deref(), Some("archive"));
    assert_eq!(updated.disposition.as_deref(), Some("trash"));

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[1].method.as_str(), "PUT");
    assert_eq!(
        request_body(&requests[1]),
        serde_json::json!({
            "enabled": true,
            "emailAddress": "forward@example.com",
            "disposition": "trash"
        })
    );
}

#[tokio::test]
async fn pop_get_and_put_use_requested_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/settings/pop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accessWindow": "fromNowOn",
            "disposition": "leaveInInbox"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/users/me/settings/pop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accessWindow": "allMail",
            "disposition": "archive"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let settings = client.get_pop_settings().await.expect("get POP");
    let updated = client
        .update_pop_settings(PopSettings {
            access_window: Some("allMail".into()),
            disposition: Some("archive".into()),
        })
        .await
        .expect("update POP");

    assert_eq!(settings.access_window.as_deref(), Some("fromNowOn"));
    assert_eq!(settings.disposition.as_deref(), Some("leaveInInbox"));
    assert_eq!(updated.access_window.as_deref(), Some("allMail"));
    assert_eq!(updated.disposition.as_deref(), Some("archive"));

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[1].method.as_str(), "PUT");
    assert_eq!(
        request_body(&requests[1]),
        serde_json::json!({
            "accessWindow": "allMail",
            "disposition": "archive"
        })
    );
}

#[tokio::test]
async fn imap_get_and_put_use_requested_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/settings/imap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enabled": true,
            "autoExpunge": false,
            "expungeBehavior": "archive",
            "maxFolderSize": 1024
        })))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/users/me/settings/imap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enabled": false,
            "autoExpunge": true,
            "expungeBehavior": "deleteForever",
            "maxFolderSize": 2048
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let settings = client.get_imap_settings().await.expect("get IMAP");
    let updated = client
        .update_imap_settings(ImapSettings {
            enabled: false,
            auto_expunge: Some(true),
            expunge_behavior: Some("deleteForever".into()),
            max_folder_size: Some(2048),
        })
        .await
        .expect("update IMAP");

    assert!(settings.enabled);
    assert_eq!(settings.auto_expunge, Some(false));
    assert_eq!(settings.expunge_behavior.as_deref(), Some("archive"));
    assert_eq!(settings.max_folder_size, Some(1024));
    assert!(!updated.enabled);

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method.as_str(), "GET");
    assert_eq!(requests[1].method.as_str(), "PUT");
    assert_eq!(
        request_body(&requests[1]),
        serde_json::json!({
            "enabled": false,
            "autoExpunge": true,
            "expungeBehavior": "deleteForever",
            "maxFolderSize": 2048
        })
    );
}

#[tokio::test]
async fn settings_enum_validation_lists_allowed_values_without_requests() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri()).await;

    let error = client
        .update_pop_settings(PopSettings {
            access_window: Some("newMail".into()),
            disposition: None,
        })
        .await
        .expect_err("invalid access window");
    assert!(matches!(
        error,
        GrrError::InvalidArgument(message)
            if message.contains("accessWindowUnspecified, disabled, fromNowOn, allMail")
    ));

    let error = client
        .update_auto_forwarding(AutoForwarding {
            disposition: Some("delete".into()),
            ..AutoForwarding::default()
        })
        .await
        .expect_err("invalid disposition");
    assert!(matches!(
        error,
        GrrError::InvalidArgument(message)
            if message.contains(
                "dispositionUnspecified, leaveInInbox, archive, trash, markRead"
            )
    ));

    let error = client
        .update_imap_settings(ImapSettings {
            expunge_behavior: Some("delete".into()),
            ..ImapSettings::default()
        })
        .await
        .expect_err("invalid expunge behavior");
    assert!(matches!(
        error,
        GrrError::InvalidArgument(message)
            if message.contains("expungeBehaviorUnspecified, archive, trash, deleteForever")
    ));

    assert!(
        server
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
}

#[tokio::test]
async fn thread_list_coverage_uses_official_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/threads"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "threads": [{"id": "thread-1", "snippet": "hello"}],
            "resultSizeEstimate": 1
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let threads = client
        .list_threads("in:inbox", 10)
        .await
        .expect("list threads");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].id, "thread-1");
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests[0].method.as_str(), "GET");
    assert!(requests[0].url.query_pairs().any(|(key, value)| {
        key == "q"
            && value == "in:inbox"
            && requests[0]
                .url
                .query_pairs()
                .any(|pair| pair == ("maxResults".into(), "10".into()))
    }));
}

#[tokio::test]
async fn send_as_verify_coverage_uses_official_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/me/settings/sendAs/alias@example.com/verify"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sendAsEmail": "alias@example.com",
            "isPrimary": false,
            "isDefault": false,
            "treatAsAlias": true,
            "verificationStatus": "accepted"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let send_as = client
        .verify_send_as("alias@example.com")
        .await
        .expect("verify send-as");

    assert_eq!(send_as.verification_status.as_deref(), Some("accepted"));
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests[0].method.as_str(), "POST");
}

#[tokio::test]
async fn label_and_send_as_patch_coverage_uses_official_endpoints() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/users/me/labels/label-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "label-1",
            "name": "Receipts"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/users/me/settings/sendAs/alias@example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sendAsEmail": "alias@example.com",
            "signature": "Sent from Rust"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let label = client
        .patch_label(
            "label-1",
            UpdateLabelOptions {
                name: Some("Receipts".into()),
                ..UpdateLabelOptions::default()
            },
        )
        .await
        .expect("patch label");
    let send_as = client
        .patch_send_as(
            "alias@example.com",
            UpdateSendAsOptions {
                signature: Some("Sent from Rust".into()),
                ..UpdateSendAsOptions::default()
            },
        )
        .await
        .expect("patch send-as");

    assert_eq!(label.name, "Receipts");
    assert_eq!(send_as.signature.as_deref(), Some("Sent from Rust"));
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|request| request.method.as_str() == "PATCH")
    );
    assert_eq!(
        request_body(&requests[0]),
        serde_json::json!({"name": "Receipts"})
    );
    assert_eq!(
        request_body(&requests[1]),
        serde_json::json!({"signature": "Sent from Rust"})
    );
}

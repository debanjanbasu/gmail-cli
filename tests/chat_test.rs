#![cfg(feature = "chat")]
//! Wiremock tests for the Chat client over the shared HTTP core.

use grr_cli::chat::{ChatClient, ChatClientBuilder};
use grr_cli::core::{GoogleAuth, GrrConfig, TokenStorage};
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_spaces_follows_page_tokens() {
    let server = MockServer::start().await;

    // Page 1: no pageToken in the query string yet.
    Mock::given(method("GET"))
        .and(path("/spaces"))
        .and(query_param_is_missing("pageToken"))
        .and(query_param("pageSize", "1000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spaces": [
                { "name": "spaces/AAA", "displayName": "Team chat", "type": "ROOM" },
                { "name": "spaces/BBB", "displayName": "DM with Boss", "type": "DM" }
            ],
            "nextPageToken": "page-2"
        })))
        .mount(&server)
        .await;

    // Page 2: fetched with pageToken=page-2, no further pages.
    Mock::given(method("GET"))
        .and(path("/spaces"))
        .and(query_param("pageToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spaces": [ { "name": "spaces/CCC", "displayName": "Release room", "type": "SPACE" } ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let spaces = client.list_spaces(None).await.unwrap();

    let names: Vec<&str> = spaces
        .iter()
        .map(|s| s.name.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(names, ["spaces/AAA", "spaces/BBB", "spaces/CCC"]);
    assert_eq!(spaces[0].display_name.as_deref(), Some("Team chat"));
    assert_eq!(spaces[0].type_.as_deref(), Some("ROOM"));
    assert_eq!(spaces[1].type_.as_deref(), Some("DM"));
    assert_eq!(spaces[2].type_.as_deref(), Some("SPACE"));
}

#[tokio::test]
async fn get_space_accepts_bare_ids() {
    let server = MockServer::start().await;
    // A bare "AAA" must hit the same resource as "spaces/AAA".
    Mock::given(method("GET"))
        .and(path("/spaces/AAA"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/AAA",
            "type": "ROOM",
            "spaceType": "SPACE",
            "displayName": "Team chat",
            "memberCount": 7,
            "lastActiveTime": "2026-09-24T09:00:00Z",
            "threadingState": "THREADED_MESSAGES"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let space = client.get_space("AAA").await.unwrap();

    assert_eq!(space.name.as_deref(), Some("spaces/AAA"));
    assert_eq!(space.display_name.as_deref(), Some("Team chat"));
    assert_eq!(space.type_.as_deref(), Some("ROOM"));
    assert_eq!(space.space_type.as_deref(), Some("SPACE"));
    assert_eq!(space.member_count, Some(7));
    assert_eq!(
        space.last_active_time.as_deref(),
        Some("2026-09-24T09:00:00Z")
    );
    assert_eq!(space.threading_state.as_deref(), Some("THREADED_MESSAGES"));
}

#[tokio::test]
async fn list_messages_parses_one_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/spaces/BBB/messages"))
        .and(query_param("pageSize", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "messages": [
                {
                    "name": "spaces/BBB/messages/msg-1",
                    "sender": { "name": "users/123", "displayName": "Boss", "type": "HUMAN" },
                    "createTime": "2026-09-24T10:00:00Z",
                    "text": "Ship grr-chat",
                    "thread": { "name": "spaces/BBB/threads/t-1" },
                    "deleted": false,
                    "formattedTime": "Sep 24, 2026, 10:00 AM",
                    "argumentText": "Ship grr-chat"
                },
                {
                    "name": "spaces/BBB/messages/msg-2",
                    "sender": { "name": "users/456", "type": "BOT", "isAnonymous": false },
                    "createTime": "2026-09-24T10:01:00Z",
                    "text": "CI is green",
                    "deleted": false,
                    "formattedTime": "Sep 24, 2026, 10:01 AM",
                    "argumentText": "CI is green"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let messages = client.list_messages("spaces/BBB", Some(10)).await.unwrap();

    assert_eq!(messages.len(), 2);
    let first = &messages[0];
    assert_eq!(first.name.as_deref(), Some("spaces/BBB/messages/msg-1"));
    assert_eq!(first.create_time.as_deref(), Some("2026-09-24T10:00:00Z"));
    assert_eq!(first.text.as_deref(), Some("Ship grr-chat"));
    assert_eq!(first.deleted, Some(false));
    assert_eq!(
        first.formatted_time.as_deref(),
        Some("Sep 24, 2026, 10:00 AM")
    );
    assert_eq!(first.argument_text.as_deref(), Some("Ship grr-chat"));

    let sender = first.sender.as_ref().unwrap();
    assert_eq!(sender.name.as_deref(), Some("users/123"));
    assert_eq!(sender.display_name.as_deref(), Some("Boss"));
    assert_eq!(sender.type_.as_deref(), Some("HUMAN"));

    let thread = first.thread.as_ref().unwrap();
    assert_eq!(thread.name.as_deref(), Some("spaces/BBB/threads/t-1"));
    assert_eq!(thread.thread_key, None);

    let second = &messages[1];
    assert_eq!(second.name.as_deref(), Some("spaces/BBB/messages/msg-2"));
    let bot = second.sender.as_ref().unwrap();
    assert_eq!(bot.type_.as_deref(), Some("BOT"));
    assert_eq!(bot.is_anonymous, Some(false));
}

#[tokio::test]
async fn send_message_posts_text_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/spaces/CCC/messages"))
        .and(body_json(serde_json::json!({ "text": "Hello from grr" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/CCC/messages/new-1",
            "sender": { "name": "users/123", "displayName": "Debanjan", "type": "HUMAN" },
            "createTime": "2026-09-24T11:00:00Z",
            "text": "Hello from grr",
            "thread": { "name": "spaces/CCC/threads/new-1" },
            "deleted": false,
            "formattedTime": "Sep 24, 2026, 11:00 AM",
            "argumentText": "Hello from grr"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let message = client
        .send_message("spaces/CCC", "Hello from grr")
        .await
        .unwrap();

    assert_eq!(message.name.as_deref(), Some("spaces/CCC/messages/new-1"));
    assert_eq!(message.text.as_deref(), Some("Hello from grr"));
    assert_eq!(message.create_time.as_deref(), Some("2026-09-24T11:00:00Z"));
    assert_eq!(message.deleted, Some(false));
    assert_eq!(
        message.sender.as_ref().and_then(|s| s.display_name.clone()),
        Some("Debanjan".into())
    );
}

/// Build a client pointed at the mock server with a pre-authed, in-memory
/// token so no OAuth flow or token-endpoint round-trip ever happens.
/// Both h3 and h2 prior-knowledge are disabled: wiremock speaks HTTP/1.1.
async fn test_client(base: &str) -> ChatClient {
    let config = GrrConfig::default();
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
    // Keep token persistence (if a test ever refreshes) off the real
    // credential store: it goes to a tempdir file instead.
    let auth = GoogleAuth::with_token(config.oauth.clone(), storage)
        .await
        .unwrap()
        .with_token_path(tempfile::tempdir().unwrap().keep());
    ChatClientBuilder::new()
        .auth(auth)
        .base_url(base.parse().unwrap())
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn setup_space_posts_realistic_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/spaces:setup"))
        .and(body_json(serde_json::json!({
            "space": {
                "spaceType": "SPACE",
                "displayName": "Launch room",
                "description": "Release coordination"
            },
            "memberships": [
                { "member": { "name": "users/123" } },
                { "member": { "name": "users/456" } }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/AAA",
            "displayName": "Launch room",
            "description": "Release coordination",
            "spaceType": "SPACE",
            "createTime": "2026-09-25T10:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let space = client
        .setup_space(
            "Launch room",
            Some("Release coordination"),
            ["users/123", "users/456"],
        )
        .await
        .unwrap();

    assert_eq!(space.name.as_deref(), Some("spaces/AAA"));
    assert_eq!(space.space_type.as_deref(), Some("SPACE"));
    assert_eq!(space.create_time.as_deref(), Some("2026-09-25T10:00:00Z"));
}

#[tokio::test]
async fn patch_space_sends_update_mask_and_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/spaces/AAA"))
        .and(query_param("updateMask", "displayName,description"))
        .and(body_json(serde_json::json!({
            "displayName": "Renamed room",
            "description": "Updated description"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/AAA",
            "displayName": "Renamed room",
            "description": "Updated description"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let space = client
        .patch_space("AAA", Some("Renamed room"), Some("Updated description"))
        .await
        .unwrap();

    assert_eq!(space.display_name.as_deref(), Some("Renamed room"));
    assert_eq!(space.description.as_deref(), Some("Updated description"));
}

#[tokio::test]
async fn delete_space_calls_delete_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/spaces/AAA"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    client.delete_space("spaces/AAA").await.unwrap();
}

#[tokio::test]
async fn membership_endpoints_list_add_get_and_delete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/spaces/AAA/members"))
        .and(query_param("pageSize", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "memberships": [{
                "name": "spaces/AAA/members/users/123",
                "createTime": "2026-09-25T10:00:00Z",
                "member": {
                    "name": "users/123",
                    "displayName": "Debanjan",
                    "type": "HUMAN",
                    "domainId": "example.com"
                },
                "role": "ROLE_MANAGER",
                "state": "JOINED"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/spaces/AAA/members/users/123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/AAA/members/users/123",
            "member": { "name": "users/123", "type": "HUMAN" },
            "state": "JOINED"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/spaces/AAA/members"))
        .and(body_json(serde_json::json!({
            "member": { "name": "users/123" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/AAA/members/users/123",
            "member": { "name": "users/123", "type": "HUMAN" },
            "state": "INVITED"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/spaces/AAA/members/users/123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let memberships = client.list_memberships("AAA", Some(25)).await.unwrap();
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].role.as_deref(), Some("ROLE_MANAGER"));

    let membership = client
        .get_membership("AAA", "memberships/users/123")
        .await
        .unwrap();
    assert_eq!(membership.state.as_deref(), Some("JOINED"));

    let created = client.create_membership("AAA", "users/123").await.unwrap();
    assert_eq!(created.state.as_deref(), Some("INVITED"));
    client
        .delete_membership("AAA", "spaces/AAA/memberships/users/123")
        .await
        .unwrap();
}

#[tokio::test]
async fn reaction_endpoints_list_create_and_delete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/spaces/AAA/messages/m1/reactions"))
        .and(query_param("pageSize", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "reactions": [{
                "name": "spaces/AAA/messages/m1/reactions/r1",
                "emoji": { "unicode": "1F44D" }
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/spaces/AAA/messages/m1/reactions"))
        .and(body_json(serde_json::json!({
            "emoji": { "unicode": "1F44D" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "spaces/AAA/messages/m1/reactions/r1",
            "emoji": { "unicode": "1F44D" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/spaces/AAA/messages/m1/reactions/r1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let reactions = client.list_reactions("AAA", "m1", Some(25)).await.unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(
        reactions[0]
            .emoji
            .as_ref()
            .and_then(|emoji| emoji.unicode.as_deref()),
        Some("1F44D")
    );

    let created = client.create_reaction("AAA", "m1", "👍").await.unwrap();
    assert_eq!(
        created.name.as_deref(),
        Some("spaces/AAA/messages/m1/reactions/r1")
    );
    client
        .delete_reaction("AAA", "spaces/AAA/messages/m1/reactions/r1")
        .await
        .unwrap();
}

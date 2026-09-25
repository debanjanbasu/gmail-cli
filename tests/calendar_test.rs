#![cfg(feature = "calendar")]
//! Wiremock tests for the Calendar client over the shared HTTP core.

use grr_cli::calendar::{
    AclPatch, AclRule, AclScope, Calendar, CalendarClient, CalendarClientBuilder,
    CalendarListPatch, CalendarPatch, Event, EventDateTime, EventInstancesOptions,
    EventListOptions, EventPatch,
};
use grr_cli::core::{GoogleAuth, GrrConfig, TokenStorage};
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_events_parses_one_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "summary": "debanjanbasu2006@gmail.com",
            "timeZone": "UTC",
            "items": [
                {
                    "id": "evt1",
                    "summary": "Team sync",
                    "location": "Room 4",
                    "description": "Weekly sync",
                    "status": "confirmed",
                    "hangoutLink": "https://meet.google.com/abc-defg",
                    "htmlLink": "https://calendar.google.com/r?eid=abc",
                    "start": { "dateTime": "2026-09-24T10:00:00Z", "timeZone": "UTC" },
                    "end": { "dateTime": "2026-09-24T11:00:00Z", "timeZone": "UTC" },
                    "recurringEventId": null,
                    "creator": { "email": "boss@example.com", "displayName": "Boss", "self": true },
                    "organizer": { "email": "team@example.com", "self": false },
                    "attendees": [
                        { "email": "me@example.com", "responseStatus": "accepted", "self": true },
                        { "email": "peer@example.com", "responseStatus": "needsAction" }
                    ]
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let events = client
        .list_events("primary", EventListOptions::default())
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.id.as_deref(), Some("evt1"));
    assert_eq!(event.summary.as_deref(), Some("Team sync"));
    assert_eq!(event.location.as_deref(), Some("Room 4"));
    assert_eq!(event.description.as_deref(), Some("Weekly sync"));
    assert_eq!(event.status.as_deref(), Some("confirmed"));
    assert_eq!(
        event.hangout_link.as_deref(),
        Some("https://meet.google.com/abc-defg")
    );
    assert_eq!(
        event.html_link.as_deref(),
        Some("https://calendar.google.com/r?eid=abc")
    );
    assert_eq!(event.recurring_event_id, None);

    let start = event.start.as_ref().unwrap();
    assert_eq!(start.date, None);
    assert_eq!(start.date_time.as_deref(), Some("2026-09-24T10:00:00Z"));
    assert_eq!(start.time_zone.as_deref(), Some("UTC"));
    let end = event.end.as_ref().unwrap();
    assert_eq!(end.date_time.as_deref(), Some("2026-09-24T11:00:00Z"));

    let creator = event.creator.as_ref().unwrap();
    assert_eq!(creator.email.as_deref(), Some("boss@example.com"));
    assert_eq!(creator.display_name.as_deref(), Some("Boss"));
    assert_eq!(creator.is_self, Some(true));
    let organizer = event.organizer.as_ref().unwrap();
    assert_eq!(organizer.email.as_deref(), Some("team@example.com"));

    let attendees = event.attendees.as_ref().unwrap();
    assert_eq!(attendees.len(), 2);
    assert_eq!(attendees[0].email.as_deref(), Some("me@example.com"));
    assert_eq!(attendees[0].response_status.as_deref(), Some("accepted"));
    assert_eq!(attendees[1].response_status.as_deref(), Some("needsAction"));
}

#[tokio::test]
async fn list_events_follows_page_tokens() {
    let server = MockServer::start().await;

    // Page 1: no pageToken in the query string yet.
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "nextPageToken": "page-2",
            "items": [ { "id": "evt1" }, { "id": "evt2" } ]
        })))
        .mount(&server)
        .await;

    // Page 2: fetched with pageToken=page-2, no further pages.
    Mock::given(method("GET"))
        .and(path("/calendars/primary/events"))
        .and(query_param("pageToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [ { "id": "evt3" } ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let events = client
        .list_events(
            "primary",
            EventListOptions {
                max_results: Some(3),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let ids: Vec<&str> = events
        .iter()
        .map(|e| e.id.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(ids, ["evt1", "evt2", "evt3"]);
}

#[tokio::test]
async fn create_event_posts_event_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "newevt",
            "summary": "Ship grr-calendar",
            "status": "confirmed",
            "start": { "dateTime": "2026-09-24T09:00:00Z" },
            "end": { "dateTime": "2026-09-24T10:00:00Z" },
            "htmlLink": "https://calendar.google.com/r?eid=new"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let event = Event {
        summary: Some("Ship grr-calendar".into()),
        start: Some(EventDateTime {
            date_time: Some("2026-09-24T09:00:00Z".into()),
            ..Default::default()
        }),
        end: Some(EventDateTime {
            date_time: Some("2026-09-24T10:00:00Z".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let created = client.create_event("primary", event).await.unwrap();
    assert_eq!(created.id.as_deref(), Some("newevt"));
    assert_eq!(created.status.as_deref(), Some("confirmed"));
    assert_eq!(created.summary.as_deref(), Some("Ship grr-calendar"));
    assert_eq!(
        created
            .start
            .as_ref()
            .and_then(|s| s.date_time.clone())
            .as_deref(),
        Some("2026-09-24T09:00:00Z")
    );
}

#[tokio::test]
async fn delete_event_accepts_204_no_content() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/calendars/primary/events/evt1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    // 204 carries no body: the client must not try to parse one.
    client.delete_event("primary", "evt1").await.unwrap();
}

#[tokio::test]
async fn list_calendars_parses_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/calendarList"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                {
                    "id": "primary",
                    "summary": "debanjanbasu2006@gmail.com",
                    "timeZone": "Asia/Kolkata",
                    "accessRole": "owner",
                    "primary": true
                },
                {
                    "id": "en.usa#holiday@group.v.calendar.google.com",
                    "summary": "Holidays in United States",
                    "accessRole": "reader",
                    "selected": true
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let calendars = client.list_calendars(Some(10)).await.unwrap();
    assert_eq!(calendars.len(), 2);
    assert_eq!(calendars[0].id, "primary");
    assert_eq!(
        calendars[0].summary.as_deref(),
        Some("debanjanbasu2006@gmail.com")
    );
    assert_eq!(calendars[0].time_zone.as_deref(), Some("Asia/Kolkata"));
    assert_eq!(calendars[0].access_role.as_deref(), Some("owner"));
    assert_eq!(calendars[0].primary, Some(true));
    assert_eq!(
        calendars[1].id,
        "en.usa#holiday@group.v.calendar.google.com"
    );
    assert_eq!(calendars[1].access_role.as_deref(), Some("reader"));
    assert_eq!(calendars[1].selected, Some(true));
}

#[tokio::test]
async fn freebusy_maps_calendar_busy_periods() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/freeBusy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "timeMin": "2026-09-24T00:00:00Z",
            "timeMax": "2026-09-25T00:00:00Z",
            "calendars": {
                "primary": {
                    "busy": [
                        { "start": "2026-09-24T10:00:00Z", "end": "2026-09-24T11:00:00Z" }
                    ]
                }
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let response = client
        .freebusy(&["primary"], "2026-09-24T00:00:00Z", "2026-09-25T00:00:00Z")
        .await
        .unwrap();

    assert_eq!(response.time_min.as_deref(), Some("2026-09-24T00:00:00Z"));
    let calendar = response.calendars.get("primary").unwrap();
    assert_eq!(calendar.busy.len(), 1);
    assert_eq!(
        calendar.busy[0].start.as_deref(),
        Some("2026-09-24T10:00:00Z")
    );
    assert_eq!(
        calendar.busy[0].end.as_deref(),
        Some("2026-09-24T11:00:00Z")
    );
}

#[tokio::test]
async fn calendars_crud_uses_calendar_resource_paths() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendars/team-calendar"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "calendar#calendar",
            "etag": "etag-1",
            "id": "team-calendar",
            "summary": "Team calendar",
            "description": "Shared planning",
            "location": "HQ",
            "timeZone": "Europe/Zurich"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/calendars"))
        .and(body_json(serde_json::json!({
            "summary": "Team calendar",
            "description": "Shared planning",
            "timeZone": "Europe/Zurich"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "team-calendar",
            "summary": "Team calendar",
            "description": "Shared planning",
            "timeZone": "Europe/Zurich"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/calendars/team-calendar"))
        .and(body_json(serde_json::json!({
            "summary": "Renamed calendar",
            "colorId": "5"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "team-calendar",
            "summary": "Renamed calendar",
            "colorId": "5"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/calendars/team-calendar"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let calendar = client.get_calendar("team-calendar").await.unwrap();
    assert_eq!(calendar.id, "team-calendar");
    assert_eq!(calendar.time_zone.as_deref(), Some("Europe/Zurich"));

    let created = client
        .create_calendar(Calendar {
            summary: "Team calendar".into(),
            description: Some("Shared planning".into()),
            time_zone: Some("Europe/Zurich".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(created.summary, "Team calendar");

    let updated = client
        .update_calendar(
            "team-calendar",
            CalendarPatch {
                summary: Some("Renamed calendar".into()),
                color_id: Some("5".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.color_id.as_deref(), Some("5"));

    client.delete_calendar("team-calendar").await.unwrap();
}

#[tokio::test]
async fn calendar_list_patch_sends_only_requested_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/users/me/calendarList/team-calendar"))
        .and(body_json(serde_json::json!({
            "colorId": "5",
            "summaryOverride": "Team",
            "selected": false,
            "hidden": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "team-calendar",
            "summaryOverride": "Team",
            "colorId": "5",
            "selected": false,
            "hidden": true,
            "defaultReminders": [{ "method": "popup", "minutes": 10 }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let entry = client
        .patch_calendar_list_entry(
            "team-calendar",
            CalendarListPatch {
                color_id: Some("5".into()),
                summary_override: Some("Team".into()),
                selected: Some(false),
                hidden: Some(true),
                default_reminders: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(entry.id, "team-calendar");
    assert_eq!(entry.summary_override.as_deref(), Some("Team"));
    assert_eq!(entry.default_reminders.unwrap()[0].minutes, Some(10));
}

#[tokio::test]
async fn event_instances_sends_time_window_and_parses_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/calendars/team-calendar/events/recurring-1/instances",
        ))
        .and(query_param("maxResults", "2"))
        .and(query_param("timeMin", "2026-09-01T00:00:00Z"))
        .and(query_param("timeMax", "2026-10-01T00:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "calendar#events",
            "timeZone": "UTC",
            "items": [
                {
                    "id": "recurring-1_20260901T000000Z",
                    "recurringEventId": "recurring-1",
                    "summary": "Weekly sync",
                    "start": { "dateTime": "2026-09-01T09:00:00Z" },
                    "end": { "dateTime": "2026-09-01T10:00:00Z" }
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let instances = client
        .list_event_instances(
            "team-calendar",
            "recurring-1",
            EventInstancesOptions {
                max_results: Some(2),
                time_min: Some("2026-09-01T00:00:00Z".into()),
                time_max: Some("2026-10-01T00:00:00Z".into()),
            },
        )
        .await
        .unwrap();

    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0].recurring_event_id.as_deref(),
        Some("recurring-1")
    );
    assert_eq!(instances[0].summary.as_deref(), Some("Weekly sync"));
}

#[tokio::test]
async fn move_event_uses_destination_query_without_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendars/source-calendar/events/event-1/move"))
        .and(query_param("destination", "target-calendar"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-1",
            "summary": "Moved event",
            "organizer": { "email": "owner@example.com", "self": true },
            "start": { "dateTime": "2026-09-24T10:00:00Z" },
            "end": { "dateTime": "2026-09-24T11:00:00Z" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let moved = client
        .move_event("source-calendar", "event-1", "target-calendar")
        .await
        .unwrap();

    assert_eq!(moved.id.as_deref(), Some("event-1"));
    assert_eq!(moved.summary.as_deref(), Some("Moved event"));
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].body.is_empty());
}

#[tokio::test]
async fn patch_event_sends_only_supplied_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/calendars/team-calendar/events/event-1"))
        .and(body_json(serde_json::json!({
            "summary": "Updated title",
            "location": "Room 2"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-1",
            "summary": "Updated title",
            "location": "Room 2"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let updated = client
        .patch_event(
            "team-calendar",
            "event-1",
            EventPatch {
                summary: Some("Updated title".into()),
                location: Some("Room 2".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.summary.as_deref(), Some("Updated title"));
    assert_eq!(updated.location.as_deref(), Some("Room 2"));
}

#[tokio::test]
async fn watch_and_stop_use_channel_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendars/team-calendar/events/watch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "api#channel",
            "id": "channel-1",
            "resourceId": "resource-1",
            "resourceUri": "https://www.googleapis.com/calendar/v3/calendars/team-calendar/events",
            "expiration": 1790000000000_u64
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/channels/stop"))
        .and(body_json(serde_json::json!({
            "id": "channel-1",
            "resourceId": "resource-1"
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let channel = client
        .watch_calendar("team-calendar", "https://hooks.example.com/calendar")
        .await
        .unwrap();
    assert_eq!(channel.id, "channel-1");
    assert_eq!(channel.resource_id.as_deref(), Some("resource-1"));

    client.stop_watch("channel-1", "resource-1").await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let watch_body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(watch_body["type"], "web_hook");
    assert_eq!(watch_body["address"], "https://hooks.example.com/calendar");
    assert!(
        watch_body["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("grr-"))
    );
}

#[tokio::test]
async fn colors_parses_calendar_and_event_palettes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/colors"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "calendar#colors",
            "updated": "2026-09-24T00:00:00Z",
            "calendar": {
                "1": { "background": "#ac725e", "foreground": "#1d1d1d" }
            },
            "event": {
                "1": { "background": "#a4bdfc", "foreground": "#1d1d1d" }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let colors = client.get_colors().await.unwrap();
    assert_eq!(colors.calendar["1"].background.as_deref(), Some("#ac725e"));
    assert_eq!(colors.event["1"].foreground.as_deref(), Some("#1d1d1d"));
}

#[tokio::test]
async fn settings_list_parses_resource_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/settings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "calendar#settings",
            "etag": "settings-etag",
            "items": [
                { "kind": "calendar#setting", "id": "timezone", "value": "Europe/Zurich" },
                { "kind": "calendar#setting", "id": "locale", "value": "en_GB" },
                { "kind": "calendar#setting", "id": "weekStart", "value": "1" }
            ],
            "nextSyncToken": "sync-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let settings = client.list_settings().await.unwrap();
    assert_eq!(settings.items.len(), 3);
    assert_eq!(settings.items[0].id, "timezone");
    assert_eq!(settings.items[0].value, "Europe/Zurich");
    assert_eq!(settings.items[2].value, "1");
    assert_eq!(settings.next_sync_token.as_deref(), Some("sync-1"));
}

#[tokio::test]
async fn acl_insert_list_get_patch_and_delete_use_acl_paths() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendars/team-calendar/acl"))
        .and(query_param("sendNotifications", "true"))
        .and(body_json(serde_json::json!({
            "scope": { "type": "user", "value": "person@example.com" },
            "role": "writer"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "calendar#aclRule",
            "etag": "acl-etag-1",
            "id": "rule-1",
            "scope": { "type": "user", "value": "person@example.com" },
            "role": "writer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/team-calendar/acl"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kind": "calendar#acl",
            "items": [{
                "id": "rule-1",
                "scope": { "type": "user", "value": "person@example.com" },
                "role": "writer"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendars/team-calendar/acl/rule-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "rule-1",
            "scope": { "type": "user", "value": "person@example.com" },
            "role": "writer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/calendars/team-calendar/acl/rule-1"))
        .and(body_json(serde_json::json!({ "role": "reader" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "rule-1",
            "scope": { "type": "user", "value": "person@example.com" },
            "role": "reader"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/calendars/team-calendar/acl/rule-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let inserted = client
        .insert_acl_rule(
            "team-calendar",
            AclRule {
                id: String::new(),
                scope: AclScope {
                    r#type: "user".into(),
                    value: Some("person@example.com".into()),
                },
                role: "writer".into(),
            },
            Some(true),
        )
        .await
        .unwrap();
    assert_eq!(inserted.id, "rule-1");

    let rules = client.list_acl("team-calendar", None).await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].scope.r#type, "user");
    assert_eq!(rules[0].role, "writer");

    let fetched = client
        .get_acl_rule("team-calendar", "rule-1")
        .await
        .unwrap();
    assert_eq!(fetched.scope.value.as_deref(), Some("person@example.com"));

    let patched = client
        .patch_acl_rule(
            "team-calendar",
            "rule-1",
            AclPatch {
                role: Some("reader".into()),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(patched.role, "reader");

    client
        .delete_acl_rule("team-calendar", "rule-1")
        .await
        .unwrap();
}

/// Build a client pointed at the mock server with a pre-authed, in-memory
/// token so no OAuth flow or token-endpoint round-trip ever happens.
/// Both h3 and h2 prior-knowledge are disabled: wiremock speaks HTTP/1.1.
async fn test_client(base: &str) -> CalendarClient {
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
    CalendarClientBuilder::new()
        .auth(auth)
        .base_url(base.parse().unwrap())
        .build()
        .await
        .unwrap()
}

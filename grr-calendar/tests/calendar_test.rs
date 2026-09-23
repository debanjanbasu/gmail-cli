//! Wiremock tests for the Calendar client over the shared HTTP core.

use grr_calendar::{CalendarClient, CalendarClientBuilder, Event, EventDateTime, EventListOptions};
use grr_core::{GoogleAuth, GrrConfig, TokenStorage};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
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

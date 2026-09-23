//! Google Calendar API response models deserialized with serde

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A calendar on the user's calendar list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListEntry {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub time_zone: Option<String>,
    pub color_id: Option<String>,
    pub background_color: Option<String>,
    pub foreground_color: Option<String>,
    pub selected: Option<bool>,
    pub access_role: Option<String>,
    pub primary: Option<bool>,
    pub deleted: Option<bool>,
    pub hidden: Option<bool>,
}

/// A calendar resource (calendars.get / calendars.insert payload)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub time_zone: Option<String>,
}

/// Calendar event
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub start: Option<EventDateTime>,
    pub end: Option<EventDateTime>,
    pub hangout_link: Option<String>,
    pub html_link: Option<String>,
    pub attendees: Option<Vec<EventAttendee>>,
    pub recurring_event_id: Option<String>,
    pub creator: Option<EventPerson>,
    pub organizer: Option<EventPerson>,
}

/// Event start/end: a bare `date` for all-day events or a `dateTime`
/// (RFC3339, optionally with a `timeZone` offset reference)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDateTime {
    pub date: Option<String>,
    pub date_time: Option<String>,
    pub time_zone: Option<String>,
}

/// Event attendee
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendee {
    pub id: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub organizer: Option<bool>,
    /// The JSON field is the keyword "self"; it cannot be a Rust field.
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
    pub resource: Option<bool>,
    pub optional: Option<bool>,
    pub response_status: Option<String>,
    pub comment: Option<String>,
    pub additional_guests: Option<u32>,
}

/// Event creator or organizer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPerson {
    pub id: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    /// The JSON field is the keyword "self"; it cannot be a Rust field.
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

/// Envelope of events.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Events {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub time_zone: Option<String>,
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub items: Vec<Event>,
}

/// Envelope of calendarList.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarList {
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub items: Vec<CalendarListEntry>,
}

/// Body of freebusy.query
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreeBusyRequest {
    pub time_min: String,
    pub time_max: String,
    #[serde(default)]
    pub items: Vec<FreeBusyItem>,
}

/// One calendar to query in a freebusy.request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreeBusyItem {
    pub id: String,
}

/// Response of freebusy.query
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreeBusyResponse {
    pub time_min: Option<String>,
    pub time_max: Option<String>,
    #[serde(default)]
    pub calendars: HashMap<String, FreeBusyCalendar>,
}

/// Free/busy periods of one queried calendar
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreeBusyCalendar {
    #[serde(default)]
    pub busy: Vec<TimePeriod>,
    pub errors: Option<Vec<FreeBusyError>>,
}

/// Why a calendar's free/busy could not be fetched
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreeBusyError {
    pub domain: Option<String>,
    pub reason: Option<String>,
}

/// A busy period (RFC3339 timestamps)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimePeriod {
    pub start: Option<String>,
    pub end: Option<String>,
}

/// Optional filters for events.list
#[derive(Debug, Clone, Default)]
pub struct EventListOptions {
    /// RFC3339 lower bound (inclusive) for an event's start time
    pub time_min: Option<String>,
    /// RFC3339 upper bound (exclusive) for an event's start time
    pub time_max: Option<String>,
    /// Free-text search over event fields
    pub query: Option<String>,
    /// Stop after this many events; None paginates until exhausted
    pub max_results: Option<usize>,
}

//! Calendar-related CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_calendar::{CalendarClient, Event, EventAttendee, EventDateTime, EventListOptions};

#[derive(Subcommand, Debug)]
pub enum CalendarCommands {
    /// List calendars on the user's calendar list
    List(ListArgs),
    /// List events on a calendar
    Events(EventsArgs),
    /// Get a single event by ID
    Get(GetArgs),
    /// Create an event
    Create(CreateArgs),
    /// Update an event
    Update(UpdateArgs),
    /// Delete an event
    Delete(DeleteArgs),
    /// Query free/busy periods across calendars
    FreeBusy(FreeBusyArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Maximum number of calendars
    #[arg(short, long)]
    pub max: Option<usize>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct EventsArgs {
    /// Calendar ID (defaults to the user's primary calendar)
    #[arg(default_value = "primary")]
    pub calendar_id: String,

    /// RFC3339 lower bound for event start times (e.g. 2026-09-24T00:00:00Z)
    #[arg(long)]
    pub time_min: Option<String>,

    /// RFC3339 upper bound (exclusive)
    #[arg(long)]
    pub time_max: Option<String>,

    /// Free-text search over event fields
    #[arg(short, long)]
    pub query: Option<String>,

    /// Maximum number of events
    #[arg(short, long, default_value = "25")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Calendar ID (defaults to the user's primary calendar)
    #[arg(default_value = "primary")]
    pub calendar_id: String,

    /// Event ID
    pub event_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Calendar ID (defaults to the user's primary calendar)
    #[arg(default_value = "primary")]
    pub calendar_id: String,

    /// Event title
    #[arg(long)]
    pub summary: String,

    /// Event start (RFC3339 timestamp or YYYY-MM-DD date for all-day)
    #[arg(long)]
    pub start: String,

    /// Event end (RFC3339 timestamp or YYYY-MM-DD date for all-day)
    #[arg(long)]
    pub end: String,

    /// Location
    #[arg(long)]
    pub location: Option<String>,

    /// Description
    #[arg(long)]
    pub description: Option<String>,

    /// Attendee emails (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub attendees: Vec<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Calendar ID (defaults to the user's primary calendar)
    #[arg(default_value = "primary")]
    pub calendar_id: String,

    /// Event ID
    pub event_id: String,

    /// New event title
    #[arg(long)]
    pub summary: Option<String>,

    /// New event start (RFC3339 timestamp or YYYY-MM-DD date for all-day)
    #[arg(long)]
    pub start: Option<String>,

    /// New event end (RFC3339 timestamp or YYYY-MM-DD date for all-day)
    #[arg(long)]
    pub end: Option<String>,

    /// New location
    #[arg(long)]
    pub location: Option<String>,

    /// New description
    #[arg(long)]
    pub description: Option<String>,

    /// New attendee emails (comma-separated; replaces existing)
    #[arg(long, value_delimiter = ',')]
    pub attendees: Vec<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Calendar ID (defaults to the user's primary calendar)
    #[arg(default_value = "primary")]
    pub calendar_id: String,

    /// Event ID
    pub event_id: String,
}

#[derive(Args, Debug)]
pub struct FreeBusyArgs {
    /// Calendar IDs (comma-separated)
    #[arg(long, value_delimiter = ',', default_value = "primary")]
    pub calendars: Vec<String>,

    /// RFC3339 lower bound (e.g. 2026-09-24T00:00:00Z)
    #[arg(long)]
    pub time_min: String,

    /// RFC3339 upper bound (exclusive)
    #[arg(long)]
    pub time_max: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

/// Build an EventDateTime from a user-supplied value: bare dates
/// (YYYY-MM-DD) ride the `date` field as all-day boundaries; anything with
/// a time component rides `dateTime`. Format validation stays with the
/// API — malformed values surface as `GrrError::Api`.
fn event_date_time(value: &str) -> EventDateTime {
    if value.contains(':') || value.contains('T') {
        EventDateTime {
            date_time: Some(value.to_string()),
            ..Default::default()
        }
    } else {
        EventDateTime {
            date: Some(value.to_string()),
            ..Default::default()
        }
    }
}

/// Attendee emails -> EventAttendee list (None when no emails given).
fn attendees_from(emails: &[String]) -> Option<Vec<EventAttendee>> {
    (!emails.is_empty()).then(|| {
        emails
            .iter()
            .map(|email| EventAttendee {
                email: Some(email.clone()),
                ..Default::default()
            })
            .collect()
    })
}

pub async fn handle_calendar_cmd(client: &CalendarClient, cmd: CalendarCommands) -> Result<()> {
    match cmd {
        CalendarCommands::List(args) => {
            let calendars = client.list_calendars(args.max).await?;
            print_output(&calendars, args.format)?;
        }
        CalendarCommands::Events(args) => {
            let opts = EventListOptions {
                time_min: args.time_min,
                time_max: args.time_max,
                query: args.query,
                max_results: Some(args.max),
            };
            let events = client.list_events(&args.calendar_id, opts).await?;
            print_output(&events, args.format)?;
        }
        CalendarCommands::Get(args) => {
            let event = client.get_event(&args.calendar_id, &args.event_id).await?;
            print_output(&event, args.format)?;
        }
        CalendarCommands::Create(args) => {
            let event = Event {
                summary: Some(args.summary),
                start: Some(event_date_time(&args.start)),
                end: Some(event_date_time(&args.end)),
                location: args.location,
                description: args.description,
                attendees: attendees_from(&args.attendees),
                ..Default::default()
            };
            let created = client.create_event(&args.calendar_id, event).await?;
            print_output(&created, args.format)?;
        }
        CalendarCommands::Update(args) => {
            // PUT replaces the whole resource: fetch, overlay the flags
            // that were given, then send the merged event back.
            let mut event = client.get_event(&args.calendar_id, &args.event_id).await?;
            if let Some(summary) = args.summary {
                event.summary = Some(summary);
            }
            if let Some(start) = args.start {
                event.start = Some(event_date_time(&start));
            }
            if let Some(end) = args.end {
                event.end = Some(event_date_time(&end));
            }
            if let Some(location) = args.location {
                event.location = Some(location);
            }
            if let Some(description) = args.description {
                event.description = Some(description);
            }
            if !args.attendees.is_empty() {
                event.attendees = attendees_from(&args.attendees);
            }
            let updated = client
                .update_event(&args.calendar_id, &args.event_id, event)
                .await?;
            print_output(&updated, args.format)?;
        }
        CalendarCommands::Delete(args) => {
            client
                .delete_event(&args.calendar_id, &args.event_id)
                .await?;
            println!("Event {} deleted", args.event_id);
        }
        CalendarCommands::FreeBusy(args) => {
            let calendars: Vec<&str> = args.calendars.iter().map(String::as_str).collect();
            let response = client
                .freebusy(&calendars, &args.time_min, &args.time_max)
                .await?;
            print_output(&response, args.format)?;
        }
    }
    Ok(())
}

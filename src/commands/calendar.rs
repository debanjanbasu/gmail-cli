//! Calendar-related CLI commands

use crate::calendar::{
    AclRule, AclScope, Calendar, CalendarClient, CalendarListPatch, CalendarPatch, Event,
    EventAttendee, EventDateTime, EventInstancesOptions, EventListOptions, EventPatch,
};
use crate::output::{OutputFormat, print_output};
use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum CalendarCommands {
    #[command(about = "List calendars on the user's calendar list")]
    List(ListArgs),
    #[command(about = "List events on a calendar")]
    Events(EventsArgs),
    #[command(about = "Get a calendar or an event")]
    Get(GetArgs),
    #[command(about = "Create a calendar")]
    New(NewCalendarArgs),
    #[command(about = "Create an event")]
    Create(CreateArgs),
    #[command(about = "Update a calendar or an event")]
    Update(UpdateArgs),
    #[command(about = "Delete a calendar or an event")]
    Delete(DeleteArgs),
    #[command(about = "Query free/busy periods across calendars")]
    FreeBusy(FreeBusyArgs),
    #[command(about = "Update a calendar-list entry")]
    Set(SetArgs),
    #[command(about = "List instances of a recurring event")]
    Instances(InstancesArgs),
    #[command(about = "Patch fields on an event")]
    Patch(PatchEventArgs),
    #[command(about = "Move an event to another calendar")]
    Move(MoveEventArgs),
    #[command(about = "Watch calendar events")]
    Watch(WatchArgs),
    #[command(about = "Stop a calendar watch channel")]
    Stop(StopArgs),
    #[command(about = "Get calendar and event colors")]
    Colors(ColorsArgs),
    #[command(about = "List calendar settings")]
    Settings(SettingsArgs),
    #[command(about = "Share a calendar")]
    Share(ShareArgs),
    #[command(about = "List calendar sharing rules")]
    Shares(SharesArgs),
    #[command(about = "Remove a calendar sharing rule")]
    Unshare(UnshareArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, help = "Maximum number of calendars")]
    pub max: Option<usize>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct EventsArgs {
    #[arg(default_value = "primary", help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(long, help = "RFC3339 lower bound for event start times")]
    pub time_min: Option<String>,

    #[arg(long, help = "RFC3339 upper bound (exclusive)")]
    pub time_max: Option<String>,

    #[arg(short, long, help = "Free-text search over event fields")]
    pub query: Option<String>,

    #[arg(short, long, default_value = "25", help = "Maximum number of events")]
    pub max: usize,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    #[arg(help = "Calendar ID; add an event ID to get an event")]
    pub calendar_id: String,

    #[arg(help = "Optional event ID")]
    pub event_id: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct NewCalendarArgs {
    #[arg(long, help = "Calendar title")]
    pub summary: String,

    #[arg(long, help = "Calendar description")]
    pub description: Option<String>,

    #[arg(long, help = "IANA time zone")]
    pub timezone: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(default_value = "primary", help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(long, help = "Event title")]
    pub summary: String,

    #[arg(long, help = "RFC3339 timestamp or YYYY-MM-DD date")]
    pub start: String,

    #[arg(long, help = "RFC3339 timestamp or YYYY-MM-DD date")]
    pub end: String,

    #[arg(long, help = "Event location")]
    pub location: Option<String>,

    #[arg(long, help = "Event description")]
    pub description: Option<String>,

    #[arg(long, value_delimiter = ',', help = "Attendee emails")]
    pub attendees: Vec<String>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(help = "Calendar ID; add an event ID to update an event")]
    pub calendar_id: String,

    #[arg(help = "Optional event ID")]
    pub event_id: Option<String>,

    #[arg(long, help = "New title")]
    pub summary: Option<String>,

    #[arg(long, help = "New RFC3339 timestamp or YYYY-MM-DD date")]
    pub start: Option<String>,

    #[arg(long, help = "New RFC3339 timestamp or YYYY-MM-DD date")]
    pub end: Option<String>,

    #[arg(long, help = "New location")]
    pub location: Option<String>,

    #[arg(long, help = "New description")]
    pub description: Option<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "New attendee emails (replaces existing)"
    )]
    pub attendees: Vec<String>,

    #[arg(long, help = "New calendar IANA time zone")]
    pub timezone: Option<String>,

    #[arg(long, help = "New calendar color ID")]
    pub color_id: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    #[arg(help = "Calendar ID; add an event ID to delete an event")]
    pub calendar_id: String,

    #[arg(help = "Optional event ID")]
    pub event_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct FreeBusyArgs {
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "primary",
        help = "Calendar IDs"
    )]
    pub calendars: Vec<String>,

    #[arg(long, help = "RFC3339 lower bound")]
    pub time_min: String,

    #[arg(long, help = "RFC3339 upper bound (exclusive)")]
    pub time_max: String,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    #[arg(help = "Calendar-list entry ID")]
    pub calendar_id: String,

    #[arg(long, help = "Calendar color ID")]
    pub color_id: Option<String>,

    #[arg(long, help = "Calendar-list summary override")]
    pub summary_override: Option<String>,

    #[arg(
        long,
        value_name = "true|false",
        action = clap::ArgAction::Set,
        help = "Whether the calendar is selected"
    )]
    pub selected: Option<bool>,

    #[arg(
        long,
        value_name = "true|false",
        action = clap::ArgAction::Set,
        help = "Whether the calendar is hidden"
    )]
    pub hidden: Option<bool>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct InstancesArgs {
    #[arg(help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(help = "Recurring event ID")]
    pub event_id: String,

    #[arg(long, help = "Maximum number of instances")]
    pub max: Option<usize>,

    #[arg(long, help = "RFC3339 lower bound (inclusive)")]
    pub time_min: Option<String>,

    #[arg(long, help = "RFC3339 upper bound (exclusive)")]
    pub time_max: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct PatchEventArgs {
    #[arg(help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(help = "Event ID")]
    pub event_id: String,

    #[arg(long, help = "New event title")]
    pub summary: Option<String>,

    #[arg(long, help = "New event description")]
    pub description: Option<String>,

    #[arg(long, help = "New RFC3339 timestamp or YYYY-MM-DD date")]
    pub start: Option<String>,

    #[arg(long, help = "New RFC3339 timestamp or YYYY-MM-DD date")]
    pub end: Option<String>,

    #[arg(long, help = "New event location")]
    pub location: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct MoveEventArgs {
    #[arg(help = "Source calendar ID")]
    pub from_calendar: String,

    #[arg(help = "Event ID")]
    pub event_id: String,

    #[arg(help = "Destination calendar ID")]
    pub to_calendar: String,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    #[arg(help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(long, help = "HTTPS webhook address")]
    pub webhook: String,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct StopArgs {
    #[arg(help = "Channel ID returned by watch")]
    pub channel_id: String,

    #[arg(help = "Opaque resource ID returned by watch")]
    pub resource_id: String,
}

#[derive(Args, Debug)]
pub struct ColorsArgs {
    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SettingsArgs {
    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShareRole {
    Reader,
    Writer,
}

impl ShareRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Writer => "writer",
        }
    }
}

#[derive(Args, Debug)]
pub struct ShareArgs {
    #[arg(help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(
        long,
        required_unless_present = "domain",
        conflicts_with = "domain",
        help = "User email address"
    )]
    pub email: Option<String>,

    #[arg(
        long,
        required_unless_present = "email",
        conflicts_with = "email",
        help = "Workspace domain name"
    )]
    pub domain: Option<String>,

    #[arg(long, value_enum, help = "Access role")]
    pub role: ShareRole,

    #[arg(long, help = "Send notifications about the sharing change")]
    pub send_notifications: bool,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SharesArgs {
    #[arg(help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(
        short,
        long,
        value_enum,
        default_value = "json",
        help = "Output format"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UnshareArgs {
    #[arg(help = "Calendar ID")]
    pub calendar_id: String,

    #[arg(help = "ACL rule ID")]
    pub rule_id: String,
}

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
            if let Some(event_id) = args.event_id {
                let event = client.get_event(&args.calendar_id, &event_id).await?;
                print_output(&event, args.format)?;
            } else {
                let calendar = client.get_calendar(&args.calendar_id).await?;
                print_output(&calendar, args.format)?;
            }
        }
        CalendarCommands::New(args) => {
            let calendar = Calendar {
                summary: args.summary,
                description: args.description,
                time_zone: args.timezone,
                ..Default::default()
            };
            let created = client.create_calendar(calendar).await?;
            print_output(&created, args.format)?;
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
            let has_event_fields = args.start.is_some()
                || args.end.is_some()
                || args.location.is_some()
                || !args.attendees.is_empty();

            if let Some(event_id) = args.event_id {
                if args.timezone.is_some() || args.color_id.is_some() {
                    bail!("--timezone and --color-id are calendar-only options");
                }
                let mut event = client.get_event(&args.calendar_id, &event_id).await?;
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
                    .update_event(&args.calendar_id, &event_id, event)
                    .await?;
                print_output(&updated, args.format)?;
            } else {
                if has_event_fields {
                    bail!("event fields require an event ID");
                }
                let patch = CalendarPatch {
                    summary: args.summary,
                    description: args.description,
                    time_zone: args.timezone,
                    color_id: args.color_id,
                    ..Default::default()
                };
                if patch.summary.is_none()
                    && patch.description.is_none()
                    && patch.time_zone.is_none()
                    && patch.color_id.is_none()
                {
                    bail!("at least one calendar field is required");
                }
                let updated = client.update_calendar(&args.calendar_id, patch).await?;
                print_output(&updated, args.format)?;
            }
        }
        CalendarCommands::Delete(args) => {
            if let Some(event_id) = args.event_id {
                client.delete_event(&args.calendar_id, &event_id).await?;
                println!("Event {} deleted", event_id);
            } else {
                client.delete_calendar(&args.calendar_id).await?;
                println!("Calendar {} deleted", args.calendar_id);
            }
        }
        CalendarCommands::FreeBusy(args) => {
            let calendars: Vec<&str> = args.calendars.iter().map(String::as_str).collect();
            let response = client
                .freebusy(&calendars, &args.time_min, &args.time_max)
                .await?;
            print_output(&response, args.format)?;
        }
        CalendarCommands::Set(args) => {
            let patch = CalendarListPatch {
                color_id: args.color_id,
                summary_override: args.summary_override,
                selected: args.selected,
                hidden: args.hidden,
                default_reminders: None,
            };
            if patch.color_id.is_none()
                && patch.summary_override.is_none()
                && patch.selected.is_none()
                && patch.hidden.is_none()
            {
                bail!("at least one calendar-list field is required");
            }
            let updated = client
                .patch_calendar_list_entry(&args.calendar_id, patch)
                .await?;
            print_output(&updated, args.format)?;
        }
        CalendarCommands::Instances(args) => {
            let instances = client
                .list_event_instances(
                    &args.calendar_id,
                    &args.event_id,
                    EventInstancesOptions {
                        max_results: args.max,
                        time_min: args.time_min,
                        time_max: args.time_max,
                    },
                )
                .await?;
            print_output(&instances, args.format)?;
        }
        CalendarCommands::Patch(args) => {
            let patch = EventPatch {
                summary: args.summary,
                description: args.description,
                location: args.location,
                start: args.start.as_deref().map(event_date_time),
                end: args.end.as_deref().map(event_date_time),
            };
            if patch.summary.is_none()
                && patch.description.is_none()
                && patch.location.is_none()
                && patch.start.is_none()
                && patch.end.is_none()
            {
                bail!("at least one event field is required");
            }
            let updated = client
                .patch_event(&args.calendar_id, &args.event_id, patch)
                .await?;
            print_output(&updated, args.format)?;
        }
        CalendarCommands::Move(args) => {
            let moved = client
                .move_event(&args.from_calendar, &args.event_id, &args.to_calendar)
                .await?;
            print_output(&moved, args.format)?;
        }
        CalendarCommands::Watch(args) => {
            let channel = client
                .watch_calendar(&args.calendar_id, &args.webhook)
                .await?;
            print_output(&channel, args.format)?;
        }
        CalendarCommands::Stop(args) => {
            client
                .stop_watch(&args.channel_id, &args.resource_id)
                .await?;
        }
        CalendarCommands::Colors(args) => {
            let colors = client.get_colors().await?;
            print_output(&colors, args.format)?;
        }
        CalendarCommands::Settings(args) => {
            let settings = client.list_settings().await?;
            print_output(&settings, args.format)?;
        }
        CalendarCommands::Share(args) => {
            let scope = match (args.email, args.domain) {
                (Some(email), None) => AclScope {
                    r#type: "user".into(),
                    value: Some(email),
                },
                (None, Some(domain)) => AclScope {
                    r#type: "domain".into(),
                    value: Some(domain),
                },
                _ => bail!("exactly one of --email or --domain is required"),
            };
            let rule = AclRule {
                id: String::new(),
                scope,
                role: args.role.as_str().into(),
            };
            let inserted = client
                .insert_acl_rule(
                    &args.calendar_id,
                    rule,
                    args.send_notifications.then_some(true),
                )
                .await?;
            print_output(&inserted, args.format)?;
        }
        CalendarCommands::Shares(args) => {
            let rules = client.list_acl(&args.calendar_id, None).await?;
            print_output(&rules, args.format)?;
        }
        CalendarCommands::Unshare(args) => {
            client
                .delete_acl_rule(&args.calendar_id, &args.rule_id)
                .await?;
        }
    }
    Ok(())
}

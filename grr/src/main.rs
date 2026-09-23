//! grr — Google tools from the terminal, at maximum performance.
//!
//! One binary, one login, every service: commands are namespaced by
//! service (`grr gmail ...`, `grr calendar ...`, `grr drive ...`,
//! `grr contacts ...`, `grr chat ...`, `grr forms ...`). Account-level
//! concerns (auth, transport, schema) stay top-level. Each service client
//! is built lazily in its dispatch arm so running one service never
//! probes another's endpoints.

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use grr_core::prelude::*;
use grr_gmail::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod output;

use commands::{
    auth, calendar, chat, contacts, drafts, drive, forms, history, import, labels, message_ops,
    messages, profile, schema, send, send_as, thread_ops, transport, watch,
};

#[derive(Parser)]
#[command(
    name = "grr",
    version,
    about = "Google tools from the terminal, at maximum performance"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    // NOTE: no global --format flag on purpose. A global `format` collides
    // by ID with per-command `format` fields of *different types*
    // (e.g. message get's MessageFormat), which panics clap's downcast at
    // runtime (0xC0000409). Every subcommand declares its own -f/--format.
}

#[derive(Subcommand)]
enum Commands {
    /// Google account authentication (PKCE browser flow; --device for headless)
    #[command(subcommand, subcommand_required = true)]
    Auth(auth::AuthCommands),

    /// Gmail operations
    #[command(subcommand)]
    Gmail(GmailCommands),

    /// Calendar operations
    #[command(subcommand)]
    Calendar(calendar::CalendarCommands),

    /// Google Drive operations
    #[command(subcommand)]
    Drive(drive::DriveCommands),

    /// Contacts (Google People API) operations
    #[command(subcommand)]
    Contacts(contacts::ContactsCommands),

    /// Google Chat operations
    #[command(subcommand)]
    Chat(chat::ChatCommands),

    /// Google Forms operations
    #[command(subcommand)]
    Forms(forms::FormsCommands),

    /// Show negotiated transport protocol and runtime features
    Transport(transport::TransportArgs),

    /// Dump the full command tree as JSON (machine-readable contract)
    Schema(schema::SchemaArgs),
}

#[derive(Subcommand)]
enum GmailCommands {
    /// Message operations
    #[command(subcommand)]
    Message(messages::MessageCommands),
    /// Label operations
    #[command(subcommand)]
    Label(labels::LabelCommands),
    /// Draft operations
    #[command(subcommand)]
    Draft(drafts::DraftCommands),
    /// Send emails
    #[command(subcommand)]
    Send(send::SendCommands),
    /// Thread operations
    #[command(subcommand)]
    Thread(thread_ops::ThreadCommands),
    /// History operations
    History(history::HistoryArgs),
    /// Send-as alias operations
    #[command(subcommand)]
    SendAs(send_as::SendAsCommands),
    /// Profile operations
    Profile(profile::ProfileArgs),
    /// Watch (push notifications) operations
    #[command(subcommand)]
    Watch(watch::WatchCommands),
    /// Import RFC 822 message
    Import(import::ImportArgs),
    /// Message operations (label, trash, delete, batch)
    #[command(subcommand)]
    Msg(message_ops::MessageOpsCommands),
}

/// Build a fresh GoogleAuth from the loaded config. One credential backs
/// every service; each client gets its own handle (the token store is
/// shared, so this is a cheap read).
async fn build_auth(config: &GrrConfig) -> Result<GoogleAuth> {
    Ok(AuthConfigBuilder::new()
        .client_id(config.oauth.client_id.clone())
        .client_secret(config.oauth.client_secret.clone())
        .build()
        .await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            // Default: info, but silence quinn_udp's harmless IPv6
            // network-unreachable warnings on v6-less networks (v4 wins).
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,quinn_udp=error".into()),
        ))
        // Logs go to stderr so stdout stays pure machine-readable output
        // (`grr gmail profile | jq` must not receive log lines).
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let cli = Cli::parse();

    // The schema dump is pure clap introspection: it must answer with zero
    // configuration, before any client (or OAuth) exists.
    if let Commands::Schema(args) = &cli.command {
        commands::schema::handle_schema_cmd(Cli::command(), args.clone())?;
        return Ok(());
    }

    let config = ConfigLoader::load().await?;

    // Gmail (also the auth/transport client) is built eagerly only for the
    // commands that use it; service clients build lazily in their arms.
    let needs_gmail = matches!(
        cli.command,
        Commands::Auth(_) | Commands::Gmail(_) | Commands::Transport(_)
    );
    let gmail_client = if needs_gmail {
        let auth = build_auth(&config).await?;
        Some(GmailClientBuilder::new().auth(auth).build().await?)
    } else {
        None
    };

    match cli.command {
        // Schema was handled above, before client construction.
        Commands::Schema(_) => unreachable!("schema handled before client construction"),
        Commands::Auth(cmd) => {
            let client = gmail_client.as_ref().expect("built for auth");
            commands::auth::handle_auth_cmd(client, cmd).await?
        }
        Commands::Gmail(cmd) => {
            let client = gmail_client.as_ref().expect("built for gmail");
            match cmd {
                GmailCommands::Message(cmd) => {
                    commands::messages::handle_message_cmd(client, cmd).await?
                }
                GmailCommands::Label(cmd) => {
                    commands::labels::handle_label_cmd(client, cmd).await?
                }
                GmailCommands::Draft(cmd) => {
                    commands::drafts::handle_draft_cmd(client, cmd).await?
                }
                GmailCommands::Send(cmd) => commands::send::handle_send_cmd(client, cmd).await?,
                GmailCommands::Thread(cmd) => {
                    commands::thread_ops::handle_thread_cmd(client, cmd).await?
                }
                GmailCommands::History(args) => {
                    commands::history::handle_history_cmd(client, args).await?
                }
                GmailCommands::SendAs(cmd) => {
                    commands::send_as::handle_send_as_cmd(client, cmd).await?
                }
                GmailCommands::Profile(args) => {
                    commands::profile::handle_profile_cmd(client, args).await?
                }
                GmailCommands::Watch(cmd) => commands::watch::handle_watch_cmd(client, cmd).await?,
                GmailCommands::Import(args) => {
                    commands::import::handle_import_cmd(client, args).await?
                }
                GmailCommands::Msg(cmd) => {
                    commands::message_ops::handle_message_ops_cmd(client, cmd).await?
                }
            }
        }
        Commands::Calendar(cmd) => {
            let auth = build_auth(&config).await?;
            let client = grr_calendar::CalendarClientBuilder::new()
                .auth(auth)
                .build()
                .await?;
            commands::calendar::handle_calendar_cmd(&client, cmd).await?
        }
        Commands::Drive(cmd) => {
            let auth = build_auth(&config).await?;
            let client = grr_drive::DriveClientBuilder::new()
                .auth(auth)
                .build()
                .await?;
            commands::drive::handle_drive_cmd(&client, cmd).await?
        }
        Commands::Contacts(cmd) => {
            let auth = build_auth(&config).await?;
            let client = grr_people::PeopleClientBuilder::new()
                .auth(auth)
                .build()
                .await?;
            commands::contacts::handle_contacts_cmd(&client, cmd).await?
        }
        Commands::Chat(cmd) => {
            let auth = build_auth(&config).await?;
            let client = grr_chat::ChatClientBuilder::new()
                .auth(auth)
                .build()
                .await?;
            commands::chat::handle_chat_cmd(&client, cmd).await?
        }
        Commands::Forms(cmd) => {
            let auth = build_auth(&config).await?;
            let client = grr_forms::FormsClientBuilder::new()
                .auth(auth)
                .build()
                .await?;
            commands::forms::handle_forms_cmd(&client, cmd).await?
        }
        Commands::Transport(args) => {
            let client = gmail_client.as_ref().expect("built for transport");
            commands::transport::handle_transport_cmd(client, args).await?
        }
    }

    Ok(())
}

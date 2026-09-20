//! gmail-cli - High-performance Gmail CLI

use anyhow::Result;
use clap::{Parser, Subcommand};
use gmail_core::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod output;

use commands::{
    auth, drafts, history, import, labels, message_ops, messages, profile, send, send_as,
    thread_ops, transport, watch,
};

#[derive(Parser)]
#[command(name = "gmail", version, about = "High-performance Gmail CLI")]
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
    /// Authenticate with Gmail
    Auth(auth::AuthArgs),
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
    /// Show negotiated transport protocol and runtime features
    Transport(transport::TransportArgs),
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
        // (`gmail profile | jq` must not receive log lines).
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let cli = Cli::parse();

    let config = ConfigLoader::load().await?;
    let auth = AuthConfigBuilder::new()
        .client_id(config.oauth.client_id.clone())
        .client_secret(config.oauth.client_secret.clone())
        .redirect_uri(config.oauth.redirect_uri.clone())
        .scopes(config.oauth.scopes.clone())
        .use_pkce(config.oauth.use_pkce)
        .build()
        .await?;

    let client = GmailClientBuilder::new(config).auth(auth).build().await?;

    match cli.command {
        Commands::Auth(args) => commands::auth::handle_auth_cmd(&client, args).await?,
        Commands::Message(cmd) => commands::messages::handle_message_cmd(&client, cmd).await?,
        Commands::Label(cmd) => commands::labels::handle_label_cmd(&client, cmd).await?,
        Commands::Draft(cmd) => commands::drafts::handle_draft_cmd(&client, cmd).await?,
        Commands::Send(cmd) => commands::send::handle_send_cmd(&client, cmd).await?,
        Commands::Thread(cmd) => commands::thread_ops::handle_thread_cmd(&client, cmd).await?,
        Commands::History(args) => commands::history::handle_history_cmd(&client, args).await?,
        Commands::SendAs(cmd) => commands::send_as::handle_send_as_cmd(&client, cmd).await?,
        Commands::Profile(args) => commands::profile::handle_profile_cmd(&client, args).await?,
        Commands::Watch(cmd) => commands::watch::handle_watch_cmd(&client, cmd).await?,
        Commands::Import(args) => commands::import::handle_import_cmd(&client, args).await?,
        Commands::Msg(cmd) => commands::message_ops::handle_message_ops_cmd(&client, cmd).await?,
        Commands::Transport(args) => {
            commands::transport::handle_transport_cmd(&client, args).await?
        }
    }

    Ok(())
}

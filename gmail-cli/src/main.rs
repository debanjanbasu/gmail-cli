//! gmail-cli - High-performance Gmail CLI

use gmail_core::prelude::*;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod output;

use commands::{
    auth, drafts, history, import, labels, message_ops, messages, profile, send, send_as, thread_ops, watch,
};
use output::OutputFormat;

#[derive(Parser)]
#[command(name = "gmail", version, about = "High-performance Gmail CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, global = true, value_enum, default_value = "json")]
    format: OutputFormat,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
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

    let client = GmailClientBuilder::new(config)
        .auth(auth)
        .build()
        .await?;

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
    }

    Ok(())
}
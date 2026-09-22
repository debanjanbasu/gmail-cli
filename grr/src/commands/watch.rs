//! Watch (push notifications) CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_core::GmailClient;

#[derive(Subcommand, Debug)]
pub enum WatchCommands {
    /// Start push notifications
    Start(StartArgs),
    /// Stop push notifications
    Stop(StopArgs),
}

#[derive(Args, Debug)]
pub struct StartArgs {
    /// Pub/Sub topic name (e.g., "projects/my-project/topics/gmail")
    pub topic_name: String,

    /// Label IDs to watch (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub label_ids: Option<Vec<String>>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct StopArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_watch_cmd(client: &GmailClient, cmd: WatchCommands) -> Result<()> {
    match cmd {
        WatchCommands::Start(args) => {
            let response = client.watch(&args.topic_name, args.label_ids).await?;
            print_output(&response, args.format)?;
        }
        WatchCommands::Stop(_args) => {
            client.stop_watch().await?;
            println!("Push notifications stopped");
        }
    }
    Ok(())
}

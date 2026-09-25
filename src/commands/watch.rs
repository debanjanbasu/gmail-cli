//! Watch (push notifications) CLI commands

use crate::gmail::GmailClient;
use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};

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

    #[arg(long, value_enum)]
    pub label_filter_action: Option<WatchLabelFilter>,

    #[arg(long, value_enum)]
    pub label_filter_behavior: Option<WatchLabelFilter>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum WatchLabelFilter {
    Include,
    Exclude,
}

impl WatchLabelFilter {
    fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Exclude => "exclude",
        }
    }
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
            let response = client
                .watch_with_options(
                    &args.topic_name,
                    args.label_ids,
                    args.label_filter_action
                        .map(|value| value.as_str().to_string()),
                    args.label_filter_behavior
                        .map(|value| value.as_str().to_string()),
                )
                .await?;
            print_output(&response, args.format)?;
        }
        WatchCommands::Stop(_args) => {
            client.stop_watch().await?;
            println!("Push notifications stopped");
        }
    }
    Ok(())
}

//! History CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::Args;
use gmail_core::GmailClient;

#[derive(Args, Debug)]
pub struct HistoryArgs {
    /// Start history ID
    pub start_history_id: String,

    /// Filter by label ID
    #[arg(long)]
    pub label_id: Option<String>,

    /// Maximum number of results
    #[arg(short, long, default_value = "100")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_history_cmd(client: &GmailClient, args: HistoryArgs) -> Result<()> {
    let history = client
        .get_history(&args.start_history_id, args.label_id.as_deref(), args.max)
        .await?;
    print_output(&history, args.format)?;
    Ok(())
}

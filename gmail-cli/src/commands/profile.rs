//! Profile CLI commands

use crate::output::{print_output, OutputFormat};
use gmail_core::GmailClient;
use clap::Args;
use anyhow::Result;

#[derive(Args, Debug)]
pub struct ProfileArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_profile_cmd(
    client: &GmailClient,
    args: ProfileArgs,
) -> Result<()> {
    let profile = client.get_profile().await?;
    print_output(&profile, args.format)?;
    Ok(())
}
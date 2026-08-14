//! Auth CLI commands

use crate::output::{print_output, OutputFormat};
use gmail_core::GmailClient;
use clap::Args;
use anyhow::Result;

#[derive(Args, Debug)]
pub struct AuthArgs {
    /// Force re-authentication
    #[arg(long)]
    pub force: bool,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_auth_cmd(
    client: &GmailClient,
    args: AuthArgs,
) -> Result<()> {
    if args.force {
        // Force re-authentication by invalidating token
        client.force_refresh().await?;
    }
    
    // Trigger OAuth flow / get access token
    let token = client.access_token().await?;
    
    print_output(&serde_json::json!({
        "authenticated": true,
        "token_preview": format!("{}...", &token[..std::cmp::min(20, token.len())])
    }), args.format)?;
    
    Ok(())
}
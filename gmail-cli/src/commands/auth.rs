//! Auth CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use gmail_core::GmailClient;

#[derive(Args, Debug)]
pub struct AuthArgs {
    /// Fresh login subcommands (default: PKCE browser flow)
    #[command(subcommand)]
    pub command: Option<AuthCommands>,

    /// Force re-authentication (fresh browser login, clears stored token)
    #[arg(long)]
    pub force: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Fresh login, dropping any stored token first.
    /// Default is the PKCE browser (loopback) flow; --device prints a
    /// URL + code for headless environments instead.
    Login(LoginArgs),
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Use the OAuth device flow instead of the browser loopback flow
    #[arg(long)]
    pub device: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

fn token_preview(token: &str) -> String {
    format!("{}...", &token[..std::cmp::min(20, token.len())])
}

pub async fn handle_auth_cmd(client: &GmailClient, args: AuthArgs) -> Result<()> {
    if let Some(AuthCommands::Login(login)) = args.command {
        if login.device {
            return handle_device_login(client, login.format).await;
        }
        let storage = client.login().await?;
        print_output(
            &serde_json::json!({
                "authenticated": true,
                "token_preview": token_preview(&storage.access_token)
            }),
            login.format,
        )?;
        return Ok(());
    }

    if args.force {
        // A forced re-auth is a fresh login: revoke the (possibly dead)
        // stored token and run the browser flow. Refreshing here would
        // just repeat the same failure against the same dead token.
        let storage = client.login().await?;
        print_output(
            &serde_json::json!({
                "authenticated": true,
                "token_preview": token_preview(&storage.access_token)
            }),
            args.format,
        )?;
        return Ok(());
    }

    // Trigger OAuth flow / get access token (refreshes silently when possible)
    let token = client.access_token().await?;

    print_output(
        &serde_json::json!({
            "authenticated": true,
            "token_preview": token_preview(&token)
        }),
        args.format,
    )?;

    Ok(())
}

async fn handle_device_login(client: &GmailClient, format: OutputFormat) -> Result<()> {
    let mut challenge = client.request_device_code().await?;
    println!(
        "Visit {} and enter code: {}",
        challenge.verification_url, challenge.user_code
    );
    loop {
        tokio::time::sleep(challenge.retry_after()).await;
        if challenge.is_expired() {
            anyhow::bail!("device code expired; rerun `gmail auth login --device`");
        }
        if let Some(storage) = client.poll_device_code(&mut challenge).await? {
            print_output(
                &serde_json::json!({
                    "authenticated": true,
                    "token_preview": token_preview(&storage.access_token)
                }),
                format,
            )?;
            return Ok(());
        }
    }
}

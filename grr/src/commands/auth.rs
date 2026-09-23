//! Auth CLI commands: account-level authentication.

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::Subcommand;
use grr_gmail::GmailClient;

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Fresh login. Default is the PKCE browser (loopback) flow; --device
    /// prints a URL + code for headless environments instead.
    Login(LoginArgs),
    /// Show the authenticated account (fails when no valid credential)
    Status(StatusArgs),
}

#[derive(clap::Args, Debug)]
pub struct LoginArgs {
    /// Use the OAuth device flow instead of the browser loopback flow
    #[arg(long)]
    pub device: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(clap::Args, Debug)]
pub struct StatusArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

fn token_preview(token: &str) -> String {
    format!("{}...", &token[..std::cmp::min(20, token.len())])
}

pub async fn handle_auth_cmd(client: &GmailClient, cmd: AuthCommands) -> Result<()> {
    match cmd {
        AuthCommands::Login(login) => {
            if login.device {
                return handle_device_login(client, login.format).await;
            }
            let storage = client.login().await?;
            print_output(
                &serde_json::json!({
                    "authenticated": true,
                    "token_backend": client.token_backend(),
                    "token_preview": token_preview(&storage.access_token)
                }),
                login.format,
            )?;
            Ok(())
        }
        AuthCommands::Status(status) => {
            let profile = client.get_profile().await?;
            print_output(
                &serde_json::json!({
                    "authenticated": true,
                    "email": profile.email_address,
                    "messages_total": profile.messages_total,
                    "threads_total": profile.threads_total,
                    "history_id": profile.history_id,
                }),
                status.format,
            )?;
            Ok(())
        }
    }
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
            anyhow::bail!("device code expired; rerun `grr auth login --device`");
        }
        if let Some(storage) = client.poll_device_code(&mut challenge).await? {
            print_output(
                &serde_json::json!({
                    "authenticated": true,
                    "token_backend": client.token_backend(),
                    "token_preview": token_preview(&storage.access_token)
                }),
                format,
            )?;
            return Ok(());
        }
    }
}

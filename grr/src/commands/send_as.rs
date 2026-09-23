//! Send-as CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_gmail::{CreateSendAsOptions, GmailClient, UpdateSendAsOptions};

#[derive(Subcommand, Debug)]
pub enum SendAsCommands {
    /// List all send-as aliases
    List(ListArgs),
    /// Get send-as alias by email
    Get(GetArgs),
    /// Create a new send-as alias
    Create(CreateArgs),
    /// Update a send-as alias
    Update(UpdateArgs),
    /// Delete a send-as alias
    Delete(DeleteArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Send-as email address
    pub send_as_email: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Send-as email address
    pub send_as_email: String,

    /// Display name
    #[arg(long)]
    pub display_name: Option<String>,

    /// Reply-to address
    #[arg(long)]
    pub reply_to: Option<String>,

    /// Signature
    #[arg(long)]
    pub signature: Option<String>,

    /// Treat as alias
    #[arg(long)]
    pub treat_as_alias: Option<bool>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Send-as email address
    pub send_as_email: String,

    /// Display name
    #[arg(long)]
    pub display_name: Option<String>,

    /// Reply-to address
    #[arg(long)]
    pub reply_to: Option<String>,

    /// Signature
    #[arg(long)]
    pub signature: Option<String>,

    /// Treat as alias
    #[arg(long)]
    pub treat_as_alias: Option<bool>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Send-as email address
    pub send_as_email: String,
}

pub async fn handle_send_as_cmd(client: &GmailClient, cmd: SendAsCommands) -> Result<()> {
    match cmd {
        SendAsCommands::List(args) => {
            let send_as = client.list_send_as().await?;
            print_output(&send_as, args.format)?;
        }
        SendAsCommands::Get(args) => {
            let send_as = client.get_send_as(&args.send_as_email).await?;
            print_output(&send_as, args.format)?;
        }
        SendAsCommands::Create(args) => {
            let options = CreateSendAsOptions {
                send_as_email: args.send_as_email,
                display_name: args.display_name,
                reply_to_address: args.reply_to,
                signature: args.signature,
                treat_as_alias: args.treat_as_alias,
            };
            let send_as = client.create_send_as(options).await?;
            print_output(&send_as, args.format)?;
        }
        SendAsCommands::Update(args) => {
            let options = UpdateSendAsOptions {
                display_name: args.display_name,
                reply_to_address: args.reply_to,
                signature: args.signature,
                treat_as_alias: args.treat_as_alias,
            };
            let send_as = client.update_send_as(&args.send_as_email, options).await?;
            print_output(&send_as, args.format)?;
        }
        SendAsCommands::Delete(args) => {
            client.delete_send_as(&args.send_as_email).await?;
            println!("Send-as alias {} deleted", args.send_as_email);
        }
    }
    Ok(())
}

//! Google Chat-related CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_chat::ChatClient;

#[derive(Subcommand, Debug)]
pub enum ChatCommands {
    /// List spaces the user belongs to
    Spaces(SpacesArgs),
    /// Show a single space by ID
    Space(SpaceArgs),
    /// List messages in a space
    Messages(MessagesArgs),
    /// Send a text message to a space
    Send(SendArgs),
}

#[derive(Args, Debug)]
pub struct SpacesArgs {
    /// Maximum number of spaces
    #[arg(short, long, default_value = "25")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SpaceArgs {
    /// Space ID (`spaces/AAA` or a bare `AAA`)
    pub space_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct MessagesArgs {
    /// Space ID (`spaces/AAA` or a bare `AAA`)
    pub space_id: String,

    /// Maximum number of messages
    #[arg(short, long, default_value = "25")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Space ID (`spaces/AAA` or a bare `AAA`)
    pub space_id: String,

    /// Message text
    #[arg(short, long)]
    pub text: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_chat_cmd(client: &ChatClient, cmd: ChatCommands) -> Result<()> {
    match cmd {
        ChatCommands::Spaces(args) => {
            let spaces = client.list_spaces(Some(args.max)).await?;
            print_output(&spaces, args.format)?;
        }
        ChatCommands::Space(args) => {
            let space = client.get_space(&args.space_id).await?;
            print_output(&space, args.format)?;
        }
        ChatCommands::Messages(args) => {
            let messages = client.list_messages(&args.space_id, Some(args.max)).await?;
            print_output(&messages, args.format)?;
        }
        ChatCommands::Send(args) => {
            anyhow::ensure!(!args.text.trim().is_empty(), "--text must not be empty");
            let message = client.send_message(&args.space_id, &args.text).await?;
            print_output(&message, args.format)?;
        }
    }
    Ok(())
}

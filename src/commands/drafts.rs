//! Draft-related CLI commands

use crate::gmail::GmailClient;
use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum DraftCommands {
    /// Create a new draft
    Create(CreateArgs),
    /// List drafts
    List(ListArgs),
    /// Get draft by ID
    Get(GetArgs),
    /// Update a draft
    Update(UpdateArgs),
    /// Delete a draft
    Delete(DeleteArgs),
    /// Send a draft
    Send(SendArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Recipient email
    pub to: String,

    /// Subject
    pub subject: String,

    /// Body text
    pub body: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Maximum number of results
    #[arg(short, long, default_value = "10")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Draft ID
    pub draft_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Draft ID
    pub draft_id: String,

    /// Recipient email
    pub to: String,

    /// Subject
    pub subject: String,

    /// Body text
    pub body: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Draft ID
    pub draft_id: String,
}

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Draft ID
    pub draft_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_draft_cmd(client: &GmailClient, cmd: DraftCommands) -> Result<()> {
    match cmd {
        DraftCommands::Create(args) => {
            let draft = client
                .create_draft(&args.to, &args.subject, &args.body)
                .await?;
            print_output(&draft, args.format)?;
        }
        DraftCommands::List(args) => {
            let drafts = client.list_drafts(args.max).await?;
            print_output(&drafts, args.format)?;
        }
        DraftCommands::Get(args) => {
            let draft = client.get_draft(&args.draft_id).await?;
            print_output(&draft, args.format)?;
        }
        DraftCommands::Update(args) => {
            let draft = client
                .update_draft(&args.draft_id, &args.to, &args.subject, &args.body)
                .await?;
            print_output(&draft, args.format)?;
        }
        DraftCommands::Delete(args) => {
            client.delete_draft(&args.draft_id).await?;
            println!("Draft {} deleted", args.draft_id);
        }
        DraftCommands::Send(args) => {
            let msg = client.send_draft(&args.draft_id).await?;
            print_output(&msg, args.format)?;
        }
    }
    Ok(())
}

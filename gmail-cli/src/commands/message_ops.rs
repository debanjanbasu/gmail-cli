//! Message operations CLI commands (trash, delete, label, batch)

use crate::output::{print_output, OutputFormat};
use gmail_core::GmailClient;
use clap::{Args, Subcommand};
use anyhow::Result;

#[derive(Subcommand, Debug)]
pub enum MessageOpsCommands {
    /// Add/remove labels on a message
    Label(LabelArgs),
    /// Move message to trash
    Trash(TrashArgs),
    /// Restore message from trash
    Untrash(UntrashArgs),
    /// Permanently delete a message
    Delete(DeleteArgs),
    /// Batch modify labels on multiple messages
    BatchLabel(BatchLabelArgs),
    /// Batch delete multiple messages
    BatchDelete(BatchDeleteArgs),
}

#[derive(Args, Debug)]
pub struct LabelArgs {
    /// Message ID
    pub message_id: String,
    
    /// Labels to add (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub add: Vec<String>,
    
    /// Labels to remove (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub remove: Vec<String>,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct TrashArgs {
    /// Message ID
    pub message_id: String,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UntrashArgs {
    /// Message ID
    pub message_id: String,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Message ID
    pub message_id: String,
}

#[derive(Args, Debug)]
pub struct BatchLabelArgs {
    /// Message IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub ids: Vec<String>,
    
    /// Labels to add (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub add: Vec<String>,
    
    /// Labels to remove (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub remove: Vec<String>,
}

#[derive(Args, Debug)]
pub struct BatchDeleteArgs {
    /// Message IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub ids: Vec<String>,
}

pub async fn handle_message_ops_cmd(
    client: &GmailClient,
    cmd: MessageOpsCommands,
) -> Result<()> {
    match cmd {
        MessageOpsCommands::Label(args) => {
            let msg = client.modify_labels(&args.message_id, &args.add, &args.remove).await?;
            print_output(&msg, args.format)?;
        }
        MessageOpsCommands::Trash(args) => {
            let msg = client.trash_message(&args.message_id).await?;
            print_output(&msg, args.format)?;
        }
        MessageOpsCommands::Untrash(args) => {
            let msg = client.untrash_message(&args.message_id).await?;
            print_output(&msg, args.format)?;
        }
        MessageOpsCommands::Delete(args) => {
            client.delete_message(&args.message_id).await?;
            println!("Message {} permanently deleted", args.message_id);
        }
        MessageOpsCommands::BatchLabel(args) => {
            client.batch_modify_labels(&args.ids, &args.add, &args.remove).await?;
            println!("Labels updated for {} messages", args.ids.len());
        }
        MessageOpsCommands::BatchDelete(args) => {
            client.batch_delete_messages(&args.ids).await?;
            println!("{} messages permanently deleted", args.ids.len());
        }
    }
    Ok(())
}
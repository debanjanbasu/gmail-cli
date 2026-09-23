//! Message operations CLI commands (trash, delete, label, batch)

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_gmail::GmailClient;

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
    #[arg(long, value_delimiter = ',', conflicts_with = "search")]
    pub ids: Vec<String>,

    /// Resolve IDs from a Gmail query instead of --ids
    /// (e.g. --search "from:linkedin.com" --remove INBOX archives them all)
    #[arg(long, conflicts_with = "ids")]
    pub search: Option<String>,

    /// Cap matched messages when using --search
    #[arg(short, long, default_value = "10000", requires = "search")]
    pub max: usize,

    /// Report what would change, change nothing (needs --search or --ids)
    #[arg(long)]
    pub dry_run: bool,

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
    #[arg(long, value_delimiter = ',', conflicts_with = "search")]
    pub ids: Vec<String>,

    /// Resolve IDs from a Gmail query instead of --ids
    #[arg(long, conflicts_with = "ids")]
    pub search: Option<String>,

    /// Cap matched messages when using --search
    #[arg(short, long, default_value = "10000", requires = "search")]
    pub max: usize,

    /// Report what would change, change nothing (needs --search or --ids)
    #[arg(long)]
    pub dry_run: bool,
}

/// The Gmail batchModify/batchDelete endpoints cap one call at 1000 IDs.
const BATCH_API_CHUNK: usize = 1000;

/// Collect IDs either from --ids or by running a query to completion.
/// Warns (to stderr) when a --search hits the --max cap, because that
/// means the operation did not cover the whole mailbox match set.
async fn resolve_ids(
    client: &GmailClient,
    ids: Vec<String>,
    search: Option<&str>,
    max: usize,
) -> Result<Vec<String>> {
    if let Some(query) = search {
        let matched = client.search(query, max).await?;
        if matched.len() >= max {
            eprintln!("warning: matched at least {max} messages; raise --max or narrow the query");
        }
        Ok(matched.into_iter().map(|m| m.id).collect())
    } else {
        Ok(ids)
    }
}

pub async fn handle_message_ops_cmd(client: &GmailClient, cmd: MessageOpsCommands) -> Result<()> {
    match cmd {
        MessageOpsCommands::Label(args) => {
            let msg = client
                .modify_labels(&args.message_id, &args.add, &args.remove)
                .await?;
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
            let ids = resolve_ids(client, args.ids, args.search.as_deref(), args.max).await?;
            if args.dry_run {
                println!(
                    "dry-run: would update labels ({:+} / {:-}) on {} messages",
                    args.add.len(),
                    args.remove.len(),
                    ids.len()
                );
                return Ok(());
            }
            for chunk in ids.chunks(BATCH_API_CHUNK) {
                client
                    .batch_modify_labels(chunk, &args.add, &args.remove)
                    .await?;
            }
            println!("Labels updated for {} messages", ids.len());
        }
        MessageOpsCommands::BatchDelete(args) => {
            let ids = resolve_ids(client, args.ids, args.search.as_deref(), args.max).await?;
            if args.dry_run {
                println!("dry-run: would permanently delete {} messages", ids.len());
                return Ok(());
            }
            for chunk in ids.chunks(BATCH_API_CHUNK) {
                client.batch_delete_messages(chunk).await?;
            }
            println!("{} messages permanently deleted", ids.len());
        }
    }
    Ok(())
}

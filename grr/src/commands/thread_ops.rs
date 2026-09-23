//! Thread operations CLI commands

use anyhow::Result;
use clap::{Args, Subcommand};
use grr_gmail::GmailClient;

#[derive(Subcommand, Debug)]
pub enum ThreadCommands {
    /// Modify labels on a thread
    Label(LabelArgs),
    /// Move thread to trash
    Trash(TrashArgs),
    /// Restore thread from trash
    Untrash(UntrashArgs),
    /// Permanently delete a thread
    Delete(DeleteArgs),
}

#[derive(Args, Debug)]
pub struct LabelArgs {
    /// Thread ID
    pub thread_id: String,

    /// Labels to add (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub add: Vec<String>,

    /// Labels to remove (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub remove: Vec<String>,
}

#[derive(Args, Debug)]
pub struct TrashArgs {
    /// Thread ID
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct UntrashArgs {
    /// Thread ID
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Thread ID
    pub thread_id: String,
}

pub async fn handle_thread_cmd(client: &GmailClient, cmd: ThreadCommands) -> Result<()> {
    match cmd {
        ThreadCommands::Label(args) => {
            client
                .modify_thread_labels(&args.thread_id, &args.add, &args.remove)
                .await?;
            println!("Labels updated for thread {}", args.thread_id);
        }
        ThreadCommands::Trash(args) => {
            client.trash_thread(&args.thread_id).await?;
            println!("Thread {} moved to trash", args.thread_id);
        }
        ThreadCommands::Untrash(args) => {
            client.untrash_thread(&args.thread_id).await?;
            println!("Thread {} restored from trash", args.thread_id);
        }
        ThreadCommands::Delete(args) => {
            client.delete_thread(&args.thread_id).await?;
            println!("Thread {} permanently deleted", args.thread_id);
        }
    }
    Ok(())
}

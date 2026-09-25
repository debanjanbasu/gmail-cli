//! Message operations, filters, and mail settings CLI commands

use crate::gmail::{
    Filter, FilterAction, FilterCriteria, ForwardingAddress, GmailClient, PopSettings,
};
use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
#[command(about = "Message operations, filters, and mail settings")]
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
    #[command(about = "List Gmail filters")]
    FilterList(FilterListArgs),
    #[command(about = "Get a Gmail filter")]
    Filter(FilterGetArgs),
    #[command(about = "Create a Gmail filter")]
    FilterCreate(FilterCreateArgs),
    #[command(about = "Delete a Gmail filter")]
    FilterDelete(FilterDeleteArgs),
    #[command(about = "List forwarding addresses")]
    ForwardingList(ForwardingListArgs),
    #[command(about = "Create a forwarding address")]
    ForwardingCreate(ForwardingCreateArgs),
    #[command(about = "Delete a forwarding address")]
    ForwardingDelete(ForwardingDeleteArgs),
    #[command(about = "Show auto-forwarding settings")]
    Autoforwarding(SettingReadArgs),
    #[command(about = "Set auto-forwarding")]
    AutoforwardingSet(AutoForwardingSetArgs),
    #[command(about = "Show POP settings")]
    Pop(SettingReadArgs),
    #[command(about = "Set POP settings")]
    PopSet(PopSetArgs),
    #[command(about = "Show IMAP settings")]
    Imap(SettingReadArgs),
    #[command(about = "Set IMAP settings")]
    ImapSet(ImapSetArgs),
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

#[derive(Args, Debug)]
pub struct FilterListArgs {
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct FilterGetArgs {
    pub id: String,
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct FilterCreateArgs {
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub subject: Option<String>,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long)]
    pub negated_query: Option<String>,
    #[arg(long)]
    pub has_attachment: Option<bool>,
    #[arg(long)]
    pub exclude_chats: Option<bool>,
    #[arg(long)]
    pub size: Option<u64>,
    #[arg(long, value_enum)]
    pub size_comparison: Option<FilterSizeComparison>,
    #[arg(long, value_delimiter = ',')]
    pub add_labels: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub remove_labels: Vec<String>,
    #[arg(long)]
    pub forward: Option<String>,
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct FilterDeleteArgs {
    pub id: String,
}

#[derive(Args, Debug)]
pub struct ForwardingListArgs {
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ForwardingCreateArgs {
    #[arg(long)]
    pub email: String,
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ForwardingDeleteArgs {
    pub email: String,
}

#[derive(Args, Debug)]
pub struct SettingReadArgs {
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct AutoForwardingSetArgs {
    #[arg(
        long,
        value_name = "true|false",
        action = clap::ArgAction::Set
    )]
    pub enabled: bool,

    #[arg(long)]
    pub email: Option<String>,

    #[arg(long, value_enum)]
    pub disposition: Option<MailDisposition>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct PopSetArgs {
    #[arg(long, value_enum)]
    pub access_window: PopAccessWindow,

    #[arg(long, value_enum)]
    pub disposition: Option<MailDisposition>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ImapSetArgs {
    #[arg(
        long,
        value_name = "true|false",
        action = clap::ArgAction::Set
    )]
    pub enabled: Option<bool>,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub auto_expunge: bool,

    #[arg(long, value_enum)]
    pub expunge_behavior: Option<ImapExpungeBehavior>,

    #[arg(long)]
    pub max_folder_size: Option<i64>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum FilterSizeComparison {
    Unspecified,
    Smaller,
    Larger,
}

impl FilterSizeComparison {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Smaller => "smaller",
            Self::Larger => "larger",
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum MailDisposition {
    #[value(name = "leaveInInbox")]
    LeaveInInbox,
    Archive,
    Trash,
    #[value(name = "markRead")]
    MarkRead,
}

impl MailDisposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::LeaveInInbox => "leaveInInbox",
            Self::Archive => "archive",
            Self::Trash => "trash",
            Self::MarkRead => "markRead",
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum PopAccessWindow {
    #[value(name = "disabled")]
    Disabled,
    #[value(name = "fromNowOn")]
    FromNowOn,
    #[value(name = "allMail")]
    AllMail,
}

impl PopAccessWindow {
    fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::FromNowOn => "fromNowOn",
            Self::AllMail => "allMail",
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum ImapExpungeBehavior {
    #[value(name = "archive")]
    Archive,
    #[value(name = "trash")]
    Trash,
    #[value(name = "deleteForever")]
    DeleteForever,
}

impl ImapExpungeBehavior {
    fn as_str(self) -> &'static str {
        match self {
            Self::Archive => "archive",
            Self::Trash => "trash",
            Self::DeleteForever => "deleteForever",
        }
    }
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
        MessageOpsCommands::FilterList(args) => {
            print_output(&client.list_filters().await?, args.format)?;
        }
        MessageOpsCommands::Filter(args) => {
            print_output(&client.get_filter(&args.id).await?, args.format)?;
        }
        MessageOpsCommands::FilterCreate(args) => {
            let filter = Filter {
                criteria: FilterCriteria {
                    from: args.from,
                    to: args.to,
                    subject: args.subject,
                    query: args.query,
                    negated_query: args.negated_query,
                    has_attachment: args.has_attachment,
                    exclude_chats: args.exclude_chats,
                    size: args.size,
                    size_comparison: args.size_comparison.map(|value| value.as_str().to_string()),
                },
                action: FilterAction {
                    add_label_ids: (!args.add_labels.is_empty()).then_some(args.add_labels),
                    remove_label_ids: (!args.remove_labels.is_empty())
                        .then_some(args.remove_labels),
                    forward: args.forward,
                },
                ..Filter::default()
            };
            print_output(&client.create_filter(filter).await?, args.format)?;
        }
        MessageOpsCommands::FilterDelete(args) => {
            client.delete_filter(&args.id).await?;
            println!("Filter {} deleted", args.id);
        }
        MessageOpsCommands::ForwardingList(args) => {
            print_output(&client.list_forwarding_addresses().await?, args.format)?;
        }
        MessageOpsCommands::ForwardingCreate(args) => {
            let address = ForwardingAddress {
                forwarding_email: args.email,
                verification_status: None,
            };
            print_output(
                &client.create_forwarding_address(address).await?,
                args.format,
            )?;
        }
        MessageOpsCommands::ForwardingDelete(args) => {
            client.delete_forwarding_address(&args.email).await?;
            println!("Forwarding address {} deleted", args.email);
        }
        MessageOpsCommands::Autoforwarding(args) => {
            print_output(&client.get_auto_forwarding().await?, args.format)?;
        }
        MessageOpsCommands::AutoforwardingSet(args) => {
            let mut settings = client.get_auto_forwarding().await?;
            settings.enabled = args.enabled;
            if let Some(email) = args.email {
                settings.email_address = email;
            }
            if let Some(disposition) = args.disposition {
                settings.disposition = Some(disposition.as_str().to_string());
            }
            print_output(&client.update_auto_forwarding(settings).await?, args.format)?;
        }
        MessageOpsCommands::Pop(args) => {
            print_output(&client.get_pop_settings().await?, args.format)?;
        }
        MessageOpsCommands::PopSet(args) => {
            let settings = PopSettings {
                access_window: Some(args.access_window.as_str().to_string()),
                disposition: args.disposition.map(|value| value.as_str().to_string()),
            };
            print_output(&client.update_pop_settings(settings).await?, args.format)?;
        }
        MessageOpsCommands::Imap(args) => {
            print_output(&client.get_imap_settings().await?, args.format)?;
        }
        MessageOpsCommands::ImapSet(args) => {
            anyhow::ensure!(
                args.enabled.is_some()
                    || args.auto_expunge
                    || args.expunge_behavior.is_some()
                    || args.max_folder_size.is_some(),
                "at least one IMAP setting must be supplied"
            );
            let mut settings = client.get_imap_settings().await?;
            if let Some(enabled) = args.enabled {
                settings.enabled = enabled;
            }
            if args.auto_expunge {
                settings.auto_expunge = Some(true);
            }
            if let Some(expunge_behavior) = args.expunge_behavior {
                settings.expunge_behavior = Some(expunge_behavior.as_str().to_string());
            }
            if let Some(max_folder_size) = args.max_folder_size {
                settings.max_folder_size = Some(max_folder_size);
            }
            print_output(&client.update_imap_settings(settings).await?, args.format)?;
        }
    }
    Ok(())
}

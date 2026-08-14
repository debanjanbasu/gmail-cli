//! Message-related CLI commands

use crate::output::{print_output, OutputFormat};
use gmail_core::GmailClient;
use clap::{Args, Subcommand};
use anyhow::Result;
use base64::Engine;

#[derive(Subcommand, Debug)]
pub enum MessageCommands {
    /// Search emails using Gmail query syntax
    Search(SearchArgs),
    /// Get full thread by ID
    Thread(ThreadArgs),
    /// Get single message by ID
    Get(GetArgs),
    /// Download attachment
    Attachment(AttachmentArgs),
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Gmail search query (e.g., "in:inbox from:github.com")
    pub query: String,
    
    /// Maximum number of results
    #[arg(short, long, default_value = "10")]
    pub max: usize,
    
    /// Page token for pagination
    #[arg(long)]
    pub page: Option<String>,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ThreadArgs {
    /// Thread ID
    pub thread_id: String,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Message ID
    pub message_id: String,
    
    /// Message format
    #[arg(short, long, value_enum, default_value = "full")]
    pub format: MessageFormat,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub output: OutputFormat,
}

#[derive(Args, Debug)]
pub struct AttachmentArgs {
    /// Message ID
    pub message_id: String,
    
    /// Attachment ID
    pub attachment_id: String,
    
    /// Output file path (optional, prints base64 to stdout if not provided)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum MessageFormat {
    #[default]
    Full,
    Metadata,
    Minimal,
    Raw,
}

pub async fn handle_message_cmd(
    client: &GmailClient,
    cmd: MessageCommands,
) -> Result<()> {
    match cmd {
        MessageCommands::Search(args) => {
            let results = client.search(&args.query, args.max).await?;
            print_output(&results, args.format)?;
        }
        MessageCommands::Thread(args) => {
            let thread = client.get_thread(&args.thread_id).await?;
            print_output(&thread, args.format)?;
        }
        MessageCommands::Get(args) => {
            let fmt = match args.format {
                MessageFormat::Full => Some("full"),
                MessageFormat::Metadata => Some("metadata"),
                MessageFormat::Minimal => Some("minimal"),
                MessageFormat::Raw => Some("raw"),
            };
            if matches!(args.format, MessageFormat::Raw) {
                let raw = client.get_message_raw(&args.message_id).await?;
                println!("{}", raw);
            } else {
                let msg = client.get_message(&args.message_id, fmt).await?;
                print_output(&msg, args.output)?;
            }
        }
        MessageCommands::Attachment(args) => {
            let attachment = client.get_attachment(&args.message_id, &args.attachment_id).await?;
            if let Some(path) = args.output {
                let data = base64::engine::general_purpose::STANDARD.decode(&attachment.data)?;
                tokio::fs::write(&path, data).await?;
                println!("Attachment saved to {}", path);
            } else {
                println!("{}", attachment.data);
            }
        }
    }
    Ok(())
}
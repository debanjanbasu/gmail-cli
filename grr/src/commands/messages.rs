//! Message-related CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use base64::Engine;
use clap::{Args, Subcommand};
use futures::future::join_all;
use grr_core::error::GrrError;
use grr_gmail::{GmailClient, extract_body};

#[derive(Subcommand, Debug)]
pub enum MessageCommands {
    /// Search emails using Gmail query syntax
    Search(SearchArgs),
    /// Get full thread by ID
    Thread(ThreadArgs),
    /// Get single message by ID
    Get(GetArgs),
    /// Fetch multiple messages in parallel
    BatchRead(BatchReadArgs),
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

    /// Include the decoded message body (forces full format)
    #[arg(long)]
    pub body: bool,

    /// Truncate --body to at most this many characters
    #[arg(long, default_value = "800", requires = "body")]
    pub max_length: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub output: OutputFormat,
}

#[derive(Args, Debug)]
pub struct BatchReadArgs {
    /// Message IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub ids: Vec<String>,

    /// Include the decoded message body of each message
    #[arg(long)]
    pub body: bool,

    /// Truncate each --body to at most this many characters
    #[arg(long, default_value = "800", requires = "body")]
    pub max_length: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
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

/// Truncate a body for agent consumption, with an explicit marker.
fn truncate_body(body: &str, max_length: usize) -> String {
    let mut chars = body.chars();
    let mut out: String = chars.by_ref().take(max_length).collect();
    if chars.next().is_some() {
        out.push_str("...[truncated]");
    }
    out
}

/// Body of a message as JSON (null when absent), truncated to max_length.
fn body_json(msg: &grr_gmail::Message, max_length: usize) -> serde_json::Value {
    match extract_body(msg) {
        Some(text) => serde_json::json!(truncate_body(&text, max_length)),
        None => serde_json::Value::Null,
    }
}

pub async fn handle_message_cmd(client: &GmailClient, cmd: MessageCommands) -> Result<()> {
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
                let raw = client.get_message_raw_bytes(&args.message_id).await?;
                println!("{}", String::from_utf8_lossy(&raw));
            } else if args.body {
                // Bodies only exist in the full representation.
                let msg = client.get_message(&args.message_id, Some("full")).await?;
                print_output(
                    &serde_json::json!({
                        "message": msg,
                        "body": body_json(&msg, args.max_length),
                    }),
                    args.output,
                )?;
            } else {
                let msg = client.get_message(&args.message_id, fmt).await?;
                print_output(&msg, args.output)?;
            }
        }
        MessageCommands::BatchRead(args) => {
            // Concurrency is bounded inside the client (in-flight request
            // valve); this just fans out and preserves input order.
            let messages = join_all(args.ids.iter().map(|id| {
                let client = client.clone();
                async move { client.get_message(id, Some("full")).await }
            }))
            .await
            .into_iter()
            .collect::<Result<Vec<_>, GrrError>>()?;
            let out: Vec<serde_json::Value> = messages
                .iter()
                .map(|msg| {
                    if args.body {
                        serde_json::json!({
                            "message": msg,
                            "body": body_json(msg, args.max_length),
                        })
                    } else {
                        serde_json::to_value(msg).unwrap_or_default()
                    }
                })
                .collect();
            print_output(&out, args.format)?;
        }
        MessageCommands::Attachment(args) => {
            if let Some(path) = args.output {
                let n = client
                    .download_attachment_to(
                        &args.message_id,
                        &args.attachment_id,
                        std::path::Path::new(&path),
                    )
                    .await?;
                println!("Saved {n} bytes to {path}");
            } else {
                let data = client
                    .get_attachment_bytes(&args.message_id, &args.attachment_id)
                    .await?;
                println!(
                    "{}",
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&data)
                );
            }
        }
    }
    Ok(())
}

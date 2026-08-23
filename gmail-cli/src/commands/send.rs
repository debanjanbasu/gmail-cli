//! Send email CLI commands

use crate::output::{print_output, OutputFormat};
use gmail_core::client::{StreamAttachment, mime_message_stream};
use gmail_core::GmailClient;
use clap::{Args, Subcommand};
use anyhow::Result;
use std::path::Path;

#[derive(Subcommand, Debug)]
pub enum SendCommands {
    /// Send an email
    Send(SendArgs),
    /// Send an email with attachments
    SendAttach(SendAttachArgs),
}

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Recipient email
    pub to: String,
    
    /// Subject
    pub subject: String,
    
    /// Body text
    pub body: String,
    
    /// CC recipients (comma-separated)
    #[arg(long)]
    pub cc: Option<String>,
    
    /// BCC recipients (comma-separated)
    #[arg(long)]
    pub bcc: Option<String>,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SendAttachArgs {
    /// Recipient email
    pub to: String,
    
    /// Subject
    pub subject: String,
    
    /// Body text
    pub body: String,
    
    /// Attachment file paths (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub attachments: Vec<String>,
    
    /// Thread ID to reply to (optional)
    #[arg(long)]
    pub thread_id: Option<String>,
    
    /// CC recipients (comma-separated)
    #[arg(long)]
    pub cc: Option<String>,
    
    /// BCC recipients (comma-separated)
    #[arg(long)]
    pub bcc: Option<String>,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_send_cmd(
    client: &GmailClient,
    cmd: SendCommands,
) -> Result<()> {
    match cmd {
        SendCommands::Send(args) => {
            let msg = if args.cc.is_some() || args.bcc.is_some() {
                client.send_with_options(
                    &args.to,
                    &args.subject,
                    &args.body,
                    args.cc.as_deref(),
                    args.bcc.as_deref(),
                ).await?
            } else {
                client.send(&args.to, &args.subject, &args.body).await?
            };
            print_output(&msg, args.format)?;
        }
        SendCommands::SendAttach(args) => {
            let attachments: Vec<StreamAttachment> = args
                .attachments
                .iter()
                .map(|path_str| {
                    let path = Path::new(path_str);
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("attachment")
                        .to_string();

                    // Determine MIME type from extension
                    let mime_type = mime_guess::from_path(path)
                        .first()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string());

                    StreamAttachment {
                        path: path.to_path_buf(),
                        filename,
                        mime_type,
                    }
                })
                .collect();

            let stream = mime_message_stream(
                &args.to,
                &args.subject,
                &args.body,
                attachments,
                args.thread_id.as_deref(),
            );

            let msg = client
                .send_mime_stream(stream, args.thread_id.as_deref())
                .await?;

            print_output(&msg, args.format)?;
        }
    }
    Ok(())
}
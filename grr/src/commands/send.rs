//! Send email CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_gmail::AttachmentData;
use grr_gmail::GmailClient;
use grr_gmail::client::{StreamAttachment, mime_message_stream};
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

/// Routing decision for send-with-attachments.
///
/// The streaming media-upload endpoint (`uploadType=media`) posts raw
/// RFC822 bytes and cannot carry a `threadId`; only the legacy JSON
/// `SendMessageRequest` path threads the message. Legacy is therefore
/// chosen **iff** a thread id was requested; streaming stays the default
/// otherwise.
fn uses_legacy_attachment_path(thread_id: Option<&str>) -> bool {
    thread_id.is_some()
}

fn attachment_filename(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string()
}

fn attachment_mime_type(path: &Path) -> String {
    mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

pub async fn handle_send_cmd(client: &GmailClient, cmd: SendCommands) -> Result<()> {
    match cmd {
        SendCommands::Send(args) => {
            let msg = if args.cc.is_some() || args.bcc.is_some() {
                client
                    .send_with_options(
                        &args.to,
                        &args.subject,
                        &args.body,
                        args.cc.as_deref(),
                        args.bcc.as_deref(),
                    )
                    .await?
            } else {
                client.send(&args.to, &args.subject, &args.body).await?
            };
            print_output(&msg, args.format)?;
        }
        SendCommands::SendAttach(args) => {
            let msg = if uses_legacy_attachment_path(args.thread_id.as_deref()) {
                let mut attachments: Vec<AttachmentData> =
                    Vec::with_capacity(args.attachments.len());
                for path_str in &args.attachments {
                    let path = Path::new(path_str);
                    attachments.push(AttachmentData {
                        content: tokio::fs::read(path).await?,
                        filename: attachment_filename(path),
                        mime_type: attachment_mime_type(path),
                    });
                }
                client
                    .send_with_attachments(
                        &args.to,
                        &args.subject,
                        &args.body,
                        attachments,
                        args.thread_id.as_deref(),
                    )
                    .await?
            } else {
                let attachments: Vec<StreamAttachment> = args
                    .attachments
                    .iter()
                    .map(|path_str| {
                        let path = Path::new(path_str);
                        StreamAttachment {
                            path: path.to_path_buf(),
                            filename: attachment_filename(path),
                            mime_type: attachment_mime_type(path),
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

                client
                    .send_mime_stream(stream, args.thread_id.as_deref())
                    .await?
            };

            print_output(&msg, args.format)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_attachment_path_chosen_iff_thread_id_present() {
        // Streaming media-upload drops threadId on the floor, so any
        // --thread_id request MUST take the legacy JSON send path.
        assert!(!uses_legacy_attachment_path(None));
        assert!(uses_legacy_attachment_path(Some("thread-abc123")));
    }
}

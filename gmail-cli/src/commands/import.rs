//! Import CLI commands

use crate::output::{print_output, OutputFormat};
use gmail_core::GmailClient;
use clap::Args;
use anyhow::Result;
use std::path::Path;
use tokio_util::io::ReaderStream;

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Path to RFC 822 message file
    pub file: String,

    /// Mark as deleted (in trash)
    #[arg(long)]
    pub deleted: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_import_cmd(
    client: &GmailClient,
    args: ImportArgs,
) -> Result<()> {
    let path = Path::new(&args.file);
    let file = tokio::fs::File::open(path).await?;

    let msg = client.import_stream(ReaderStream::new(file), args.deleted).await?;
    print_output(&msg, args.format)?;
    Ok(())
}

//! Drive-related CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_drive::{DriveClient, FileListOptions};
use std::path::Path;

#[derive(Subcommand, Debug)]
pub enum DriveCommands {
    /// List files on the user's Drive
    List(ListArgs),
    /// Search files with a Drive query
    Search(SearchArgs),
    /// Get a single file's metadata by ID
    Get(GetArgs),
    /// Download a file's content to disk
    Download(DownloadArgs),
    /// Upload a local file to Drive
    Upload(UploadArgs),
    /// Rename a file
    Rename(RenameArgs),
    /// Delete a file
    Delete(DeleteArgs),
    /// Show account storage quota
    Quota(QuotaArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Drive query (e.g. "name contains 'report'"); defaults to non-trashed files
    #[arg(short, long)]
    pub query: Option<String>,

    /// Maximum number of files
    #[arg(short, long, default_value = "25")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Drive query (e.g. "name contains 'report'", "trashed = false")
    #[arg(short, long)]
    pub query: String,

    /// Maximum number of files
    #[arg(short, long, default_value = "25")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// File ID
    pub file_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DownloadArgs {
    /// File ID
    pub file_id: String,

    /// Destination path
    #[arg(short, long)]
    pub output: String,
}

#[derive(Args, Debug)]
pub struct UploadArgs {
    /// Local file to upload
    pub file: String,

    /// Uploaded file name (defaults to the local file name)
    #[arg(long)]
    pub name: Option<String>,

    /// Parent folder ID (defaults to the My Drive root)
    #[arg(long)]
    pub parent: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    /// File ID
    pub file_id: String,

    /// New file name
    pub new_name: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// File ID
    pub file_id: String,
}

#[derive(Args, Debug)]
pub struct QuotaArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_drive_cmd(client: &DriveClient, cmd: DriveCommands) -> Result<()> {
    match cmd {
        DriveCommands::List(args) => {
            let opts = FileListOptions {
                q: Some(args.query.unwrap_or_else(|| "trashed = false".into())),
                max_results: args.max,
            };
            let files = client.list_files(opts).await?;
            print_output(&files, args.format)?;
        }
        DriveCommands::Search(args) => {
            let opts = FileListOptions {
                q: Some(args.query),
                max_results: args.max,
            };
            let files = client.list_files(opts).await?;
            print_output(&files, args.format)?;
        }
        DriveCommands::Get(args) => {
            let file = client.get_file(&args.file_id).await?;
            print_output(&file, args.format)?;
        }
        DriveCommands::Download(args) => {
            let written = client
                .download_file(&args.file_id, Path::new(&args.output))
                .await?;
            println!("Downloaded {written} bytes to {}", args.output);
        }
        DriveCommands::Upload(args) => {
            let file = client
                .upload_file(
                    Path::new(&args.file),
                    args.name.as_deref(),
                    args.parent.as_deref(),
                )
                .await?;
            print_output(&file, args.format)?;
        }
        DriveCommands::Rename(args) => {
            let file = client.rename_file(&args.file_id, &args.new_name).await?;
            print_output(&file, args.format)?;
        }
        DriveCommands::Delete(args) => {
            client.delete_file(&args.file_id).await?;
            println!("File {} deleted", args.file_id);
        }
        DriveCommands::Quota(args) => {
            let about = client.about().await?;
            print_output(&about, args.format)?;
        }
    }
    Ok(())
}

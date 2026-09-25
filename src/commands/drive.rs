//! Drive-related CLI commands

use crate::drive::{DriveClient, FileListOptions, PermissionGrant};
use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

#[derive(Subcommand, Debug)]
pub enum DriveCommands {
    List(ListArgs),
    Search(SearchArgs),
    Get(GetArgs),
    Mkdir(MkdirArgs),
    Trash(TrashArgs),
    Restore(RestoreArgs),
    Copy(CopyArgs),
    EmptyTrash,
    Download(DownloadArgs),
    Upload(UploadArgs),
    Rename(RenameArgs),
    Delete(DeleteArgs),
    Export(ExportArgs),
    Share(ShareArgs),
    Shares(SharesArgs),
    Unshare(UnshareArgs),
    Comments(CommentsArgs),
    Comment(CommentArgs),
    CommentAdd(CommentAddArgs),
    CommentDelete(CommentDeleteArgs),
    Revisions(RevisionsArgs),
    Revision(RevisionArgs),
    Quota(QuotaArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(short, long)]
    pub query: Option<String>,

    #[arg(long, conflicts_with = "query")]
    pub trashed: bool,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(short, long)]
    pub query: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    pub file_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct MkdirArgs {
    pub name: String,

    #[arg(long)]
    pub parent: Option<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct TrashArgs {
    pub file_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct RestoreArgs {
    pub file_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CopyArgs {
    pub file_id: String,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long)]
    pub parent: Option<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DownloadArgs {
    pub file_id: String,

    #[arg(short, long)]
    pub output: String,
}

#[derive(Args, Debug)]
pub struct UploadArgs {
    pub file: String,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long)]
    pub parent: Option<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    pub file_id: String,

    pub new_name: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    pub file_id: String,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    pub file_id: String,

    #[arg(long)]
    pub mime: String,

    #[arg(short, long)]
    pub output: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DriveShareRole {
    Reader,
    Writer,
    Commenter,
}

impl DriveShareRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Writer => "writer",
            Self::Commenter => "commenter",
        }
    }
}

#[derive(Args, Debug)]
pub struct ShareArgs {
    pub file_id: String,

    #[arg(
        long,
        required_unless_present_any = ["domain", "anyone"],
        conflicts_with_all = ["domain", "anyone"]
    )]
    pub email: Option<String>,

    #[arg(
        long,
        required_unless_present_any = ["email", "anyone"],
        conflicts_with_all = ["email", "anyone"]
    )]
    pub domain: Option<String>,

    #[arg(
        long,
        required_unless_present_any = ["email", "domain"],
        conflicts_with_all = ["email", "domain"]
    )]
    pub anyone: bool,

    #[arg(long, value_enum)]
    pub role: DriveShareRole,

    #[arg(long)]
    pub no_notify: bool,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SharesArgs {
    pub file_id: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UnshareArgs {
    pub file_id: String,

    pub permission_id: String,
}

#[derive(Args, Debug)]
pub struct CommentsArgs {
    pub file_id: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CommentArgs {
    pub file_id: String,

    pub comment_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CommentAddArgs {
    pub file_id: String,

    #[arg(long)]
    pub text: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CommentDeleteArgs {
    pub file_id: String,

    pub comment_id: String,
}

#[derive(Args, Debug)]
pub struct RevisionsArgs {
    pub file_id: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct RevisionArgs {
    pub file_id: String,

    pub revision_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct QuotaArgs {
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

async fn print_drive_output<T>(value: T, format: OutputFormat) -> Result<()>
where
    T: Serialize + Send + 'static,
{
    tokio::task::spawn_blocking(move || print_output(&value, format)).await??;
    Ok(())
}

async fn write_status(message: String) -> Result<()> {
    let mut stdout = tokio::io::stdout();
    stdout.write_all(message.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
    Ok(())
}

pub async fn handle_drive_cmd(client: &DriveClient, cmd: DriveCommands) -> Result<()> {
    match cmd {
        DriveCommands::List(args) => {
            let query = if args.trashed {
                "trashed = true".to_owned()
            } else {
                args.query.unwrap_or_else(|| "trashed = false".to_owned())
            };
            let files = client
                .list_files(FileListOptions {
                    q: Some(query),
                    max_results: args.max,
                })
                .await?;
            print_drive_output(files, args.format).await?;
        }
        DriveCommands::Search(args) => {
            let files = client
                .list_files(FileListOptions {
                    q: Some(args.query),
                    max_results: args.max,
                })
                .await?;
            print_drive_output(files, args.format).await?;
        }
        DriveCommands::Get(args) => {
            let file = client.get_file(&args.file_id).await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::Mkdir(args) => {
            let file = client
                .create_folder(&args.name, args.parent.as_deref())
                .await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::Trash(args) => {
            let file = client.trash_file(&args.file_id).await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::Restore(args) => {
            let file = client.restore_file(&args.file_id).await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::Copy(args) => {
            let file = client
                .copy_file(&args.file_id, args.name.as_deref(), args.parent.as_deref())
                .await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::EmptyTrash => {
            client.empty_trash().await?;
            write_status("Trash emptied".to_owned()).await?;
        }
        DriveCommands::Download(args) => {
            let written = client
                .download_file(&args.file_id, Path::new(&args.output))
                .await?;
            write_status(format!("Downloaded {written} bytes to {}", args.output)).await?;
        }
        DriveCommands::Upload(args) => {
            let file = client
                .upload_file(
                    Path::new(&args.file),
                    args.name.as_deref(),
                    args.parent.as_deref(),
                )
                .await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::Rename(args) => {
            let file = client.rename_file(&args.file_id, &args.new_name).await?;
            print_drive_output(file, args.format).await?;
        }
        DriveCommands::Delete(args) => {
            client.delete_file(&args.file_id).await?;
            write_status(format!("File {} deleted", args.file_id)).await?;
        }
        DriveCommands::Export(args) => {
            let written = client
                .export_file(&args.file_id, &args.mime, Path::new(&args.output))
                .await?;
            write_status(format!("Exported {written} bytes to {}", args.output)).await?;
        }
        DriveCommands::Share(args) => {
            let grant = if let Some(email_address) = args.email {
                PermissionGrant::User { email_address }
            } else if let Some(domain) = args.domain {
                PermissionGrant::Domain { domain }
            } else {
                PermissionGrant::Anyone
            };
            let permission = client
                .create_permission(&args.file_id, args.role.as_str(), &grant, !args.no_notify)
                .await?;
            print_drive_output(permission, args.format).await?;
        }
        DriveCommands::Shares(args) => {
            let permissions = client.list_permissions(&args.file_id, args.max).await?;
            print_drive_output(permissions, args.format).await?;
        }
        DriveCommands::Unshare(args) => {
            client
                .delete_permission(&args.file_id, &args.permission_id)
                .await?;
            write_status(format!(
                "Permission {} removed from {}",
                args.permission_id, args.file_id
            ))
            .await?;
        }
        DriveCommands::Comments(args) => {
            let comments = client.list_comments(&args.file_id, args.max).await?;
            print_drive_output(comments, args.format).await?;
        }
        DriveCommands::Comment(args) => {
            let comment = client.get_comment(&args.file_id, &args.comment_id).await?;
            print_drive_output(comment, args.format).await?;
        }
        DriveCommands::CommentAdd(args) => {
            let comment = client.create_comment(&args.file_id, &args.text).await?;
            print_drive_output(comment, args.format).await?;
        }
        DriveCommands::CommentDelete(args) => {
            client
                .delete_comment(&args.file_id, &args.comment_id)
                .await?;
            write_status(format!("Comment {} deleted", args.comment_id)).await?;
        }
        DriveCommands::Revisions(args) => {
            let revisions = client.list_revisions(&args.file_id, args.max).await?;
            print_drive_output(revisions, args.format).await?;
        }
        DriveCommands::Revision(args) => {
            let revision = client
                .get_revision(&args.file_id, &args.revision_id)
                .await?;
            print_drive_output(revision, args.format).await?;
        }
        DriveCommands::Quota(args) => {
            let about = client.about().await?;
            print_drive_output(about, args.format).await?;
        }
    }
    Ok(())
}

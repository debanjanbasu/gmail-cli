//! Label-related CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_core::{CreateLabelOptions, GmailClient, LabelColor, UpdateLabelOptions};

#[derive(Subcommand, Debug)]
pub enum LabelCommands {
    /// List all labels
    List(ListArgs),
    /// Get label by ID
    Get(GetArgs),
    /// Create a new label
    Create(CreateArgs),
    /// Update a label
    Update(UpdateArgs),
    /// Delete a label
    Delete(DeleteArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Label ID
    pub label_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Label name
    pub name: String,

    /// Label list visibility
    #[arg(long, value_enum)]
    pub label_list_visibility: Option<LabelListVisibility>,

    /// Message list visibility
    #[arg(long, value_enum)]
    pub message_list_visibility: Option<MessageListVisibility>,

    /// Background color (hex)
    #[arg(long)]
    pub color_bg: Option<String>,

    /// Text color (hex)
    #[arg(long)]
    pub color_text: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Label ID
    pub label_id: String,

    /// New label name
    #[arg(long)]
    pub name: Option<String>,

    /// Label list visibility
    #[arg(long, value_enum)]
    pub label_list_visibility: Option<LabelListVisibility>,

    /// Message list visibility
    #[arg(long, value_enum)]
    pub message_list_visibility: Option<MessageListVisibility>,

    /// Background color (hex)
    #[arg(long)]
    pub color_bg: Option<String>,

    /// Text color (hex)
    #[arg(long)]
    pub color_text: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Label ID
    pub label_id: String,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LabelListVisibility {
    LabelShow,
    LabelHide,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum MessageListVisibility {
    Show,
    Hide,
}

impl From<LabelListVisibility> for String {
    fn from(v: LabelListVisibility) -> Self {
        match v {
            LabelListVisibility::LabelShow => "labelShow".to_string(),
            LabelListVisibility::LabelHide => "labelHide".to_string(),
        }
    }
}

impl From<MessageListVisibility> for String {
    fn from(v: MessageListVisibility) -> Self {
        match v {
            MessageListVisibility::Show => "show".to_string(),
            MessageListVisibility::Hide => "hide".to_string(),
        }
    }
}

pub async fn handle_label_cmd(client: &GmailClient, cmd: LabelCommands) -> Result<()> {
    match cmd {
        LabelCommands::List(args) => {
            let labels = client.list_labels().await?;
            print_output(&labels, args.format)?;
        }
        LabelCommands::Get(args) => {
            let label = client.get_label(&args.label_id).await?;
            print_output(&label, args.format)?;
        }
        LabelCommands::Create(args) => {
            let color = if args.color_bg.is_some() || args.color_text.is_some() {
                Some(LabelColor {
                    text_color: args.color_text.unwrap_or_else(|| "#ffffff".to_string()),
                    background_color: args.color_bg.unwrap_or_else(|| "#4a86e8".to_string()),
                })
            } else {
                None
            };

            let options = CreateLabelOptions {
                label_list_visibility: args.label_list_visibility.map(|v| v.into()),
                message_list_visibility: args.message_list_visibility.map(|v| v.into()),
                color,
            };

            let label = client.create_label(&args.name, options).await?;
            print_output(&label, args.format)?;
        }
        LabelCommands::Update(args) => {
            let color = if args.color_bg.is_some() || args.color_text.is_some() {
                Some(LabelColor {
                    text_color: args.color_text.unwrap_or_else(|| "#ffffff".to_string()),
                    background_color: args.color_bg.unwrap_or_else(|| "#4a86e8".to_string()),
                })
            } else {
                None
            };

            let options = UpdateLabelOptions {
                name: args.name,
                label_list_visibility: args.label_list_visibility.map(|v| v.into()),
                message_list_visibility: args.message_list_visibility.map(|v| v.into()),
                color,
            };

            let label = client.update_label(&args.label_id, options).await?;
            print_output(&label, args.format)?;
        }
        LabelCommands::Delete(args) => {
            client.delete_label(&args.label_id).await?;
            println!("Label {} deleted", args.label_id);
        }
    }
    Ok(())
}

//! Forms-related CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};
use grr_forms::FormsClient;

#[derive(Subcommand, Debug)]
pub enum FormsCommands {
    /// Get a form's structure (items and questions) by ID
    Get(GetArgs),
    /// List a form's submitted responses
    Responses(ResponsesArgs),
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Form ID (the {FORM_ID} in docs.google.com/forms/d/{FORM_ID}/edit)
    pub form_id: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ResponsesArgs {
    /// Form ID (the {FORM_ID} in docs.google.com/forms/d/{FORM_ID}/edit)
    pub form_id: String,

    /// Maximum number of responses
    #[arg(short, long, default_value = "100")]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_forms_cmd(client: &FormsClient, cmd: FormsCommands) -> Result<()> {
    match cmd {
        FormsCommands::Get(args) => {
            let form = client.get_form(&args.form_id).await?;
            print_output(&form, args.format)?;
        }
        FormsCommands::Responses(args) => {
            let responses = client.list_responses(&args.form_id, Some(args.max)).await?;
            print_output(&responses, args.format)?;
        }
    }
    Ok(())
}

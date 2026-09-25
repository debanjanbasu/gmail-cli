use crate::forms::FormsClient;
use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum FormsCommands {
    Get(GetArgs),
    Responses(ResponsesArgs),
    New(NewArgs),
    Update(UpdateArgs),
    Watch(WatchArgs),
    Watches(WatchesArgs),
    WatchDelete(WatchDeleteArgs),
    WatchRenew(WatchRenewArgs),
}

#[derive(Args, Debug)]
pub struct GetArgs {
    pub form_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ResponsesArgs {
    pub form_id: String,

    #[arg(short, long, default_value = "100")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long)]
    pub publish: bool,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    pub form_id: String,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    pub form_id: String,

    #[arg(long)]
    pub topic: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct WatchesArgs {
    pub form_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct WatchDeleteArgs {
    pub form_id: String,
    pub watch_id: String,
}

#[derive(Args, Debug)]
pub struct WatchRenewArgs {
    pub form_id: String,
    pub watch_id: String,

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
        FormsCommands::New(args) => {
            anyhow::ensure!(!args.title.trim().is_empty(), "--title must not be empty");
            let form = client
                .create_form(&args.title, args.description.as_deref(), args.publish)
                .await?;
            print_output(&form, args.format)?;
        }
        FormsCommands::Update(args) => {
            anyhow::ensure!(
                args.title.is_some() || args.description.is_some(),
                "at least one of --title or --description must be supplied"
            );
            let form = client
                .update_form(
                    &args.form_id,
                    args.title.as_deref(),
                    args.description.as_deref(),
                )
                .await?;
            print_output(&form, args.format)?;
        }
        FormsCommands::Watch(args) => {
            anyhow::ensure!(!args.topic.trim().is_empty(), "--topic must not be empty");
            let watch = client.watch_form(&args.form_id, &args.topic).await?;
            print_output(&watch, args.format)?;
        }
        FormsCommands::Watches(args) => {
            let watches = client.list_watches(&args.form_id).await?;
            print_output(&watches, args.format)?;
        }
        FormsCommands::WatchDelete(args) => {
            client.delete_watch(&args.form_id, &args.watch_id).await?;
        }
        FormsCommands::WatchRenew(args) => {
            let watch = client.renew_watch(&args.form_id, &args.watch_id).await?;
            print_output(&watch, args.format)?;
        }
    }
    Ok(())
}

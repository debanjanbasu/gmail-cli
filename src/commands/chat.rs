use crate::chat::ChatClient;
use crate::output::{OutputFormat, print_output};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ChatCommands {
    Spaces(SpacesArgs),
    Space(SpaceArgs),
    SpaceNew(SpaceNewArgs),
    SpaceRename(SpaceRenameArgs),
    SpaceDelete(SpaceDeleteArgs),
    Members(MembersArgs),
    Member(MemberArgs),
    MemberAdd(MemberAddArgs),
    MemberRemove(MemberRemoveArgs),
    Messages(MessagesArgs),
    Send(SendArgs),
    React(ReactArgs),
    Reactions(ReactionsArgs),
    Unreact(UnreactArgs),
}

#[derive(Args, Debug)]
pub struct SpacesArgs {
    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SpaceArgs {
    pub space_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SpaceNewArgs {
    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long = "member", value_name = "USER")]
    pub members: Vec<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SpaceRenameArgs {
    pub space_id: String,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SpaceDeleteArgs {
    pub space_id: String,
}

#[derive(Args, Debug)]
pub struct MembersArgs {
    pub space_id: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct MemberArgs {
    pub space_id: String,
    pub membership_id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct MemberAddArgs {
    pub space_id: String,

    #[arg(long)]
    pub user: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct MemberRemoveArgs {
    pub space_id: String,
    pub membership_id: String,
}

#[derive(Args, Debug)]
pub struct MessagesArgs {
    pub space_id: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SendArgs {
    pub space_id: String,

    #[arg(short, long)]
    pub text: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ReactArgs {
    pub space_id: String,
    pub message_id: String,

    #[arg(long)]
    pub emoji: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ReactionsArgs {
    pub space_id: String,
    pub message_id: String,

    #[arg(short, long, default_value = "25")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UnreactArgs {
    pub space_id: String,
    pub reaction_name: String,
}

pub async fn handle_chat_cmd(client: &ChatClient, cmd: ChatCommands) -> Result<()> {
    match cmd {
        ChatCommands::Spaces(args) => {
            let spaces = client.list_spaces(Some(args.max)).await?;
            print_output(&spaces, args.format)?;
        }
        ChatCommands::Space(args) => {
            let space = client.get_space(&args.space_id).await?;
            print_output(&space, args.format)?;
        }
        ChatCommands::SpaceNew(args) => {
            anyhow::ensure!(!args.name.trim().is_empty(), "--name must not be empty");
            let space = client
                .setup_space(&args.name, args.description.as_deref(), &args.members)
                .await?;
            print_output(&space, args.format)?;
        }
        ChatCommands::SpaceRename(args) => {
            let space = client
                .patch_space(
                    &args.space_id,
                    Some(&args.name),
                    args.description.as_deref(),
                )
                .await?;
            print_output(&space, args.format)?;
        }
        ChatCommands::SpaceDelete(args) => {
            client.delete_space(&args.space_id).await?;
        }
        ChatCommands::Members(args) => {
            let memberships = client
                .list_memberships(&args.space_id, Some(args.max))
                .await?;
            print_output(&memberships, args.format)?;
        }
        ChatCommands::Member(args) => {
            let membership = client
                .get_membership(&args.space_id, &args.membership_id)
                .await?;
            print_output(&membership, args.format)?;
        }
        ChatCommands::MemberAdd(args) => {
            anyhow::ensure!(!args.user.trim().is_empty(), "--user must not be empty");
            let membership = client.create_membership(&args.space_id, &args.user).await?;
            print_output(&membership, args.format)?;
        }
        ChatCommands::MemberRemove(args) => {
            client
                .delete_membership(&args.space_id, &args.membership_id)
                .await?;
        }
        ChatCommands::Messages(args) => {
            let messages = client.list_messages(&args.space_id, Some(args.max)).await?;
            print_output(&messages, args.format)?;
        }
        ChatCommands::Send(args) => {
            anyhow::ensure!(!args.text.trim().is_empty(), "--text must not be empty");
            let message = client.send_message(&args.space_id, &args.text).await?;
            print_output(&message, args.format)?;
        }
        ChatCommands::React(args) => {
            anyhow::ensure!(!args.emoji.trim().is_empty(), "--emoji must not be empty");
            let reaction = client
                .create_reaction(&args.space_id, &args.message_id, &args.emoji)
                .await?;
            print_output(&reaction, args.format)?;
        }
        ChatCommands::Reactions(args) => {
            let reactions = client
                .list_reactions(&args.space_id, &args.message_id, Some(args.max))
                .await?;
            print_output(&reactions, args.format)?;
        }
        ChatCommands::Unreact(args) => {
            client
                .delete_reaction(&args.space_id, &args.reaction_name)
                .await?;
        }
    }
    Ok(())
}

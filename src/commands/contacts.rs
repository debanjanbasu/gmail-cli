//! Contacts (People API) CLI commands

use std::path::PathBuf;

use crate::output::{OutputFormat, print_output};
use crate::people::{
    EmailAddress, ListConnectionsOptions, Name, PeopleClient, Person, PhoneNumber,
};
use anyhow::{Result, bail};
use base64::Engine as _;
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ContactsCommands {
    /// List connections (the user's contacts)
    List(ListArgs),
    /// Search the user's contacts with a prefix query
    Search(SearchArgs),
    /// Get a person by resource name
    Get(GetArgs),
    /// Create a new contact
    Create(CreateArgs),
    /// Update an existing contact
    Update(UpdateArgs),
    /// Delete a contact
    Delete(DeleteArgs),
    #[command(name = "groups", about = "List contact groups")]
    Groups(GroupsArgs),
    #[command(name = "group", about = "Get a contact group")]
    Group(GroupArgs),
    #[command(name = "group-new", about = "Create a contact group")]
    GroupNew(GroupNewArgs),
    #[command(name = "group-rename", about = "Rename a contact group")]
    GroupRename(GroupRenameArgs),
    #[command(name = "group-delete", about = "Delete a contact group")]
    GroupDelete(GroupDeleteArgs),
    #[command(name = "group-add", about = "Add contacts to a group")]
    GroupAdd(GroupMembersArgs),
    #[command(name = "group-remove", about = "Remove contacts from a group")]
    GroupRemove(GroupMembersArgs),
    #[command(name = "get-batch", about = "Get multiple people")]
    GetBatch(GetBatchArgs),
    #[command(name = "others", about = "List other contacts")]
    Others(OthersArgs),
    #[command(name = "adopt", about = "Copy an other contact into my contacts")]
    Adopt(AdoptArgs),
    #[command(name = "photos", about = "List a person's photos")]
    Photos(PhotosArgs),
    #[command(name = "photo-set", about = "Update a contact's photo")]
    PhotoSet(PhotoSetArgs),
    #[command(name = "photo-remove", about = "Delete a contact's photo")]
    PhotoRemove(PhotoRemoveArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter connections by name (plain-text prefix query)
    #[arg(long)]
    pub query_name: Option<String>,

    /// Maximum number of contacts to return
    #[arg(long, default_value_t = 100)]
    pub max: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Prefix query (matches names, emails, phones, organizations)
    pub query: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Resource name (e.g. people/123)
    pub resource_name: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Given (first) name
    #[arg(long)]
    pub given_name: Option<String>,

    /// Family (last) name
    #[arg(long)]
    pub family_name: Option<String>,

    /// Email address
    #[arg(long)]
    pub email: Option<String>,

    /// Phone number
    #[arg(long)]
    pub phone: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Resource name (e.g. people/123)
    pub resource_name: String,

    /// New given (first) name
    #[arg(long)]
    pub given_name: Option<String>,

    /// New family (last) name
    #[arg(long)]
    pub family_name: Option<String>,

    /// New email address (replaces all existing ones)
    #[arg(long)]
    pub email: Option<String>,

    /// New phone number (replaces all existing ones)
    #[arg(long)]
    pub phone: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Resource name (e.g. people/123)
    pub resource_name: String,
}

#[derive(Args, Debug)]
pub struct GroupsArgs {
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GroupArgs {
    #[arg(help = "Contact group ID or resource name")]
    pub id: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GroupNewArgs {
    #[arg(long, help = "Contact group name")]
    pub name: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GroupRenameArgs {
    #[arg(help = "Contact group ID or resource name")]
    pub id: String,

    #[arg(long, help = "New contact group name")]
    pub name: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GroupDeleteArgs {
    #[arg(help = "Contact group ID or resource name")]
    pub id: String,
}

#[derive(Args, Debug)]
pub struct GroupMembersArgs {
    #[arg(help = "Contact group ID or resource name")]
    pub id: String,

    #[arg(
        long,
        value_delimiter = ',',
        required = true,
        help = "Comma-separated people resource names"
    )]
    pub people: Vec<String>,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct GetBatchArgs {
    #[arg(
        long,
        value_delimiter = ',',
        required = true,
        help = "Comma-separated people resource names"
    )]
    pub people: Vec<String>,

    #[arg(
        long,
        default_value = "names,emailAddresses",
        help = "Comma-separated person fields"
    )]
    pub fields: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct OthersArgs {
    #[arg(long, default_value_t = 100, help = "Maximum number of contacts")]
    pub max: usize,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct AdoptArgs {
    #[arg(help = "Other-contact ID or resource name (people/123 is accepted)")]
    pub resource_name: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct PhotosArgs {
    #[arg(help = "Person resource name (e.g. people/123)")]
    pub resource_name: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct PhotoSetArgs {
    #[arg(help = "Person resource name (e.g. people/123)")]
    pub resource_name: String,

    #[arg(long, help = "Path to the photo file")]
    pub path: PathBuf,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct PhotoRemoveArgs {
    #[arg(help = "Person resource name (e.g. people/123)")]
    pub resource_name: String,

    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub async fn handle_contacts_cmd(client: &PeopleClient, cmd: ContactsCommands) -> Result<()> {
    match cmd {
        ContactsCommands::List(args) => {
            let page_size = u32::try_from(args.max.min(1000)).unwrap_or(1000);
            let people = client
                .list_connections(ListConnectionsOptions {
                    page_size,
                    query_name: args.query_name,
                    max_results: args.max,
                })
                .await?;
            print_output(&people, args.format)?;
        }
        ContactsCommands::Search(args) => {
            let people = client.search_contacts(&args.query).await?;
            print_output(&people, args.format)?;
        }
        ContactsCommands::Get(args) => {
            let person = client.get_person(&args.resource_name).await?;
            print_output(&person, args.format)?;
        }
        ContactsCommands::Create(args) => {
            ensure_any_contact_field(
                &args.given_name,
                &args.family_name,
                &args.email,
                &args.phone,
            )?;
            let person = Person {
                names: build_name(&args.given_name, &args.family_name),
                email_addresses: build_email(&args.email),
                phone_numbers: build_phone(&args.phone),
                ..Person::default()
            };
            let created = client.create_contact(person).await?;
            print_output(&created, args.format)?;
        }
        ContactsCommands::Update(args) => {
            ensure_any_contact_field(
                &args.given_name,
                &args.family_name,
                &args.email,
                &args.phone,
            )?;
            let mut person = client.get_person(&args.resource_name).await?;
            let mut fields: Vec<&str> = Vec::new();

            if args.given_name.is_some() || args.family_name.is_some() {
                fields.push("names");
                let names = person.names.get_or_insert_with(Vec::new);
                if names.is_empty() {
                    names.push(Name::default());
                }
                if let Some(name) = names.first_mut() {
                    if args.given_name.is_some() {
                        name.given_name = args.given_name.clone();
                    }
                    if args.family_name.is_some() {
                        name.family_name = args.family_name.clone();
                    }
                }
            }
            if let Some(email) = &args.email {
                fields.push("emailAddresses");
                person.email_addresses = Some(vec![EmailAddress {
                    value: Some(email.clone()),
                    ..EmailAddress::default()
                }]);
            }
            if let Some(phone) = &args.phone {
                fields.push("phoneNumbers");
                person.phone_numbers = Some(vec![PhoneNumber {
                    value: Some(phone.clone()),
                    ..PhoneNumber::default()
                }]);
            }

            let updated = client
                .update_contact(&args.resource_name, person, &fields)
                .await?;
            print_output(&updated, args.format)?;
        }
        ContactsCommands::Delete(args) => {
            client.delete_contact(&args.resource_name).await?;
            println!("Contact {} deleted", args.resource_name);
        }
        ContactsCommands::Groups(args) => {
            let groups = client.list_contact_groups().await?;
            print_output(&groups, args.format)?;
        }
        ContactsCommands::Group(args) => {
            let resource_name = contact_group_resource_name(&args.id)?;
            let group = client.get_contact_group(&resource_name).await?;
            print_output(&group, args.format)?;
        }
        ContactsCommands::GroupNew(args) => {
            let group = client.create_contact_group(&args.name).await?;
            print_output(&group, args.format)?;
        }
        ContactsCommands::GroupRename(args) => {
            let resource_name = contact_group_resource_name(&args.id)?;
            let group = client
                .update_contact_group(&resource_name, &args.name)
                .await?;
            print_output(&group, args.format)?;
        }
        ContactsCommands::GroupDelete(args) => {
            let resource_name = contact_group_resource_name(&args.id)?;
            client.delete_contact_group(&resource_name).await?;
            println!("Contact group {resource_name} deleted");
        }
        ContactsCommands::GroupAdd(args) => {
            let resource_name = contact_group_resource_name(&args.id)?;
            let response = client
                .modify_contact_group_members(&resource_name, &args.people, &[])
                .await?;
            print_output(&response, args.format)?;
        }
        ContactsCommands::GroupRemove(args) => {
            let resource_name = contact_group_resource_name(&args.id)?;
            let response = client
                .modify_contact_group_members(&resource_name, &[], &args.people)
                .await?;
            print_output(&response, args.format)?;
        }
        ContactsCommands::GetBatch(args) => {
            let response = client.get_people(&args.people, &args.fields).await?;
            print_output(&response, args.format)?;
        }
        ContactsCommands::Others(args) => {
            let contacts = client.list_other_contacts(args.max).await?;
            print_output(&contacts, args.format)?;
        }
        ContactsCommands::Adopt(args) => {
            let resource_name = other_contact_resource_name(&args.resource_name)?;
            let person = client
                .copy_other_contact_to_my_contacts_group(&resource_name)
                .await?;
            print_output(&person, args.format)?;
        }
        ContactsCommands::Photos(args) => {
            let photos = client.list_photos(&args.resource_name).await?;
            print_output(&photos, args.format)?;
        }
        ContactsCommands::PhotoSet(args) => {
            let bytes = tokio::fs::read(&args.path).await?;
            let photo_bytes = base64::engine::general_purpose::STANDARD.encode(bytes);
            let response = client
                .update_contact_photo(&args.resource_name, &photo_bytes)
                .await?;
            let photos = response
                .person
                .and_then(|person| person.photos)
                .unwrap_or_default();
            print_output(&photos, args.format)?;
        }
        ContactsCommands::PhotoRemove(args) => {
            let response = client.delete_contact_photo(&args.resource_name).await?;
            let photos = response
                .person
                .and_then(|person| person.photos)
                .unwrap_or_default();
            print_output(&photos, args.format)?;
        }
    }
    Ok(())
}

fn contact_group_resource_name(value: &str) -> Result<String> {
    let value = value.trim();
    if let Some(id) = value.strip_prefix("contactGroups/") {
        if id.is_empty() || id.contains('/') {
            bail!("contact group must have the form contactGroups/<id>");
        }
        return Ok(value.to_owned());
    }
    if value.is_empty() || value.contains('/') {
        bail!("contact group must be an ID or contactGroups/<id>");
    }
    Ok(format!("contactGroups/{value}"))
}

fn other_contact_resource_name(value: &str) -> Result<String> {
    let value = value.trim();
    if let Some(id) = value.strip_prefix("otherContacts/") {
        if id.is_empty() || id.contains('/') {
            bail!("other contact must have the form otherContacts/<id>");
        }
        return Ok(value.to_owned());
    }
    if let Some(id) = value.strip_prefix("people/") {
        if id.is_empty() || id.contains('/') {
            bail!("other contact must have the form otherContacts/<id>");
        }
        return Ok(format!("otherContacts/{id}"));
    }
    if value.is_empty() || value.contains('/') {
        bail!("other contact must be an ID, people/<id>, or otherContacts/<id>");
    }
    Ok(format!("otherContacts/{value}"))
}

/// A contact with no name, email, or phone cannot be created or updated.
fn ensure_any_contact_field(
    given_name: &Option<String>,
    family_name: &Option<String>,
    email: &Option<String>,
    phone: &Option<String>,
) -> Result<()> {
    if given_name.is_none() && family_name.is_none() && email.is_none() && phone.is_none() {
        bail!("at least one of --given-name, --family-name, --email, or --phone is required");
    }
    Ok(())
}

fn build_name(given_name: &Option<String>, family_name: &Option<String>) -> Option<Vec<Name>> {
    if given_name.is_none() && family_name.is_none() {
        return None;
    }
    Some(vec![Name {
        given_name: given_name.clone(),
        family_name: family_name.clone(),
        ..Name::default()
    }])
}

fn build_email(email: &Option<String>) -> Option<Vec<EmailAddress>> {
    email.as_ref().map(|value| {
        vec![EmailAddress {
            value: Some(value.clone()),
            ..EmailAddress::default()
        }]
    })
}

fn build_phone(phone: &Option<String>) -> Option<Vec<PhoneNumber>> {
    phone.as_ref().map(|value| {
        vec![PhoneNumber {
            value: Some(value.clone()),
            ..PhoneNumber::default()
        }]
    })
}

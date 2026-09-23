//! Contacts (People API) CLI commands

use crate::output::{OutputFormat, print_output};
use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use grr_people::{EmailAddress, ListConnectionsOptions, Name, PeopleClient, Person, PhoneNumber};

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
    }
    Ok(())
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

//! gmail-cli - High-performance Gmail CLI

use gmail_core::prelude::*;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "gmail", version, about = "High-performance Gmail CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Gmail
    Auth,
    /// Search emails
    Search {
        query: String,
        #[arg(short, long, default_value = "10")]
        max: usize,
    },
    /// Send email
    Send {
        to: String,
        subject: String,
        body: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    
    let config = GmailConfig::default();
    let auth = AuthConfigBuilder::new()
        .client_id(std::env::var("GMAIL_CLIENT_ID").unwrap_or_default())
        .client_secret(std::env::var("GMAIL_CLIENT_SECRET").unwrap_or_default())
        .build()
        .await?;

    let client = GmailClientBuilder::new(config)
        .auth(auth)
        .build()
        .await?;

    match cli.command {
        Commands::Auth => {
            let _ = client.access_token().await?;
            println!("Authentication successful!");
        }
        Commands::Search { query, max } => {
            let messages = client.search(&query, max).await?;
            println!("{}", serde_json::to_string_pretty(&messages)?);
        }
        Commands::Send { to, subject, body } => {
            let msg = client.send(&to, &subject, &body).await?;
            println!("Sent: {}", msg.id);
        }
    }

    Ok(())
}
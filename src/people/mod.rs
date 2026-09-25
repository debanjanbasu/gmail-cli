//! People (Google Contacts) API client for the grr suite.
//!
//! Typed endpoints (list connections, prefix search, get, create,
//! update, delete) over the shared HTTP/3 engine.
//!
//! Scopes: the shared credential already requests
//! `https://www.googleapis.com/auth/contacts`, covering every endpoint
//! in this module.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_cli::core::auth::AuthConfigBuilder;
//! use grr_cli::people::{ListConnectionsOptions, PeopleClientBuilder};
//!
//! let auth = AuthConfigBuilder::new()
//!     .client_id("...".to_string())
//!     .client_secret(Some("...".to_string()))
//!     .build()
//!     .await?;
//! let client = PeopleClientBuilder::new().auth(auth).build().await?;
//! let contacts = client.list_connections(ListConnectionsOptions::default()).await?;
//! println!("{} contacts", contacts.len());
//! # Ok(())
//! # }
//! ```

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod client;
pub mod models;

pub use client::*;
pub use models::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::people::client::*;
    pub use crate::people::models::*;
}

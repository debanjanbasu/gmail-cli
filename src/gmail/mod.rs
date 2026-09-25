//! Gmail API client for the grr suite.
//!
//! Typed endpoints (messages, labels, drafts, threads, history, settings,
//! watch, streaming upload) over the shared HTTP/3 engine.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_cli::core::auth::AuthConfigBuilder;
//! use grr_cli::gmail::GmailClientBuilder;
//!
//! let auth = AuthConfigBuilder::new()
//!     .client_id("...".to_string())
//!     .client_secret(Some("...".to_string()))
//!     .build()
//!     .await?;
//! let client = GmailClientBuilder::new().auth(auth).build().await?;
//! for msg in client.search("in:inbox", 10).await? {
//!     println!("{}", msg.id);
//! }
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
    pub use crate::gmail::client::*;
    pub use crate::gmail::models::*;
}

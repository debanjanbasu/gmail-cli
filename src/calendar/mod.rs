//! Google Calendar API client for the grr suite.
//!
//! Typed endpoints (calendars, calendarList, events, channels, colors,
//! settings, ACL, freebusy) over the shared HTTP/3 engine.
//!
//! Scopes: the shared credential already requests
//! `https://www.googleapis.com/auth/calendar`, covering every endpoint
//! in this module.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_cli::core::auth::AuthConfigBuilder;
//! use grr_cli::calendar::{CalendarClientBuilder, EventListOptions};
//!
//! let auth = AuthConfigBuilder::new()
//!     .client_id("...".to_string())
//!     .client_secret(Some("...".to_string()))
//!     .build()
//!     .await?;
//! let client = CalendarClientBuilder::new().auth(auth).build().await?;
//! for event in client
//!     .list_events("primary", EventListOptions::default())
//!     .await?
//! {
//!     println!("{:?}", event.summary);
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
    pub use crate::calendar::client::*;
    pub use crate::calendar::models::*;
}

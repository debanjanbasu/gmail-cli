//! grr-calendar — Google Calendar API client for the grr suite.
//!
//! Typed endpoints (calendarList, events CRUD, freebusy) over grr-core's
//! shared HTTP/3 engine.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_core::auth::AuthConfigBuilder;
//! use grr_calendar::{CalendarClientBuilder, EventListOptions};
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
#![allow(clippy::future_not_send)]
#![allow(clippy::large_futures)]

pub mod client;
pub mod models;

pub use client::*;
pub use models::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::client::*;
    pub use crate::models::*;
}

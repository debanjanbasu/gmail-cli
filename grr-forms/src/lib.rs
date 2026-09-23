//! grr-forms — Google Forms API client for the grr suite.
//!
//! Typed endpoints (forms.get, forms.responses.list) over grr-core's
//! shared HTTP/3 engine. Forms has no list endpoint: callers pass a form
//! ID straight from the form URL (docs.google.com/forms/d/{FORM_ID}/edit)
//! or the full `forms/{id}` resource name.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_core::auth::AuthConfigBuilder;
//! use grr_forms::FormsClientBuilder;
//!
//! let auth = AuthConfigBuilder::new()
//!     .client_id("...".to_string())
//!     .client_secret(Some("...".to_string()))
//!     .build()
//!     .await?;
//! let client = FormsClientBuilder::new().auth(auth).build().await?;
//! let form = client.get_form("FORM_ID").await?;
//! println!("{:?}", form.title);
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

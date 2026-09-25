//! Google Forms API client for the grr suite.
//!
//! Typed endpoints (forms.get, forms.responses.list) over the shared
//! HTTP/3 engine. Forms has no list endpoint — callers bring the form ID
//! themselves (see [`FormsClient::get_form`] for the accepted spellings).
//!
//! Scopes: the shared credential already requests
//! `https://www.googleapis.com/auth/forms.body` and
//! `https://www.googleapis.com/auth/forms.responses.readonly`, covering
//! every endpoint in this module.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_cli::core::auth::AuthConfigBuilder;
//! use grr_cli::forms::FormsClientBuilder;
//!
//! let auth = AuthConfigBuilder::new()
//!     .client_id("...".to_string())
//!     .client_secret(Some("...".to_string()))
//!     .build()
//!     .await?;
//! let client = FormsClientBuilder::new().auth(auth).build().await?;
//! let form = client.get_form("FORM_ID").await?;
//! println!("{:?}", form.info.and_then(|info| info.title));
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
    pub use crate::forms::client::*;
    pub use crate::forms::models::*;
}

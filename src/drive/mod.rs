//! Google Drive API client for the grr suite.
//!
//! Typed Drive files, sharing, comments, revisions, streaming media
//! transfer, and about/quota over the shared HTTP/3 engine.
//!
//! Scopes: the shared credential already requests
//! `https://www.googleapis.com/auth/drive`, covering every endpoint
//! in this module.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use grr_cli::core::auth::AuthConfigBuilder;
//! use grr_cli::drive::{DriveClientBuilder, FileListOptions};
//!
//! let auth = AuthConfigBuilder::new()
//!     .client_id("...".to_string())
//!     .client_secret(Some("...".to_string()))
//!     .build()
//!     .await?;
//! let client = DriveClientBuilder::new().auth(auth).build().await?;
//! for file in client
//!     .list_files(FileListOptions {
//!         q: Some("trashed = false".into()),
//!         max_results: 10,
//!     })
//!     .await?
//! {
//!     println!("{:?}", file.name);
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
    pub use crate::drive::client::*;
    pub use crate::drive::models::*;
}

//! grr-core - High-performance Google API client library
//!
//! - HTTP/3 with QUIC (via reqwest unstable; optional cargo feature)
//! - Auto-tuned transport: zero config, adaptive pooling and backoff
//! - OS-keyring token storage with file fallback
//! - Parallel batch operations bounded by API rate limits, not threads
//! - Automatic io_uring detection on Linux

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![allow(clippy::future_not_send)]
#![allow(clippy::large_futures)]

pub mod auth;
pub mod client;
pub mod config;
pub mod config_loader;
pub mod error;
pub mod fs_io;
pub mod models;
pub mod runtime;

pub use auth::{AuthConfigBuilder, DeviceAuthChallenge, GmailAuth, TokenStorage};
pub use client::{GmailClient, GmailClientBuilder};
pub use config::{GmailConfig, OAuthConfig};
pub use config_loader::ConfigLoader;
pub use error::{GmailError, Result};
pub use models::*;
pub use runtime::{RuntimeFeatures, detect_runtime_features};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::auth::{AuthConfigBuilder, DeviceAuthChallenge, GmailAuth, TokenStorage};
    pub use crate::client::{GmailClient, GmailClientBuilder};
    pub use crate::config::{GmailConfig, OAuthConfig};
    pub use crate::config_loader::ConfigLoader;
    pub use crate::error::{GmailError, Result};
    pub use crate::models::*;
    pub use crate::runtime::{RuntimeFeatures, detect_runtime_features};
}

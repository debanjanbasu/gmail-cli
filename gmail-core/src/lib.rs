//! gmail-core - High-performance Gmail API client library
//!
//! Features:
//! - HTTP/3 with QUIC (via reqwest unstable)
//! - Zero-copy streaming with rkyv
//! - Parallel batch operations with semaphore-based concurrency control
//! - Automatic io_uring detection on Linux
//! - zstd/brotli compression with pre-trained dictionaries
//! - napi-rs bindings for Node.js/opencode integration

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
pub mod models;
pub mod runtime;

pub use auth::{AuthConfigBuilder, GmailAuth, TokenStorage};
pub use client::{GmailClient, GmailClientBuilder};
pub use config::{GmailConfig, OAuthConfig, PerformanceConfig};
pub use config_loader::ConfigLoader;
pub use error::{GmailError, Result};
pub use models::*;
pub use runtime::{detect_runtime_features, RuntimeFeatures};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::auth::{AuthConfigBuilder, GmailAuth, TokenStorage};
    pub use crate::client::{GmailClient, GmailClientBuilder};
    pub use crate::config::{GmailConfig, OAuthConfig, PerformanceConfig};
pub use crate::config_loader::ConfigLoader;
    pub use crate::error::{GmailError, Result};
    pub use crate::models::*;
    pub use crate::runtime::{detect_runtime_features, RuntimeFeatures};
}
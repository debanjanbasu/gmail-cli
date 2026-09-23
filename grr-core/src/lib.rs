//! grr-core — the shared base for every grr service client.
//!
//! Service-agnostic: OAuth (keyring-backed), zero-config HTTP transport
//! with HTTP/3 and rate-limit-aware retry, config, and runtime probes.
//! Service crates (grr-gmail, grr-calendar, ...) build typed clients on
//! [`HttpCore`].
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
pub mod config;
pub mod config_loader;
pub mod error;
pub mod fs_io;
pub mod http;
pub mod runtime;

pub use auth::{AuthConfigBuilder, DeviceAuthChallenge, GoogleAuth, TokenStorage};
pub use config::{GrrConfig, OAuthConfig};
pub use config_loader::ConfigLoader;
pub use error::{GrrError, Result};
pub use http::{
    HttpCore, TransportInfo, TransportMode, apply_transport_version, resolve_transport_mode,
};
pub use runtime::{RuntimeFeatures, detect_runtime_features};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::auth::{AuthConfigBuilder, DeviceAuthChallenge, GoogleAuth, TokenStorage};
    pub use crate::config::{GrrConfig, OAuthConfig};
    pub use crate::config_loader::ConfigLoader;
    pub use crate::error::{GrrError, Result};
    pub use crate::http::{
        HttpCore, TransportInfo, TransportMode, apply_transport_version, resolve_transport_mode,
    };
    pub use crate::runtime::{RuntimeFeatures, detect_runtime_features};
}

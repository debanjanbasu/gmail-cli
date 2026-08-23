//! Configuration loading with TOML file and environment variable support

use crate::config::GmailConfig;
use crate::error::{GmailError, Result};
use dirs;
use figment::{
    Figment, Provider,
    providers::Format,
    providers::{Env, Toml},
};
use std::path::PathBuf;
use tracing::info;

/// Configuration loader that merges TOML file with environment variables
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from TOML file with environment variable overrides
    ///
    /// Priority order (highest to lowest):
    /// 1. Environment variables (GMAIL_*)
    /// 2. TOML config file
    /// 3. Default values
    pub async fn load() -> Result<GmailConfig> {
        let config_path = Self::config_path()?;

        info!("Loading config from: {:?}", config_path);

        let figment = Figment::new()
            .merge(Toml::file(&config_path))
            .merge(Self::env_provider());

        let config: GmailConfig = figment
            .extract()
            .map_err(|e| GmailError::Config(e.to_string()))?;

        info!(
            "Loaded config: client_id={}, client_secret={}",
            config.oauth.client_id,
            if config.oauth.client_secret.is_empty() {
                "<empty>"
            } else {
                "<set>"
            }
        );
        Ok(config)
    }

    /// Create an Env provider that maps GMAIL_* env vars to kebab-case keys
    /// matching the serde rename_all = "kebab-case" setting
    fn env_provider() -> impl Provider {
        Env::raw().filter_map(|key| {
            let key = key.as_str();
            if let Some(stripped) = key.strip_prefix("GMAIL_") {
                // Convert to kebab-case: OAUTH__CLIENT_ID -> oauth.client-id
                // First replace __ with . for nesting, then _ with - for field names
                let key = stripped.replace("__", ".");
                let key = key.replace('_', "-");
                let key = key.to_ascii_lowercase();
                Some(key.into())
            } else {
                None
            }
        })
    }

    /// Determine the config file path
    ///
    /// Checks in order:
    /// 1. GMAIL_CONFIG_PATH environment variable
    /// 2. ~/.gmail-opencode/config.toml (cross-platform, matches TypeScript version)
    fn config_path() -> Result<PathBuf> {
        if let Ok(path) = std::env::var("GMAIL_CONFIG_PATH") {
            return Ok(PathBuf::from(path));
        }

        let home = dirs::home_dir()
            .ok_or_else(|| GmailError::Config("Could not find home directory".into()))?;

        Ok(home.join(".gmail-opencode").join("config.toml"))
    }
}

//! Configuration loading with TOML file and environment variable support

use crate::config::GmailConfig;
use crate::error::{GmailError, Result};
use dirs;
use figment::{Figment, Provider, providers::{Toml, Env}, providers::Format};
use std::path::PathBuf;

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
        
        let figment = Figment::new()
            .merge(Toml::file(&config_path))
            .merge(Self::env_provider());
            
        let config: GmailConfig = figment.extract()
            .map_err(|e| GmailError::Config(e.to_string()))?;
        Ok(config)
    }
    
    /// Create an Env provider that maps GMAIL_* env vars to kebab-case keys
    /// matching the serde rename_all = "kebab-case" setting
    fn env_provider() -> impl Provider {
        Env::raw()
            .filter_map(|key| {
                let key = key.as_str();
                if key.starts_with("GMAIL_") {
                    let key = &key[6..]; // Remove "GMAIL_"
                    // Convert to kebab-case: OAUTH__CLIENT_ID -> oauth.client-id
                    // First replace __ with . for nesting, then _ with - for field names
                    let key = key.replace("__", ".");
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
    /// 2. ~/.config/gmail-opencode/config.toml
    fn config_path() -> Result<PathBuf> {
        if let Ok(path) = std::env::var("GMAIL_CONFIG_PATH") {
            return Ok(PathBuf::from(path));
        }
        
        let config_dir = dirs::config_dir()
            .ok_or_else(|| GmailError::Config("Could not find config directory".into()))?
            .join("gmail-opencode");
            
        Ok(config_dir.join("config.toml"))
    }
}
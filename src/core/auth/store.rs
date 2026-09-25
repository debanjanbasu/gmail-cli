//! Token persistence: OS keyring by default (Windows Credential Manager,
//! macOS Keychain, Linux Secret Service via D-Bus), with a plain platform
//! file fallback for headless systems without a keyring daemon.
//!
//! Fully automatic: no configuration, no env knobs. If the OS keyring is
//! unavailable the store degrades to `<cache-dir>/grr/token.json`; a token
//! found in the fallback file is imported into the keyring and the file
//! removed only after the keyring write succeeds, so machines regain keyring
//! storage without any user action.

use std::path::PathBuf;

use tracing::{debug, info, warn};

use crate::core::error::{GrrError, Result};

use super::TokenStorage;

const KEYRING_SERVICE: &str = "grr";
const KEYRING_ACCOUNT: &str = "google-oauth";

/// Where tokens live.
pub(crate) enum TokenStore {
    /// Keyring entries are cheap handles (service + account strings) and
    /// are NOT Clone in keyring v4, so we re-create the handle on demand
    /// inside the blocking task instead of holding a borrowed Entry.
    Keyring,
    File(PathBuf),
    Memory,
}

impl TokenStore {
    /// Auto-detect: keyring when the OS provides one, file otherwise.
    pub async fn auto() -> Result<Self> {
        let result = tokio::task::spawn_blocking(|| {
            keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map(|_| ())
        })
        .await
        .map_err(|e| GrrError::Internal(e.to_string()))?;

        match result {
            Ok(()) => Ok(Self::Keyring),
            Err(e) => {
                debug!("keyring unavailable ({e}); using file token store");
                Ok(Self::File(Self::fallback_path()))
            }
        }
    }

    /// Explicit file store (test injection).
    pub fn file(path: PathBuf) -> Self {
        Self::File(path)
    }

    fn fallback_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("grr")
            .join("token.json")
    }

    /// Backend name for logs and `auth login` output.
    pub fn backend(&self) -> &'static str {
        match self {
            Self::Keyring => "os-keyring",
            Self::File(_) => "file",
            Self::Memory => "memory",
        }
    }

    /// Load the stored token, if any.
    pub async fn load(&self) -> Result<Option<TokenStorage>> {
        match self {
            Self::Keyring => {
                let res = tokio::task::spawn_blocking(|| {
                    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                        .and_then(|entry| entry.get_password())
                })
                .await
                .map_err(|e| GrrError::Internal(e.to_string()))?;
                match res {
                    Ok(secret) if !secret.trim().is_empty() => {
                        let storage: TokenStorage = serde_json::from_str(&secret)?;
                        info!("Loaded token from OS keyring");
                        Ok(Some(storage))
                    }
                    Ok(_) => Ok(None),
                    Err(keyring::Error::NoEntry) => {
                        if let Some(storage) = Self::read_fallback_file().await? {
                            info!("Importing token from fallback file into OS keyring");
                            if Self::write_keyring(serde_json::to_string(&storage)?).await? {
                                Self::remove_fallback_file().await;
                            }
                            return Ok(Some(storage));
                        }
                        Ok(None)
                    }
                    Err(e) => {
                        warn!("keyring read failed ({e}); falling back to file token");
                        Self::read_fallback_file().await
                    }
                }
            }
            Self::File(path) => match tokio::fs::read_to_string(path).await {
                Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            },
            Self::Memory => Ok(None),
        }
    }

    /// Persist the token. Keyring failures degrade to the fallback file so
    /// login never fails because the keyring daemon hiccuped.
    pub async fn save(&self, storage: &TokenStorage) -> Result<()> {
        let secret = serde_json::to_string(storage)?;
        match self {
            Self::Keyring => {
                if !Self::write_keyring(secret.clone()).await? {
                    warn!("keyring write failed; writing token to fallback file");
                    Self::write_fallback_file(&secret).await?;
                }
            }
            Self::File(path) => {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(path, &secret).await?;
            }
            Self::Memory => {}
        }
        Ok(())
    }

    /// Remove the stored credential everywhere.
    pub async fn delete(&self) -> Result<()> {
        match self {
            Self::Keyring => {
                let res = tokio::task::spawn_blocking(|| {
                    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                        .and_then(|entry| entry.delete_credential())
                })
                .await
                .map_err(|e| GrrError::Internal(e.to_string()))?;
                match res {
                    Ok(()) | Err(keyring::Error::NoEntry) => {}
                    Err(e) => warn!("keyring delete failed: {e}"),
                }
                Self::remove_fallback_file().await;
            }
            Self::File(path) => match tokio::fs::remove_file(path).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            },
            Self::Memory => {}
        }
        Ok(())
    }

    async fn write_keyring(secret: String) -> Result<bool> {
        let result = tokio::task::spawn_blocking(move || {
            keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                .and_then(|entry| entry.set_password(&secret))
        })
        .await
        .map_err(|e| GrrError::Internal(e.to_string()))?;

        match result {
            Ok(()) => Ok(true),
            Err(e) => {
                warn!("keyring write failed: {e}");
                Ok(false)
            }
        }
    }

    async fn read_fallback_file() -> Result<Option<TokenStorage>> {
        let path = Self::fallback_path();
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn write_fallback_file(secret: &str) -> Result<()> {
        let path = Self::fallback_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, secret).await?;
        Ok(())
    }

    async fn remove_fallback_file() {
        let path = Self::fallback_path();
        match tokio::fs::remove_file(&path).await {
            Ok(()) => debug!("Removed fallback token file {:?}", path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!("Could not remove fallback token file {:?}: {e}", path),
        }
    }
}

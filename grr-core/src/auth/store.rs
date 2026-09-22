//! Token persistence: OS keyring by default (Windows Credential Manager,
//! macOS Keychain, Linux Secret Service via D-Bus), with a plain platform
//! file fallback for headless systems without a keyring daemon.
//!
//! Fully automatic: no configuration, no env knobs. If the OS keyring is
//! unavailable the store degrades to `<cache-dir>/grr/token.json`; a token
//! found in the fallback file is imported into the keyring and the file
//! removed on the next successful save, so machines regain keyring storage
//! without any user action.

use std::path::PathBuf;

use tracing::{debug, info, warn};

use crate::error::{GmailError, Result};

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
}

impl TokenStore {
    /// Auto-detect: keyring when the OS provides one, file otherwise.
    pub fn auto() -> Self {
        match keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT) {
            Ok(_) => Self::Keyring,
            Err(e) => {
                debug!("keyring unavailable ({e}); using file token store");
                Self::File(Self::fallback_path())
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
        }
    }

    /// Load the stored token, if any.
    pub async fn load(&self) -> Result<Option<TokenStorage>> {
        match self {
            Self::Keyring => {
                let res = tokio::task::spawn_blocking(move || {
                    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                        .and_then(|entry| entry.get_password())
                })
                .await
                .map_err(|e| GmailError::Internal(e.to_string()))?;
                match res {
                    Ok(secret) if !secret.trim().is_empty() => {
                        let storage: TokenStorage = serde_json::from_str(&secret)?;
                        info!("Loaded token from OS keyring");
                        Ok(Some(storage))
                    }
                    Ok(_) => Ok(None),
                    Err(keyring::Error::NoEntry) => {
                        // Bootstrap: a token left in the fallback file (e.g.
                        // by a previous headless run or a manual copy) moves
                        // into the keyring on first sight.
                        if let Some(storage) = Self::read_fallback_file().await? {
                            info!("Importing token from fallback file into OS keyring");
                            self.save(&storage).await?;
                            Self::remove_fallback_file().await;
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
            Self::File(path) => {
                if !path.exists() {
                    return Ok(None);
                }
                let content = tokio::fs::read_to_string(path).await?;
                let storage: TokenStorage = serde_json::from_str(&content)?;
                Ok(Some(storage))
            }
        }
    }

    /// Persist the token. Keyring failures degrade to the fallback file so
    /// login never fails because the keyring daemon hiccuped.
    pub async fn save(&self, storage: &TokenStorage) -> Result<()> {
        let secret = serde_json::to_string(storage)?;
        match self {
            Self::Keyring => {
                let secret_for_keyring = secret.clone();
                let res = tokio::task::spawn_blocking(move || {
                    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                        .and_then(|entry| entry.set_password(&secret_for_keyring))
                })
                .await
                .map_err(|e| GmailError::Internal(e.to_string()))?;
                if let Err(e) = res {
                    warn!("keyring write failed ({e}); writing token to fallback file");
                    Self::write_fallback_file(&secret).await?;
                }
            }
            Self::File(path) => {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(path, &secret).await?;
            }
        }
        Ok(())
    }

    /// Remove the stored credential everywhere.
    pub async fn delete(&self) -> Result<()> {
        match self {
            Self::Keyring => {
                let res = tokio::task::spawn_blocking(move || {
                    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                        .and_then(|entry| entry.delete_credential())
                })
                .await
                .map_err(|e| GmailError::Internal(e.to_string()))?;
                match res {
                    Ok(()) | Err(keyring::Error::NoEntry) => {}
                    Err(e) => warn!("keyring delete failed: {e}"),
                }
            }
            Self::File(path) => {
                if path.exists() {
                    tokio::fs::remove_file(path).await?;
                }
            }
        }
        // Belt and braces: revoke must clear any fallback copy too.
        Self::remove_fallback_file().await;
        Ok(())
    }

    async fn read_fallback_file() -> Result<Option<TokenStorage>> {
        let path = Self::fallback_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(Some(serde_json::from_str(&content)?))
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
        if tokio::fs::remove_file(&path).await.is_ok() {
            debug!("Removed fallback token file {:?}", path);
        }
    }
}

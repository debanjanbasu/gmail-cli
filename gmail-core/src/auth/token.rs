//! Token storage and persistence

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::Result;

/// Token storage with automatic refresh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStorage {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp
    pub token_type: String,
    pub scope: String,
}

impl TokenStorage {
    /// Check if token is expired or about to expire (within 60 seconds)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now + 60 >= self.expires_at
    }
    
    /// Get remaining lifetime in seconds
    pub fn remaining_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at.saturating_sub(now)
    }
}

impl super::GmailAuth {
    /// Save token to disk
    pub(crate) async fn save_token(&self, storage: &TokenStorage) -> Result<()> {
        if let Some(parent) = self.token_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(storage)?;
        tokio::fs::write(&self.token_path, content).await?;
        debug!("Token saved to {:?}", self.token_path);
        Ok(())
    }
}
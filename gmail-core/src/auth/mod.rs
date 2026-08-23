//! OAuth2 authentication with PKCE support using direct HTTP calls

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use reqwest::Client as HttpClient;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::OAuthConfig;
use crate::error::{GmailError, Result};

mod oauth;
mod server;
mod token;

pub use token::TokenStorage;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// OAuth2 client with PKCE support
pub struct GmailAuth {
    config: OAuthConfig,
    http_client: HttpClient,
    token_storage: Arc<RwLock<Option<TokenStorage>>>,
    token_path: PathBuf,
    token_endpoint: String,
}

impl GmailAuth {
    /// Create new GmailAuth from config
    pub async fn new(config: OAuthConfig) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| GmailError::Config(format!("Failed to create HTTP client: {}", e)))?;

        // Determine token storage path
        let token_path = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gmail-opencode")
            .join("token.json");

        // Load existing token if available
        let token_storage = if token_path.exists() {
            let content = tokio::fs::read_to_string(&token_path).await?;
            let storage: TokenStorage = serde_json::from_str(&content)?;
            info!("Loaded existing token from {:?}", token_path);
            Some(storage)
        } else {
            None
        };

        Ok(Self {
            config,
            http_client,
            token_storage: Arc::new(RwLock::new(token_storage)),
            token_path,
            token_endpoint: GOOGLE_TOKEN_URL.to_string(),
        })
    }

    /// Construct an auth handle pre-loaded with an in-memory token.
    ///
    /// Test hook (`#[doc(hidden)]`): skips the OAuth flow entirely; as long
    /// as the injected token stays valid nothing touches the on-disk cache.
    #[doc(hidden)]
    pub async fn with_token(config: OAuthConfig, storage: TokenStorage) -> Result<Self> {
        let this = Self::new(config).await?;
        *this.token_storage.write().await = Some(storage);
        Ok(this)
    }

    /// Override the OAuth2 token endpoint URL (test injection hook).
    #[doc(hidden)]
    pub fn with_token_endpoint(mut self, url: impl Into<String>) -> Self {
        self.token_endpoint = url.into();
        self
    }

    /// Override the on-disk token persistence path (test injection hook).
    ///
    /// Without this, any code path that calls [`Self::save_token`] or
    /// [`Self::revoke`] writes/deletes the real user credential at the
    /// platform cache dir; tests must always redirect it into a tempdir.
    #[doc(hidden)]
    pub fn with_token_path(mut self, path: PathBuf) -> Self {
        self.token_path = path;
        self
    }

    /// Get valid access token, refreshing if necessary
    pub async fn get_access_token(&self) -> Result<String> {
        let mut storage_guard = self.token_storage.write().await;

        if let Some(storage) = storage_guard.as_ref() {
            if !storage.is_expired() {
                debug!(
                    "Using cached access token ({}s remaining)",
                    storage.remaining_secs()
                );
                return Ok(storage.access_token.clone());
            }

            // A stored credential that cannot produce a fresh token must never
            // trigger the implicit browser flow: `run_oauth_flow` binds port
            // 3434 mid-API-call and wedges callers for the callback timeout.
            let Some(refresh_token) = storage.refresh_token.clone() else {
                return Err(GmailError::Auth(
                    anyhow!(
                        "stored token is expired and has no refresh token; rerun `gmail auth login`"
                    )
                    .into(),
                ));
            };

            info!("Refreshing expired access token");
            match self.refresh_token(&refresh_token).await {
                Ok(new_storage) => {
                    *storage_guard = Some(new_storage.clone());
                    self.save_token(&new_storage).await?;
                    return Ok(new_storage.access_token);
                }
                Err(e) => {
                    warn!("Token refresh failed: {}", e);
                    // Keep the expired storage in place: clearing it would
                    // route subsequent callers into the implicit OAuth flow.
                    return Err(GmailError::Auth(
                        anyhow!("{e}; rerun `gmail auth login`").into(),
                    ));
                }
            }
        }

        // Fresh install / explicit login path: no stored token at all.
        info!("No valid token, starting OAuth flow");
        let storage = self.run_oauth_flow().await?;
        *storage_guard = Some(storage.clone());
        self.save_token(&storage).await?;
        Ok(storage.access_token)
    }

    /// Refresh access token using refresh token
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenStorage> {
        let response = self
            .http_client
            .post(self.token_endpoint.as_str())
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(GmailError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(GmailError::Http)?;

        if !status.is_success() {
            // Google reports rejection reasons (e.g. invalid_grant) in the
            // response body; surface them instead of a generic parse failure.
            let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                GmailError::Auth(
                    anyhow!("token endpoint returned {}: unparseable body ({e})", status).into(),
                )
            })?;
            let reason = body["error"]
                .as_str()
                .or_else(|| body["error_description"].as_str())
                .unwrap_or("unknown error");
            let description = body["error_description"].as_str().unwrap_or("");
            let detail = if description.is_empty() || description == reason {
                reason.to_string()
            } else {
                format!("{reason}: {description}")
            };
            return Err(GmailError::Auth(
                anyhow!("token endpoint returned {}: {}", status, detail).into(),
            ));
        }

        let token_data: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            GmailError::Auth(
                anyhow!("token endpoint returned unparseable success body ({e})").into(),
            )
        })?;

        let access_token = token_data["access_token"]
            .as_str()
            .ok_or_else(|| GmailError::Auth(anyhow!("No access token in response").into()))?
            .to_string();

        let expires_in = token_data["expires_in"].as_u64().unwrap_or(3600);

        // Google only sometimes rotates the refresh token; when the
        // response omits one, keep the stored value instead of clobbering
        // it with None (which would brick all future refreshes).
        let refresh_token = token_data["refresh_token"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| refresh_token.to_string());

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            + expires_in;

        Ok(TokenStorage {
            access_token,
            refresh_token: Some(refresh_token),
            expires_at,
            token_type: "Bearer".to_string(),
            scope: self.config.scopes.join(" "),
        })
    }

    /// Revoke current token
    pub async fn revoke(&self) -> Result<()> {
        let mut storage_guard = self.token_storage.write().await;
        *storage_guard = None;
        if self.token_path.exists() {
            tokio::fs::remove_file(&self.token_path).await?;
        }
        info!("Token revoked and removed");
        Ok(())
    }

    /// Force re-authentication by clearing token and starting OAuth flow
    pub async fn force_refresh(&self) -> Result<()> {
        self.revoke().await?;
        self.get_access_token().await?;
        Ok(())
    }
}

pub struct AuthConfigBuilder {
    config: OAuthConfig,
}

impl AuthConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: OAuthConfig::default(),
        }
    }

    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.config.client_id = id.into();
        self
    }

    pub fn client_secret(mut self, secret: impl Into<String>) -> Self {
        self.config.client_secret = secret.into();
        self
    }

    pub fn redirect_uri(mut self, uri: impl Into<String>) -> Self {
        self.config.redirect_uri = uri.into();
        self
    }

    pub fn scopes(mut self, scopes: Vec<String>) -> Self {
        self.config.scopes = scopes;
        self
    }

    pub fn use_pkce(mut self, use_pkce: bool) -> Self {
        self.config.use_pkce = use_pkce;
        self
    }

    pub async fn build(self) -> Result<GmailAuth> {
        GmailAuth::new(self.config).await
    }
}

impl Default for AuthConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

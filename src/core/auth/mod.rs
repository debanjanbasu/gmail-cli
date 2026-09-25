//! OAuth2 authentication: PKCE loopback flow (RFC 8252) with an RFC 8628
//! device-flow fallback, tokens in the OS keyring.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::Client as HttpClient;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::core::config::OAuthConfig;
use crate::core::error::{GrrError, Result};

mod device;
mod oauth;
mod server;
mod store;
mod token;

pub use device::DeviceAuthChallenge;
pub use token::TokenStorage;

use store::TokenStore;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Loopback redirect for the native-app flow (RFC 8252 §7.3).
pub(crate) const REDIRECT_URI: &str = "http://localhost:3434/oauth/callback";

/// Union of every service's scopes (Gmail, Calendar, Drive, People, Chat,
/// Forms). Least-privilege selection is deliberately not implemented:
/// one credential, everything works, nothing to configure.
pub(crate) const SCOPES: &[&str] = &[
    // Gmail
    "https://www.googleapis.com/auth/gmail.readonly",
    "https://www.googleapis.com/auth/gmail.compose",
    "https://www.googleapis.com/auth/gmail.modify",
    "https://www.googleapis.com/auth/gmail.labels",
    "https://mail.google.com/",
    // Calendar
    "https://www.googleapis.com/auth/calendar",
    // Drive
    "https://www.googleapis.com/auth/drive",
    // People (contacts)
    "https://www.googleapis.com/auth/contacts",
    "https://www.googleapis.com/auth/contacts.other.readonly",
    // Chat
    "https://www.googleapis.com/auth/chat.messages",
    "https://www.googleapis.com/auth/chat.spaces",
    "https://www.googleapis.com/auth/chat.delete",
    "https://www.googleapis.com/auth/chat.memberships",
    "https://www.googleapis.com/auth/chat.messages.reactions",
    // Forms
    "https://www.googleapis.com/auth/forms.body",
    "https://www.googleapis.com/auth/forms.responses.readonly",
];

pub(crate) fn scopes_joined() -> String {
    SCOPES.join(" ")
}

/// OAuth2 client with PKCE support
pub struct GoogleAuth {
    config: OAuthConfig,
    http_client: HttpClient,
    token_storage: Arc<RwLock<Option<TokenStorage>>>,
    store: TokenStore,
    token_endpoint: String,
}

impl GoogleAuth {
    /// Create new GoogleAuth from config
    pub async fn new(config: OAuthConfig) -> Result<Self> {
        Self::with_store(config, TokenStore::auto().await?).await
    }

    async fn with_store(config: OAuthConfig, store: TokenStore) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| GrrError::Config(format!("Failed to create HTTP client: {}", e)))?;
        let token_storage = store.load().await?;

        Ok(Self {
            config,
            http_client,
            token_storage: Arc::new(RwLock::new(token_storage)),
            store,
            token_endpoint: GOOGLE_TOKEN_URL.to_string(),
        })
    }

    /// Token storage backend in use (for logs and `auth login` output).
    pub fn token_backend(&self) -> &'static str {
        self.store.backend()
    }

    /// Construct an auth handle pre-loaded with an in-memory token.
    ///
    /// Test hook (`#[doc(hidden)]`): skips OAuth and persistent storage
    /// entirely until a test explicitly installs a token path.
    #[doc(hidden)]
    pub async fn with_token(config: OAuthConfig, storage: TokenStorage) -> Result<Self> {
        let this = Self::with_store(config, TokenStore::Memory).await?;
        *this.token_storage.write().await = Some(storage);
        Ok(this)
    }

    /// Override the OAuth2 token endpoint URL (test injection hook).
    #[doc(hidden)]
    pub fn with_token_endpoint(mut self, url: impl Into<String>) -> Self {
        self.token_endpoint = url.into();
        self
    }

    /// Redirect all token persistence to an explicit file (test injection
    /// hook). Production code never touches the real credential store
    /// because tests always pass a tempdir path here.
    #[doc(hidden)]
    pub fn with_token_path(mut self, path: PathBuf) -> Self {
        self.store = TokenStore::file(path);
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
                return Err(GrrError::Auth(
                    anyhow!(
                        "stored token is expired and has no refresh token; rerun `grr auth login`"
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
                    return Err(GrrError::Auth(
                        anyhow!("{e}; rerun `grr auth login`").into(),
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

    /// Refresh access token using refresh token.
    /// The secret is sent only when configured (Google mandates it even
    /// for Desktop clients; PKCE-only providers omit it entirely).
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenStorage> {
        let mut form = vec![
            ("client_id", self.config.client_id.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        if let Some(secret) = self
            .config
            .client_secret
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            form.push(("client_secret", secret));
        }
        let response = self
            .http_client
            .post(self.token_endpoint.as_str())
            .form(&form)
            .send()
            .await
            .map_err(GrrError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(GrrError::Http)?;

        if !status.is_success() {
            // Google reports rejection reasons (e.g. invalid_grant) in the
            // response body; surface them instead of a generic parse failure.
            let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                GrrError::Auth(
                    anyhow!("token endpoint returned {}: unparseable body ({e})", status).into(),
                )
            })?;
            return Err(GrrError::Auth(
                anyhow!(
                    "token endpoint returned {}: {}",
                    status,
                    self::device::error_detail(&body)
                )
                .into(),
            ));
        }

        let token_data: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            GrrError::Auth(anyhow!("token endpoint returned unparseable success body ({e})").into())
        })?;

        let mut storage = self::device::token_storage_from_response(&token_data, SCOPES)?;

        // Google only sometimes rotates the refresh token; when the
        // response omits one, keep the stored value instead of clobbering
        // it with None (which would brick all future refreshes).
        if storage.refresh_token.is_none() {
            storage.refresh_token = Some(refresh_token.to_string());
        }

        Ok(storage)
    }

    /// Revoke current token
    pub async fn revoke(&self) -> Result<()> {
        let mut storage_guard = self.token_storage.write().await;
        *storage_guard = None;
        self.store.delete().await?;
        info!("Token revoked and removed");
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

    pub fn client_secret(mut self, secret: Option<String>) -> Self {
        // Empty strings behave as absent: the secret is omitted everywhere.
        self.config.client_secret = secret.filter(|s| !s.is_empty());
        self
    }

    pub async fn build(self) -> Result<GoogleAuth> {
        GoogleAuth::new(self.config).await
    }
}

impl Default for AuthConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

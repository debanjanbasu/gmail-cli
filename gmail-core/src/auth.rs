//! OAuth2 authentication with PKCE support using direct HTTP calls

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use base64::Engine;
use rand::RngCore;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use urlencoding;

use crate::config::OAuthConfig;
use crate::error::{GmailError, Result};

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

/// OAuth2 client with PKCE support
pub struct GmailAuth {
    config: OAuthConfig,
    http_client: HttpClient,
    token_storage: Arc<RwLock<Option<TokenStorage>>>,
    token_path: PathBuf,
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
        })
    }

    /// Get valid access token, refreshing if necessary
    pub async fn get_access_token(&self) -> Result<String> {
        let mut storage_guard = self.token_storage.write().await;
        
        if let Some(storage) = storage_guard.as_ref() {
            if !storage.is_expired() {
                debug!("Using cached access token ({}s remaining)", storage.remaining_secs());
                return Ok(storage.access_token.clone());
            }
            
            // Try to refresh
            if let Some(refresh_token) = &storage.refresh_token {
                info!("Refreshing expired access token");
                match self.refresh_token(refresh_token).await {
                    Ok(new_storage) => {
                        *storage_guard = Some(new_storage.clone());
                        self.save_token(&new_storage).await?;
                        return Ok(new_storage.access_token);
                    }
                    Err(e) => {
                        warn!("Token refresh failed: {}", e);
                        // Clear invalid token
                        *storage_guard = None;
                    }
                }
            }
        }

        // Need to run full OAuth flow
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
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| GmailError::Http(e))?;

        let token_data: serde_json::Value = response.json().await.map_err(|e| GmailError::Http(e))?;
        
        let access_token = token_data["access_token"]
            .as_str()
            .ok_or_else(|| GmailError::Auth(anyhow!("No access token in response").into()))?
            .to_string();
        
        let expires_in = token_data["expires_in"]
            .as_u64()
            .unwrap_or(3600);
        
        let refresh_token = token_data["refresh_token"]
            .as_str()
            .map(|s| s.to_string());

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + expires_in;

        Ok(TokenStorage {
            access_token,
            refresh_token,
            expires_at,
            token_type: "Bearer".to_string(),
            scope: self.config.scopes.join(" "),
        })
    }

    /// Run full OAuth2 flow with PKCE
    async fn run_oauth_flow(&self) -> Result<TokenStorage> {
        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = if self.config.use_pkce {
            let mut verifier_bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut verifier_bytes);
            let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&verifier_bytes);
            let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(Sha256::digest(verifier.as_bytes()).as_slice());
            (Some(challenge), Some(verifier))
        } else {
            (None, None)
        };

        // Generate authorization URL
        let mut auth_url = "https://accounts.google.com/o/oauth2/v2/auth".to_string();
        auth_url.push_str("?response_type=code");
        auth_url.push_str(&format!("&client_id={}", urlencoding::encode(&self.config.client_id)));
        auth_url.push_str(&format!("&redirect_uri={}", urlencoding::encode(&self.config.redirect_uri)));
        auth_url.push_str(&format!("&scope={}", urlencoding::encode(&self.config.scopes.join(" "))));
        auth_url.push_str("&access_type=offline");
        auth_url.push_str("&prompt=consent");
        
        if let Some(challenge) = &pkce_challenge {
            auth_url.push_str(&format!("&code_challenge={}", challenge));
            auth_url.push_str("&code_challenge_method=S256");
        }

        info!("Opening browser for authentication: {}", auth_url);
        
        // Open browser
        if let Err(e) = open::that(auth_url.as_str()) {
            warn!("Failed to open browser: {}. Please manually open: {}", e, auth_url);
        }

        // Start local server to receive callback
        let (code, _state) = self.start_callback_server().await?;

        // Exchange code for tokens
        let mut form = vec![
            ("code", code),
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("redirect_uri", self.config.redirect_uri.clone()),
            ("grant_type", "authorization_code".to_string()),
        ];
        
        if let Some(verifier) = pkce_verifier {
            form.push(("code_verifier", verifier));
        }

        let response = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&form)
            .send()
            .await
            .map_err(|e| GmailError::Http(e))?;

        let token_data: serde_json::Value = response.json().await.map_err(|e| GmailError::Http(e))?;
        
        let access_token = token_data["access_token"]
            .as_str()
            .ok_or_else(|| GmailError::Auth(anyhow!("No access token in response").into()))?
            .to_string();
        
        let expires_in = token_data["expires_in"]
            .as_u64()
            .unwrap_or(3600);
        
        let refresh_token = token_data["refresh_token"]
            .as_str()
            .map(|s| s.to_string());

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + expires_in;

        Ok(TokenStorage {
            access_token,
            refresh_token,
            expires_at,
            token_type: "Bearer".to_string(),
            scope: self.config.scopes.join(" "),
        })
    }

    /// Start local HTTP server to receive OAuth callback
    async fn start_callback_server(&self) -> Result<(String, String)> {
        use axum::{Router, extract::Query, response::Html, routing::get};
        use std::sync::Arc;
        use tokio::sync::oneshot;

        let (tx, rx) = oneshot::channel::<(String, String)>();
        let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

        let app = Router::new().route(
            "/oauth/callback",
            get({
                let tx = tx.clone();
                move |Query(params): Query<std::collections::HashMap<String, String>>| async move {
                    let code = params.get("code").cloned().unwrap_or_default();
                    let state = params.get("state").cloned().unwrap_or_default();
                    
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send((code, state));
                    }
                    
                    Html(r#"<h1>Authentication successful! You can close this window.</h1>"#)
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3434")
            .await
            .map_err(|e| GmailError::Io(e))?;

        info!("Waiting for OAuth callback on http://127.0.0.1:3434/oauth/callback");

        // Run server in background
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        // Wait for callback with timeout
        let (code, state) = tokio::time::timeout(Duration::from_secs(120), rx)
            .await
            .map_err(|_| GmailError::Timeout("OAuth callback timeout".into()))?
            .map_err(|_| GmailError::Auth(anyhow::anyhow!("OAuth callback channel closed").into()))?;

        server_handle.abort();

        if code.is_empty() {
            return Err(GmailError::Auth(anyhow::anyhow!("No authorization code received").into()));
        }

        Ok((code, state))
    }

    /// Save token to disk
    async fn save_token(&self, storage: &TokenStorage) -> Result<()> {
        if let Some(parent) = self.token_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(storage)?;
        tokio::fs::write(&self.token_path, content).await?;
        debug!("Token saved to {:?}", self.token_path);
        Ok(())
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
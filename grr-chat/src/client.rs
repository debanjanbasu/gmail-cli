//! High-performance Google Chat client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in grr-core; this crate
//! adds Chat's URL space, models, and list pagination.

use tracing::info;
use url::Url;

use grr_core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use grr_core::error::{GrrError, Result};
use grr_core::http::{HttpCore, TransportInfo};
use grr_core::runtime::detect_runtime_features;

use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://chat.googleapis.com/v1/";
/// Any authenticated request proves the transport; spaces is the cheapest
/// authenticated GET in Chat's URL space.
const PROBE_PATH: &str = "spaces";

/// Google Chat API client builder.
pub struct ChatClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
}

impl ChatClientBuilder {
    pub fn new() -> Self {
        Self {
            auth: None,
            base_url: None,
        }
    }

    pub fn auth(mut self, auth: GoogleAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Override the Chat API base URL (primarily for test injection).
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<ChatClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid base URL: {e}")))?,
        };

        let core = if has_base_override {
            // Explicit base URL (test injection): skip the transport probe —
            // mock servers speak HTTP/1.1 and QUIC packets would just time out.
            HttpCore::unprobed(auth, grr_core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await
        };
        ChatClient::new(core, base_url)
    }
}

impl Default for ChatClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance Google Chat client.
#[derive(Clone)]
pub struct ChatClient {
    core: HttpCore,
    base_url: Url,
}

impl ChatClient {
    /// Create new client from a connected core (test injection point).
    pub fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "ChatClient initialized: http3={}, io_uring={}",
            features.http3, features.io_uring
        );
        Ok(Self { core, base_url })
    }

    /// The shared HTTP engine (advanced use; typed methods preferred).
    pub fn core(&self) -> &HttpCore {
        &self.core
    }

    /// Transport negotiation details observed during client construction.
    pub fn transport_info(&self) -> &TransportInfo {
        self.core.transport_info()
    }

    /// Which token backend is live ("os-keyring" or "file").
    pub fn token_backend(&self) -> &'static str {
        self.core.auth().token_backend()
    }

    /// Fresh interactive login (PKCE browser flow), dropping any stored
    /// token first so a dead credential can never block consent.
    pub async fn login(&self) -> Result<TokenStorage> {
        self.core.auth().login().await
    }

    /// Start an OAuth device flow; display the challenge to the user.
    pub async fn request_device_code(&self) -> Result<DeviceAuthChallenge> {
        self.core.auth().request_device_code().await
    }

    /// Poll a device-flow challenge. `Ok(None)` means keep waiting.
    pub async fn poll_device_code(
        &self,
        challenge: &mut DeviceAuthChallenge,
    ) -> Result<Option<TokenStorage>> {
        self.core.auth().poll_device_code(challenge).await
    }

    /// Get access token
    pub async fn access_token(&self) -> Result<String> {
        self.core.auth().get_access_token().await
    }

    /// Build API URL with proper error handling
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {e}")))
    }

    /// Resource name of a user-supplied space identifier: `spaces/AAA`
    /// passes through (its trailing segment percent-encoded), a bare
    /// `AAA` is prefixed to `spaces/AAA`.
    fn space_path(&self, space_id: &str) -> String {
        let bare = space_id.strip_prefix("spaces/").unwrap_or(space_id);
        format!("spaces/{}", urlencoding::encode(bare))
    }

    /// URL of a single space resource
    fn space_url(&self, space_id: &str) -> Result<Url> {
        self.api_url(&self.space_path(space_id))
    }

    /// URL of a space's messages collection
    fn messages_url(&self, space_id: &str) -> Result<Url> {
        self.api_url(&format!("{}/messages", self.space_path(space_id)))
    }

    // ══════════════════════════════════════════════════════════════════
    // Chat Operations
    // ══════════════════════════════════════════════════════════════════

    /// List the spaces the user belongs to with automatic pagination up to
    /// `max` entries (all pages when `None`).
    pub async fn list_spaces(&self, max: Option<usize>) -> Result<Vec<Space>> {
        let mut all_spaces = Vec::new();
        let mut page_token: Option<String> = None;
        // spaces.list caps a single page at 1,000 spaces.
        let batch_size = max.map_or(1000, |m| m.min(1000));

        loop {
            let mut request = self
                .core
                .get(self.api_url("spaces")?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let page: ListSpacesResponse = response.json().await?;

            if let Some(spaces) = page.spaces {
                all_spaces.extend(spaces);
            }
            if let Some(m) = max
                && all_spaces.len() >= m
            {
                all_spaces.truncate(m);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_spaces)
    }

    /// Get a single space by ID (`spaces/AAA` or a bare `AAA`)
    pub async fn get_space(&self, space_id: &str) -> Result<Space> {
        let response = self
            .core
            .execute(self.core.get(self.space_url(space_id)?))
            .await?;
        Ok(response.json().await?)
    }

    /// List messages in a space with automatic pagination up to `max`
    /// messages (all pages when `None`).
    pub async fn list_messages(&self, space_id: &str, max: Option<usize>) -> Result<Vec<Message>> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;
        // spaces.messages.list caps a single page at 1,000 messages.
        let batch_size = max.map_or(1000, |m| m.min(1000));

        loop {
            let mut request = self
                .core
                .get(self.messages_url(space_id)?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let page: ListMessagesResponse = response.json().await?;

            if let Some(messages) = page.messages {
                all_messages.extend(messages);
            }
            if let Some(m) = max
                && all_messages.len() >= m
            {
                all_messages.truncate(m);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_messages)
    }

    /// Send a plain-text message to a space (POST `{"text": ...}`).
    ///
    /// Validation stays at the CLI layer (empty text is rejected there
    /// before the client ever sees it); the client passes the body
    /// through untouched.
    pub async fn send_message(&self, space_id: &str, text: &str) -> Result<Message> {
        let request = CreateMessageRequest {
            text: text.to_string(),
        };
        let response = self
            .core
            .execute(self.core.post(self.messages_url(space_id)?).json(&request))
            .await?;
        Ok(response.json().await?)
    }
}

//! High-performance Gmail client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in grr-core; this crate
//! adds Gmail's URL space, models, streaming upload, and search
//! pagination.

use tracing::info;
use url::Url;

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, TransportInfo};
use crate::core::runtime::detect_runtime_features;

use crate::gmail::models::*;

const DEFAULT_BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1/";
const DEFAULT_UPLOAD_BASE_URL: &str = "https://gmail.googleapis.com/upload/gmail/v1/";

/// Gmail API client builder.
pub struct GmailClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
    upload_base_url: Option<Url>,
}

impl GmailClientBuilder {
    pub fn new() -> Self {
        Self {
            auth: None,
            base_url: None,
            upload_base_url: None,
        }
    }

    pub fn auth(mut self, auth: GoogleAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Override the Gmail API base URL (primarily for test injection).
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Override the Gmail media-upload base URL (primarily for test
    /// injection).
    pub fn upload_base_url(mut self, url: Url) -> Self {
        self.upload_base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<GmailClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid base URL: {}", e)))?,
        };
        let upload_base_url = match self.upload_base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_UPLOAD_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid upload base URL: {}", e)))?,
        };

        let core = if has_base_override {
            // Explicit base URL (test injection): skip the transport probe —
            // mock servers speak HTTP/1.1 and QUIC packets would just time out.
            HttpCore::unprobed(auth, crate::core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, "users/me/profile").await
        };
        GmailClient::new(core, base_url, upload_base_url)
    }
}

impl Default for GmailClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance Gmail client.
#[derive(Clone)]
pub struct GmailClient {
    core: HttpCore,
    base_url: Url,
    upload_base_url: Url,
}

impl GmailClient {
    /// Create new client from a connected core (test injection point).
    pub fn new(core: HttpCore, base_url: Url, upload_base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "GmailClient initialized: http3={}, io_uring={}",
            features.http3, features.io_uring
        );
        Ok(Self {
            core,
            base_url,
            upload_base_url,
        })
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

    /// Build API URL with proper error handling
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {}", e)))
    }

    // ══════════════════════════════════════════════════════════════════
    // Search Operations
    // ══════════════════════════════════════════════════════════════════

    /// Search messages with automatic pagination up to `max_results`.
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<MessageRef>> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;
        let batch_size = max_results.min(500);

        loop {
            let mut request = self
                .core
                .get(self.api_url("users/me/messages")?)
                .query(&[("q", query), ("maxResults", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let search_response: SearchResponse = response.json().await?;

            all_messages.extend(search_response.messages);
            if all_messages.len() >= max_results {
                all_messages.truncate(max_results);
                break;
            }

            page_token = search_response.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_messages)
    }
}

mod drafts;
mod history;
mod labels;
mod messages;
mod settings;
mod streaming;
mod threads;
mod watch;

pub use streaming::*;

/// Build RFC 5322 email
fn build_email(
    to: &str,
    subject: &str,
    body: &str,
    cc: Option<&str>,
    bcc: Option<&str>,
) -> Result<String> {
    let mut email = String::new();
    email.push_str(&format!("To: {}\r\n", to));

    if let Some(cc) = cc {
        email.push_str(&format!("Cc: {}\r\n", cc));
    }

    if let Some(bcc) = bcc {
        email.push_str(&format!("Bcc: {}\r\n", bcc));
    }

    email.push_str(&format!("Subject: {}\r\n", subject));
    email.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    email.push_str("\r\n");
    email.push_str(body);

    Ok(email)
}

// Re-exported for downstream users of the old flat layout.
pub use crate::gmail::models::extract_body;

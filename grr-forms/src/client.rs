//! High-performance Google Forms client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in grr-core; this crate
//! adds Forms' URL space, models, and response-list pagination.

use tracing::info;
use url::Url;

use grr_core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use grr_core::error::{GrrError, Result};
use grr_core::http::{HttpCore, TransportInfo};
use grr_core::runtime::detect_runtime_features;

use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://forms.googleapis.com/v1/";
/// Forms has no cheap "list" endpoint to probe: GET "forms" 404s, and any
/// HTTP response — even 404 — proves the transport negotiated.
const PROBE_PATH: &str = "forms";

/// Google Forms API client builder.
pub struct FormsClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
}

impl FormsClientBuilder {
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

    /// Override the Forms API base URL (primarily for test injection).
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<FormsClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid base URL: {}", e)))?,
        };

        let core = if has_base_override {
            // Explicit base URL (test injection): skip the transport probe —
            // mock servers speak HTTP/1.1 and QUIC packets would just time out.
            HttpCore::unprobed(auth, grr_core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await
        };
        FormsClient::new(core, base_url)
    }
}

impl Default for FormsClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance Google Forms client.
#[derive(Clone)]
pub struct FormsClient {
    core: HttpCore,
    base_url: Url,
}

impl FormsClient {
    /// Create new client from a connected core (test injection point).
    pub fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "FormsClient initialized: http3={}, io_uring={}",
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
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {}", e)))
    }

    /// URL of a form resource
    fn form_url(&self, form_id: &str) -> Result<Url> {
        self.api_url(&normalize_form_id(form_id))
    }

    /// URL of a form's responses collection
    fn responses_url(&self, form_id: &str) -> Result<Url> {
        self.api_url(&format!("{}/responses", normalize_form_id(form_id)))
    }

    // ══════════════════════════════════════════════════════════════════
    // Forms Operations
    // ══════════════════════════════════════════════════════════════════

    /// Get a form by ID. Forms has no list endpoint: callers pass the
    /// form ID from the form URL (docs.google.com/forms/d/{FORM_ID}/edit)
    /// or the full `forms/{id}` resource name; both spellings work.
    pub async fn get_form(&self, form_id: &str) -> Result<Form> {
        let response = self
            .core
            .execute(self.core.get(self.form_url(form_id)?))
            .await?;
        Ok(response.json().await?)
    }

    /// List a form's responses with automatic pagination up to `max`
    /// responses (all pages when `None`).
    pub async fn list_responses(
        &self,
        form_id: &str,
        max: Option<usize>,
    ) -> Result<Vec<FormResponse>> {
        let mut all_responses = Vec::new();
        let mut page_token: Option<String> = None;
        // responses.list caps a single page at 5,000 entries.
        let batch_size = max.map_or(5000, |m| m.min(5000));

        loop {
            let mut request = self
                .core
                .get(self.responses_url(form_id)?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let page: ListFormResponsesResponse = response.json().await?;

            if let Some(page_responses) = page.responses {
                all_responses.extend(page_responses);
            }
            if let Some(m) = max
                && all_responses.len() >= m
            {
                all_responses.truncate(m);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_responses)
    }
}

/// Normalize a user-supplied form identifier to the API resource path
/// segment ("forms/{id}"): a bare form ID gets the prefix; an
/// already-prefixed resource name passes through untouched. The ID is
/// percent-encoded so exotic characters survive the URL join.
fn normalize_form_id(form_id: &str) -> String {
    let bare = form_id.strip_prefix("forms/").unwrap_or(form_id);
    format!("forms/{}", urlencoding::encode(bare))
}

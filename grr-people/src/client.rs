//! People (Google Contacts) API client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in grr-core; this crate
//! adds the People API URL space, models, and connection pagination.
//!
//! The People API uses gRPC-transcoding URLs: custom methods are suffixes
//! on a resource path (`people/c123:updateContact`,
//! `people/c123:deleteContact`) or on a collection (`people:searchContacts`,
//! `people:createContact`). `Url::join` is scheme-first, so a bare custom
//! method whose `:` precedes any `/` would parse as a `people:` scheme —
//! those paths are joined with the RFC 3986 `./` relative-segment prefix,
//! which pins the colon into the path instead.

use tracing::info;
use url::Url;

use grr_core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use grr_core::error::{GrrError, Result};
use grr_core::http::{HttpCore, TransportInfo};
use grr_core::runtime::detect_runtime_features;

use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://people.googleapis.com/v1/";
/// Any authenticated response proves the transport, even a 404/401.
const PROBE_PATH: &str = "people/me?personFields=names";
const CONNECTIONS_PATH: &str = "people/me/connections";
/// `people:` would parse as a URL scheme; the `./` prefix forces a
/// relative-path join.
const SEARCH_CONTACTS_PATH: &str = "./people:searchContacts";
const CREATE_CONTACT_PATH: &str = "./people:createContact";
/// Person fields fetched on reads (list/get).
const READ_PERSON_FIELDS: &str = "names,emailAddresses,phoneNumbers,organizations,photos";
/// Person fields returned by prefix search.
const SEARCH_READ_MASK: &str = "names,emailAddresses,phoneNumbers";
/// API hard limit on connections per page.
const MAX_PAGE_SIZE: u32 = 1000;

/// Options for [`PeopleClient::list_connections`].
#[derive(Debug, Clone)]
pub struct ListConnectionsOptions {
    /// Per-request page size, clamped to 1..=1000 (API default: 100).
    pub page_size: u32,
    /// Optional plain-text query matched against contact names
    /// (forwarded as `queryName`).
    pub query_name: Option<String>,
    /// Maximum total connections to collect across pages; 0 = unbounded.
    pub max_results: usize,
}

impl Default for ListConnectionsOptions {
    fn default() -> Self {
        Self {
            page_size: 100,
            query_name: None,
            max_results: 0,
        }
    }
}

/// People API client builder.
pub struct PeopleClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
}

impl PeopleClientBuilder {
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

    /// Override the People API base URL (primarily for test injection).
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<PeopleClient> {
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
        PeopleClient::new(core, base_url)
    }
}

impl Default for PeopleClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// People (Google Contacts) API client.
#[derive(Clone)]
pub struct PeopleClient {
    core: HttpCore,
    base_url: Url,
}

impl PeopleClient {
    /// Create new client from a connected core (test injection point).
    pub fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "PeopleClient initialized: http3={}, io_uring={}",
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

    /// Build API URL with proper error handling.
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {}", e)))
    }

    // ══════════════════════════════════════════════════════════════════
    // Connection Operations
    // ══════════════════════════════════════════════════════════════════

    /// List the authenticated user's connections (their contacts),
    /// following pages until `max_results` is reached or the pages run
    /// out.
    pub async fn list_connections(&self, opts: ListConnectionsOptions) -> Result<Vec<Person>> {
        let batch = opts.page_size.clamp(1, MAX_PAGE_SIZE).to_string();
        let mut connections: Vec<Person> = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut request = self.core.get(self.api_url(CONNECTIONS_PATH)?).query(&[
                ("personFields", READ_PERSON_FIELDS),
                ("pageSize", batch.as_str()),
            ]);

            if let Some(name) = opts.query_name.as_deref() {
                request = request.query(&[("queryName", name)]);
            }
            if let Some(token) = page_token.as_deref() {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListConnectionsResponse = self.core.execute(request).await?.json().await?;

            if let Some(page_connections) = page.connections {
                connections.extend(page_connections);
            }

            if opts.max_results > 0 && connections.len() >= opts.max_results {
                connections.truncate(opts.max_results);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(connections)
    }

    /// Prefix-search the authenticated user's own contacts
    /// (`people:searchContacts`). The query matches prefixes of names,
    /// nicknames, email addresses, phone numbers, and organizations.
    pub async fn search_contacts(&self, query: &str) -> Result<Vec<Person>> {
        let request = self
            .core
            .get(self.api_url(SEARCH_CONTACTS_PATH)?)
            .query(&[("query", query), ("readMask", SEARCH_READ_MASK)]);

        let response: SearchContactsResponse = self.core.execute(request).await?.json().await?;

        Ok(response
            .results
            .unwrap_or_default()
            .into_iter()
            .filter_map(|result| result.person)
            .collect())
    }

    /// Get a single person by resource name (e.g. `people/c123`).
    pub async fn get_person(&self, resource_name: &str) -> Result<Person> {
        let request = self
            .core
            .get(self.api_url(resource_name)?)
            .query(&[("personFields", READ_PERSON_FIELDS)]);

        Ok(self.core.execute(request).await?.json().await?)
    }

    /// Create a new contact in the authenticated user's default contact
    /// group (`people:createContact`).
    pub async fn create_contact(&self, person: Person) -> Result<Person> {
        let request = self
            .core
            .post(self.api_url(CREATE_CONTACT_PATH)?)
            .json(&person);

        Ok(self.core.execute(request).await?.json().await?)
    }

    /// Update specific fields of an existing contact
    /// (`people/{resourceName}:updateContact`).
    ///
    /// `update_fields` names the Person fields the body replaces
    /// (joined into the required `updatePersonFields` mask); the body
    /// should carry the contact's current `etag`, so fetch first with
    /// [`PeopleClient::get_person`].
    pub async fn update_contact(
        &self,
        resource_name: &str,
        person: Person,
        update_fields: &[&str],
    ) -> Result<Person> {
        if update_fields.is_empty() {
            return Err(GrrError::InvalidArgument(
                "update_contact: at least one update field is required".into(),
            ));
        }
        let mask = update_fields.join(",");
        let path = format!("{resource_name}:updateContact");
        let request = self
            .core
            .patch(self.api_url(&path)?)
            .query(&[("updatePersonFields", mask.as_str())])
            .json(&person);

        Ok(self.core.execute(request).await?.json().await?)
    }

    /// Delete a contact (`people/{resourceName}:deleteContact`; the API
    /// answers 204 on success).
    pub async fn delete_contact(&self, resource_name: &str) -> Result<()> {
        let path = format!("{resource_name}:deleteContact");
        self.core
            .execute(self.core.delete(self.api_url(&path)?))
            .await?;
        Ok(())
    }
}

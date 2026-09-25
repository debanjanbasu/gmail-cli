//! High-performance People (Google Contacts) client: typed endpoints over [`HttpCore`].
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

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, TransportInfo};
use crate::core::runtime::detect_runtime_features;

use crate::people::models::*;

const DEFAULT_BASE_URL: &str = "https://people.googleapis.com/v1/";
/// Any authenticated response proves the transport, even a 404/401.
const PROBE_PATH: &str = "people/me?personFields=names";
const CONNECTIONS_PATH: &str = "people/me/connections";
const CONTACT_GROUPS_PATH: &str = "contactGroups";
const OTHER_CONTACTS_PATH: &str = "otherContacts";
/// `people:` would parse as a URL scheme; the `./` prefix forces a
/// relative-path join.
const SEARCH_CONTACTS_PATH: &str = "./people:searchContacts";
const CREATE_CONTACT_PATH: &str = "./people:createContact";
const BATCH_GET_PATH: &str = "./people:batchGet";
/// Person fields fetched on reads (list/get).
const READ_PERSON_FIELDS: &str = "names,emailAddresses,phoneNumbers,organizations,photos";
/// Person fields returned by prefix search.
const SEARCH_READ_MASK: &str = "names,emailAddresses,phoneNumbers";
const GROUP_FIELDS: &str = "clientData,groupType,memberCount,metadata,name";
const CREATE_GROUP_READ_FIELDS: &str = "clientData,groupType,metadata,name";
const OTHER_CONTACTS_READ_MASK: &str = "names,emailAddresses";
const COPY_OTHER_CONTACT_MASK: &str = "names,emailAddresses,phoneNumbers";
const PHOTO_PERSON_FIELDS: &str = "photos";
const READ_SOURCE_TYPE_CONTACT: &str = "READ_SOURCE_TYPE_CONTACT";
/// API hard limit on connections per page.
const MAX_PAGE_SIZE: u32 = 1000;
const MAX_BATCH_GET_RESOURCE_NAMES: usize = 200;
const MAX_CONTACT_GROUP_MEMBERS_MODIFY: usize = 1000;

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
            HttpCore::unprobed(auth, crate::core::http::build_http_client())
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

/// High-performance People (Google Contacts) client.
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
        let batch_size = opts.page_size.clamp(1, MAX_PAGE_SIZE).to_string();
        let mut all_connections = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut request = self.core.get(self.api_url(CONNECTIONS_PATH)?).query(&[
                ("personFields", READ_PERSON_FIELDS),
                ("pageSize", batch_size.as_str()),
            ]);

            if let Some(name) = opts.query_name.as_deref() {
                request = request.query(&[("queryName", name)]);
            }
            if let Some(token) = page_token.as_deref() {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListConnectionsResponse = self.core.execute(request).await?.json().await?;

            if let Some(page_connections) = page.connections {
                all_connections.extend(page_connections);
            }

            if opts.max_results > 0 && all_connections.len() >= opts.max_results {
                all_connections.truncate(opts.max_results);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_connections)
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

    pub async fn list_contact_groups(&self) -> Result<Vec<ContactGroup>> {
        let page_size = MAX_PAGE_SIZE.to_string();
        let mut groups = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut request = self.core.get(self.api_url(CONTACT_GROUPS_PATH)?).query(&[
                ("groupFields", GROUP_FIELDS),
                ("pageSize", page_size.as_str()),
            ]);
            if let Some(token) = page_token.as_deref() {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListContactGroupsResponse = self.core.execute(request).await?.json().await?;
            if let Some(page_groups) = page.contact_groups {
                groups.extend(page_groups);
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(groups)
    }

    pub async fn get_contact_group(&self, resource_name: &str) -> Result<ContactGroup> {
        validate_resource_name(resource_name, "contactGroups")?;
        let request = self
            .core
            .get(self.api_url(resource_name)?)
            .query(&[("groupFields", GROUP_FIELDS)]);

        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn create_contact_group(&self, name: &str) -> Result<ContactGroup> {
        validate_nonempty(name, "contact group name")?;
        let body = CreateContactGroupRequest {
            contact_group: ContactGroup {
                name: Some(name.to_owned()),
                ..ContactGroup::default()
            },
            read_group_fields: Some(CREATE_GROUP_READ_FIELDS.to_owned()),
        };
        let request = self
            .core
            .post(self.api_url(CONTACT_GROUPS_PATH)?)
            .json(&body);

        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn update_contact_group(
        &self,
        resource_name: &str,
        name: &str,
    ) -> Result<ContactGroup> {
        validate_resource_name(resource_name, "contactGroups")?;
        validate_nonempty(name, "contact group name")?;
        let body = UpdateContactGroupRequest {
            contact_group: ContactGroup {
                resource_name: Some(resource_name.to_owned()),
                name: Some(name.to_owned()),
                ..ContactGroup::default()
            },
            update_group_fields: Some("name".to_owned()),
            read_group_fields: Some(GROUP_FIELDS.to_owned()),
        };
        let request = self.core.put(self.api_url(resource_name)?).json(&body);

        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn delete_contact_group(&self, resource_name: &str) -> Result<()> {
        validate_resource_name(resource_name, "contactGroups")?;
        self.core
            .execute(self.core.delete(self.api_url(resource_name)?))
            .await?;
        Ok(())
    }

    pub async fn modify_contact_group_members(
        &self,
        resource_name: &str,
        resource_names_to_add: &[String],
        resource_names_to_remove: &[String],
    ) -> Result<ModifyContactGroupMembersResponse> {
        validate_resource_name(resource_name, "contactGroups")?;
        let total = resource_names_to_add
            .len()
            .checked_add(resource_names_to_remove.len())
            .ok_or_else(|| GrrError::InvalidArgument("too many contact group members".into()))?;
        if total == 0 {
            return Err(GrrError::InvalidArgument(
                "at least one contact group member is required".into(),
            ));
        }
        if total > MAX_CONTACT_GROUP_MEMBERS_MODIFY {
            return Err(GrrError::InvalidArgument(format!(
                "contact group member limit is {MAX_CONTACT_GROUP_MEMBERS_MODIFY}"
            )));
        }
        for resource_name in resource_names_to_add.iter().chain(resource_names_to_remove) {
            validate_resource_name(resource_name, "people")?;
        }

        let body = ModifyContactGroupMembersRequest {
            resource_names_to_add: resource_names_to_add.to_vec(),
            resource_names_to_remove: resource_names_to_remove.to_vec(),
        };
        let path = format!("{resource_name}/members:modify");
        let request = self.core.post(self.api_url(&path)?).json(&body);

        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn get_people(
        &self,
        resource_names: &[String],
        person_fields: &str,
    ) -> Result<GetPeopleResponse> {
        validate_nonempty(person_fields, "personFields")?;
        if resource_names.is_empty() {
            return Err(GrrError::InvalidArgument(
                "at least one resource name is required".into(),
            ));
        }
        if resource_names.len() > MAX_BATCH_GET_RESOURCE_NAMES {
            return Err(GrrError::InvalidArgument(format!(
                "batch get supports at most {MAX_BATCH_GET_RESOURCE_NAMES} resource names"
            )));
        }
        for resource_name in resource_names {
            validate_resource_name(resource_name, "people")?;
        }

        let mut url = self.api_url(BATCH_GET_PATH)?;
        {
            let mut query = url.query_pairs_mut();
            for resource_name in resource_names {
                query.append_pair("resourceNames", resource_name);
            }
            query.append_pair("personFields", person_fields);
        }

        let request = self.core.get(url);
        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn list_other_contacts(&self, max_results: usize) -> Result<Vec<Person>> {
        let page_size = if max_results == 0 {
            MAX_PAGE_SIZE
        } else {
            u32::try_from(max_results.min(MAX_PAGE_SIZE as usize)).unwrap_or(MAX_PAGE_SIZE)
        }
        .to_string();
        let mut contacts = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut request = self.core.get(self.api_url(OTHER_CONTACTS_PATH)?).query(&[
                ("readMask", OTHER_CONTACTS_READ_MASK),
                ("pageSize", page_size.as_str()),
            ]);
            if let Some(token) = page_token.as_deref() {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListOtherContactsResponse = self.core.execute(request).await?.json().await?;
            if let Some(page_contacts) = page.other_contacts {
                contacts.extend(page_contacts);
            }

            if max_results > 0 && contacts.len() >= max_results {
                contacts.truncate(max_results);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(contacts)
    }

    pub async fn copy_other_contact_to_my_contacts_group(
        &self,
        resource_name: &str,
    ) -> Result<Person> {
        validate_resource_name(resource_name, "otherContacts")?;
        let body = CopyOtherContactToMyContactsGroupRequest {
            copy_mask: COPY_OTHER_CONTACT_MASK.to_owned(),
            read_mask: None,
            sources: Some(vec![READ_SOURCE_TYPE_CONTACT.to_owned()]),
        };
        let path = format!("{resource_name}:copyOtherContactToMyContactsGroup");
        let request = self.core.post(self.api_url(&path)?).json(&body);

        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn list_photos(&self, resource_name: &str) -> Result<Vec<Photo>> {
        validate_resource_name(resource_name, "people")?;
        let request = self
            .core
            .get(self.api_url(resource_name)?)
            .query(&[("personFields", PHOTO_PERSON_FIELDS)]);
        let person: Person = self.core.execute(request).await?.json().await?;

        Ok(person.photos.unwrap_or_default())
    }

    pub async fn update_contact_photo(
        &self,
        resource_name: &str,
        photo_bytes: &str,
    ) -> Result<UpdateContactPhotoResponse> {
        validate_resource_name(resource_name, "people")?;
        validate_nonempty(photo_bytes, "base64 photo bytes")?;
        let body = UpdateContactPhotoRequest {
            photo_bytes: photo_bytes.to_owned(),
            person_fields: Some(PHOTO_PERSON_FIELDS.to_owned()),
            sources: Some(vec![READ_SOURCE_TYPE_CONTACT.to_owned()]),
        };
        let path = format!("{resource_name}:updateContactPhoto");
        let request = self.core.patch(self.api_url(&path)?).json(&body);

        Ok(self.core.execute(request).await?.json().await?)
    }

    pub async fn delete_contact_photo(
        &self,
        resource_name: &str,
    ) -> Result<DeleteContactPhotoResponse> {
        validate_resource_name(resource_name, "people")?;
        let path = format!("{resource_name}:deleteContactPhoto");
        let request = self
            .core
            .delete(self.api_url(&path)?)
            .query(&[("personFields", PHOTO_PERSON_FIELDS)]);

        Ok(self.core.execute(request).await?.json().await?)
    }
}

fn validate_resource_name(resource_name: &str, collection: &str) -> Result<()> {
    let prefix = format!("{collection}/");
    let valid = resource_name
        .strip_prefix(&prefix)
        .and_then(|id| (!id.is_empty() && !id.contains('/')).then_some(id))
        .is_some();
    if !valid {
        return Err(GrrError::InvalidArgument(format!(
            "resource name must have the form {collection}/<id>"
        )));
    }
    Ok(())
}

fn validate_nonempty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(GrrError::InvalidArgument(format!("{field} is required")));
    }
    Ok(())
}

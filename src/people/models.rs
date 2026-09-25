//! Google People API response models deserialized with serde
//!
//! The People API is old and lenient: contact points carry their address
//! in `value` (older v1 style), and any field may be omitted from a
//! response. Every model therefore treats all fields as optional,
//! skips `None` on serialization, and ignores unknown fields.

use serde::{Deserialize, Serialize};

/// A person: one contact (or profile) returned by the People API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    /// Server-assigned resource name (e.g. `people/c123`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    /// Opaque concurrency token; `updateContact` requires the current one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub names: Option<Vec<Name>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_addresses: Option<Vec<EmailAddress>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_numbers: Option<Vec<PhoneNumber>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizations: Option<Vec<Organization>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photos: Option<Vec<Photo>>,
}

/// A person's name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
}

/// A person's email address (the address itself lives in `value`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailAddress {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// A person's phone number (the number itself lives in `value`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNumber {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// A person's organization (employer, school, ...).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Organization name (e.g. the company).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The person's title within the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// A person's photo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PhotoMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_primary: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

/// Response of `people.connections.list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConnectionsResponse {
    /// The list of people the user is connected to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connections: Option<Vec<Person>>,
    /// Token to send as `pageToken` for the next page; absent on the
    /// last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    /// Total number of items in the list without pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_items: Option<i64>,
}

/// Response of `people:searchContacts`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchContactsResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<SearchContactsResult>>,
}

/// One search result: a matched person.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchContactsResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<Person>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactGroup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListContactGroupsResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_groups: Option<Vec<ContactGroup>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_items: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactGroupRequest {
    pub contact_group: ContactGroup,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_group_fields: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactGroupRequest {
    pub contact_group: ContactGroup,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_group_fields: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_group_fields: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifyContactGroupMembersRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_names_to_add: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_names_to_remove: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifyContactGroupMembersResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_found_resource_names: Option<Vec<String>>,
    #[serde(
        default,
        rename = "canNotRemoveLastContactGroupResourceNames",
        skip_serializing_if = "Option::is_none"
    )]
    pub can_not_remove_last_contact_group_resource_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPeopleResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responses: Option<Vec<PersonResponse>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<Person>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status_code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_resource_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOtherContactsResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_contacts: Option<Vec<Person>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_size: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyOtherContactToMyContactsGroupRequest {
    pub copy_mask: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_mask: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactPhotoRequest {
    pub photo_bytes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_fields: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactPhotoResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<Person>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteContactPhotoResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<Person>,
}

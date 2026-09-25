//! Google Drive API response models deserialized with serde

use serde::{Deserialize, Serialize};

/// A Drive file (or folder) metadata resource
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: Option<String>,
    pub name: Option<String>,
    pub mime_type: Option<String>,
    /// int64 fields ride the wire as JSON strings
    pub size: Option<String>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    /// IDs of the folders containing this file
    #[serde(default)]
    pub parents: Vec<String>,
    pub web_view_link: Option<String>,
    pub trashed: Option<bool>,
    pub md5_checksum: Option<String>,
    pub shared: Option<bool>,
    pub icon_link: Option<String>,
    pub shortcut_details: Option<ShortcutDetails>,
    pub permission_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutDetails {
    pub resource_key: Option<String>,
    pub shortcut_id: Option<String>,
    pub target_id: Option<String>,
    pub target_mime_type: Option<String>,
}

/// Envelope of files.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub files: Vec<File>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub display_name: Option<String>,
    pub photo_link: Option<String>,
    pub email_address: Option<String>,
    pub permission_id: Option<String>,
    pub me: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub id: Option<String>,
    pub r#type: Option<String>,
    pub role: Option<String>,
    pub email_address: Option<String>,
    pub domain: Option<String>,
    pub display_name: Option<String>,
    pub deleted: Option<bool>,
    pub pending_owner: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionList {
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentReply {
    pub id: Option<String>,
    pub author: Option<User>,
    pub content: Option<String>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub resolved: Option<bool>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: Option<String>,
    pub author: Option<User>,
    pub content: Option<String>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub resolved: Option<bool>,
    pub replies: Option<Vec<CommentReply>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentList {
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub id: Option<String>,
    pub modified_time: Option<String>,
    pub last_modifying_user: Option<User>,
    pub size: Option<String>,
    pub mime_type: Option<String>,
    pub keep_forever: Option<bool>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionList {
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub revisions: Vec<Revision>,
}

#[derive(Debug, Clone)]
pub enum PermissionGrant {
    User { email_address: String },
    Domain { domain: String },
    Anyone,
}

/// Response of about.get
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct About {
    pub user: Option<AboutUser>,
    pub storage_quota: Option<StorageQuota>,
}

/// The authenticated user, as reported by about.get
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutUser {
    pub display_name: Option<String>,
    pub email_address: Option<String>,
}

/// Account storage quota (int64 fields ride the wire as JSON strings)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageQuota {
    pub limit: Option<String>,
    pub usage: Option<String>,
    pub usage_in_drive: Option<String>,
}

/// Optional filters for files.list
#[derive(Debug, Clone, Default)]
pub struct FileListOptions {
    /// Drive query string (e.g. "name contains 'report'", "trashed = false")
    pub q: Option<String>,
    /// Stop after this many files; 0 paginates until exhausted
    pub max_results: usize,
}

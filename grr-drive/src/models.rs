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
}

/// Envelope of files.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub files: Vec<File>,
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

//! High-performance Google Drive client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in core; this module
//! adds Drive's URL space, models, pagination, streaming media transfer, and
//! streaming multipart upload.

use std::path::Path;

use bytes::Bytes;
use futures::{StreamExt, future, stream};
use rand::random;
use reqwest::Response;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::io::ReaderStream;
use tracing::info;
use url::Url;

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, QueryParams, TransportInfo, join_url, parse_url};
use crate::core::runtime::detect_runtime_features;
use crate::core::{Page, paginate};

use crate::drive::models::*;

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/drive/v3/";
const DEFAULT_UPLOAD_BASE_URL: &str = "https://www.googleapis.com/upload/drive/v3/";
const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";
const PROBE_PATH: &str = "files";
const MAX_PAGE_SIZE: usize = 1000;
const MAX_SUBRESOURCE_PAGE_SIZE: usize = 100;
const PERMISSION_FIELDS: &str = "id,type,role,emailAddress,domain,displayName,deleted,pendingOwner";
const COMMENT_FIELDS: &str =
    "id,author(displayName,photoLink),content,createdTime,modifiedTime,resolved,replies";
const COMMENT_LIST_FIELDS: &str = "nextPageToken,comments(id,author(displayName,photoLink),content,createdTime,modifiedTime,resolved,replies)";
const REVISION_FIELDS: &str =
    "id,modifiedTime,lastModifyingUser(displayName,photoLink),size,mimeType,keepForever";
const REVISION_LIST_FIELDS: &str = "nextPageToken,revisions(id,modifiedTime,lastModifyingUser(displayName,photoLink),size,mimeType,keepForever)";

/// Google Drive API client builder.
pub struct DriveClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
    upload_base_url: Option<Url>,
}

impl DriveClientBuilder {
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

    /// Override the Drive API base URL (primarily for test injection).
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Override the Drive media-upload base URL (primarily for test
    /// injection).
    pub fn upload_base_url(mut self, url: Url) -> Self {
        self.upload_base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<DriveClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => parse_url(DEFAULT_BASE_URL, "base")?,
        };
        let upload_base_url = match self.upload_base_url {
            Some(url) => url,
            None => parse_url(DEFAULT_UPLOAD_BASE_URL, "upload base")?,
        };

        let core = if has_base_override {
            HttpCore::unprobed(auth, crate::core::http::build_http_client()?)
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await?
        };
        DriveClient::new(core, base_url, upload_base_url).await
    }
}

impl Default for DriveClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance Google Drive client.
#[derive(Clone)]
pub struct DriveClient {
    core: HttpCore,
    base_url: Url,
    upload_base_url: Url,
}

fn generate_boundary() -> String {
    format!("grr-drive-{:032x}", random::<u128>())
}

fn page_size(max_results: usize, maximum: usize) -> usize {
    if max_results == 0 {
        maximum
    } else {
        max_results.min(maximum)
    }
}

impl DriveClient {
    /// Create new client from a connected core (test injection point).
    pub async fn new(core: HttpCore, base_url: Url, upload_base_url: Url) -> Result<Self> {
        let features = detect_runtime_features().await;
        info!(
            "DriveClient initialized: http3=always, io_uring={}",
            features.io_uring
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

    /// Get access token
    pub async fn access_token(&self) -> Result<String> {
        self.core.auth().get_access_token().await
    }

    fn api_url(&self, path: &str) -> Result<Url> {
        join_url(&self.base_url, path, "API")
    }

    fn upload_url(&self, path: &str) -> Result<Url> {
        join_url(&self.upload_base_url, path, "upload")
    }

    fn file_url(&self, file_id: &str) -> Result<Url> {
        self.api_url(&format!("files/{}", urlencoding::encode(file_id)))
    }

    fn file_collection_url(&self, file_id: &str, collection: &str) -> Result<Url> {
        self.api_url(&format!(
            "files/{}/{collection}",
            urlencoding::encode(file_id)
        ))
    }

    fn file_item_url(&self, file_id: &str, collection: &str, item_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "files/{}/{collection}/{}",
            urlencoding::encode(file_id),
            urlencoding::encode(item_id)
        ))
    }

    fn media_url(&self, file_id: &str) -> Result<Url> {
        let mut url = self.file_url(file_id)?;
        url.query_pairs_mut().append_pair("alt", "media");
        Ok(url)
    }

    fn export_url(&self, file_id: &str, mime_type: &str) -> Result<Url> {
        let mut url = self.file_collection_url(file_id, "export")?;
        url.query_pairs_mut().append_pair("mimeType", mime_type);
        Ok(url)
    }

    async fn write_response_to_path(response: Response, dest: &Path) -> Result<u64> {
        let mut file = BufWriter::new(tokio::fs::File::create(dest).await?);
        let mut written = 0_u64;
        let mut chunks = response.bytes_stream();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            written += chunk.len() as u64;
        }
        file.flush().await?;
        Ok(written)
    }

    pub async fn list_files(&self, opts: FileListOptions) -> Result<Vec<File>> {
        let max = if opts.max_results == 0 {
            None
        } else {
            Some(opts.max_results)
        };
        let q = opts.q.as_deref();
        let batch_size = page_size(opts.max_results, MAX_PAGE_SIZE);

        paginate(max, move |page_token| async move {
            let params = QueryParams::new()
                .add("pageSize", batch_size.to_string())
                .add_optional("q", q)
                .add_page_token(page_token.as_deref());
            let page: FileList = self
                .core
                .execute_json(params.apply(self.core.get(self.api_url("files")?)))
                .await?;
            Ok(Page::new(page.files, page.next_page_token))
        })
        .await
    }

    pub async fn get_file(&self, file_id: &str) -> Result<File> {
        let response = self
            .core
            .execute(self.core.get(self.file_url(file_id)?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<File> {
        let mut body = serde_json::Map::new();
        body.insert("name".into(), name.into());
        body.insert("mimeType".into(), FOLDER_MIME_TYPE.into());
        if let Some(parent_id) = parent_id {
            body.insert(
                "parents".into(),
                serde_json::Value::Array(vec![parent_id.into()]),
            );
        }

        let response = self
            .core
            .execute(self.core.post(self.api_url("files")?).json(&body))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn download_file(&self, file_id: &str, dest: &Path) -> Result<u64> {
        let response = self
            .core
            .execute(self.core.get(self.media_url(file_id)?))
            .await?;
        Self::write_response_to_path(response, dest).await
    }

    pub async fn download_file_bytes(&self, file_id: &str) -> Result<Vec<u8>> {
        let response = self
            .core
            .execute(self.core.get(self.media_url(file_id)?))
            .await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub async fn export_file(&self, file_id: &str, mime_type: &str, dest: &Path) -> Result<u64> {
        let response = self
            .core
            .execute(self.core.get(self.export_url(file_id, mime_type)?))
            .await
            .map_err(|error| match error {
                GrrError::PermissionDenied(message) => GrrError::Api {
                    status: 403,
                    message: format!("Drive export failed: {message}"),
                },
                other => other,
            })?;
        Self::write_response_to_path(response, dest).await
    }

    pub async fn upload_file(
        &self,
        path: &Path,
        name: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<File> {
        let filename = name.map_or_else(
            || {
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                )
            },
            str::to_owned,
        );
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let mut metadata = serde_json::Map::new();
        metadata.insert("name".into(), filename.into());
        if let Some(parent_id) = parent_id {
            metadata.insert(
                "parents".into(),
                serde_json::Value::Array(vec![parent_id.into()]),
            );
        }
        let metadata_json = serde_json::Value::Object(metadata).to_string();

        let boundary = generate_boundary();
        let prefix = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata_json}\r\n--{boundary}\r\nContent-Type: {mime}\r\n\r\n"
        );
        let suffix = format!("\r\n--{boundary}--\r\n");
        let media = tokio::fs::File::open(path).await?;
        let media_length = media.metadata().await?.len();
        let content_length = (prefix.len() + suffix.len()) as u64 + media_length;
        let media_stream = ReaderStream::with_capacity(media, 64 * 1024);
        let prefix_stream = stream::once(future::ready(Ok::<Bytes, std::io::Error>(Bytes::from(
            prefix,
        ))));
        let suffix_stream = stream::once(future::ready(Ok::<Bytes, std::io::Error>(Bytes::from(
            suffix,
        ))));
        let body_stream = prefix_stream.chain(media_stream).chain(suffix_stream);

        let mut url = self.upload_url("files")?;
        url.query_pairs_mut().append_pair("uploadType", "multipart");

        let response = self
            .core
            .execute(
                self.core
                    .post(url)
                    .header(
                        CONTENT_TYPE,
                        format!("multipart/related; boundary={boundary}"),
                    )
                    .header(CONTENT_LENGTH, content_length.to_string())
                    .body(reqwest::Body::wrap_stream(body_stream)),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn rename_file(&self, file_id: &str, new_name: &str) -> Result<File> {
        let response = self
            .core
            .execute(
                self.core
                    .patch(self.file_url(file_id)?)
                    .json(&serde_json::json!({ "name": new_name })),
            )
            .await?;
        Ok(response.json().await?)
    }

    async fn set_trashed(&self, file_id: &str, trashed: bool) -> Result<File> {
        let response = self
            .core
            .execute(
                self.core
                    .patch(self.file_url(file_id)?)
                    .json(&serde_json::json!({ "trashed": trashed })),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn trash_file(&self, file_id: &str) -> Result<File> {
        self.set_trashed(file_id, true).await
    }

    pub async fn restore_file(&self, file_id: &str) -> Result<File> {
        self.set_trashed(file_id, false).await
    }

    pub async fn copy_file(
        &self,
        file_id: &str,
        name: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<File> {
        let mut body = serde_json::Map::new();
        if let Some(name) = name {
            body.insert("name".into(), name.into());
        }
        if let Some(parent_id) = parent_id {
            body.insert(
                "parents".into(),
                serde_json::Value::Array(vec![parent_id.into()]),
            );
        }

        let response = self
            .core
            .execute(
                self.core
                    .post(self.file_collection_url(file_id, "copy")?)
                    .json(&body),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.file_url(file_id)?))
            .await?;
        Ok(())
    }

    pub async fn empty_trash(&self) -> Result<()> {
        self.core
            .execute(self.core.post(self.api_url("files/emptyTrash")?))
            .await?;
        Ok(())
    }

    pub async fn list_permissions(
        &self,
        file_id: &str,
        max_results: usize,
    ) -> Result<Vec<Permission>> {
        let max = if max_results == 0 {
            None
        } else {
            Some(max_results)
        };
        let batch_size = page_size(max_results, MAX_SUBRESOURCE_PAGE_SIZE);

        paginate(max, move |page_token| async move {
            let params = QueryParams::new()
                .add("pageSize", batch_size.to_string())
                .add("fields", PERMISSION_FIELDS)
                .add_page_token(page_token.as_deref());
            let page: PermissionList = self
                .core
                .execute_json(
                    params.apply(
                        self.core
                            .get(self.file_collection_url(file_id, "permissions")?),
                    ),
                )
                .await?;
            Ok(Page::new(page.permissions, page.next_page_token))
        })
        .await
    }

    pub async fn create_permission(
        &self,
        file_id: &str,
        role: &str,
        grant: &PermissionGrant,
        send_notification_email: bool,
    ) -> Result<Permission> {
        let mut body = serde_json::Map::new();
        body.insert("role".into(), role.into());
        match grant {
            PermissionGrant::User { email_address } => {
                body.insert("type".into(), "user".into());
                body.insert("emailAddress".into(), email_address.as_str().into());
            }
            PermissionGrant::Domain { domain } => {
                body.insert("type".into(), "domain".into());
                body.insert("domain".into(), domain.as_str().into());
            }
            PermissionGrant::Anyone => {
                body.insert("type".into(), "anyone".into());
            }
        }

        let mut request = self
            .core
            .post(self.file_collection_url(file_id, "permissions")?)
            .query(&[("fields", PERMISSION_FIELDS)])
            .json(&body);
        if matches!(grant, PermissionGrant::User { .. }) {
            let notification = send_notification_email.to_string();
            request = request.query(&[("sendNotificationEmail", notification.as_str())]);
        }

        let response = self.core.execute(request).await?;
        Ok(response.json().await?)
    }

    pub async fn delete_permission(&self, file_id: &str, permission_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.file_item_url(file_id, "permissions", permission_id)?),
            )
            .await?;
        Ok(())
    }

    pub async fn list_comments(&self, file_id: &str, max_results: usize) -> Result<Vec<Comment>> {
        let max = if max_results == 0 {
            None
        } else {
            Some(max_results)
        };
        let batch_size = page_size(max_results, MAX_SUBRESOURCE_PAGE_SIZE);

        paginate(max, move |page_token| async move {
            let params = QueryParams::new()
                .add("pageSize", batch_size.to_string())
                .add("fields", COMMENT_LIST_FIELDS)
                .add_page_token(page_token.as_deref());
            let page: CommentList = self
                .core
                .execute_json(
                    params.apply(
                        self.core
                            .get(self.file_collection_url(file_id, "comments")?),
                    ),
                )
                .await?;
            Ok(Page::new(page.comments, page.next_page_token))
        })
        .await
    }

    pub async fn get_comment(&self, file_id: &str, comment_id: &str) -> Result<Comment> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.file_item_url(file_id, "comments", comment_id)?)
                    .query(&[("fields", COMMENT_FIELDS)]),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_comment(&self, file_id: &str, content: &str) -> Result<Comment> {
        let response = self
            .core
            .execute(
                self.core
                    .post(self.file_collection_url(file_id, "comments")?)
                    .query(&[("fields", COMMENT_FIELDS)])
                    .json(&serde_json::json!({ "content": content })),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_comment(&self, file_id: &str, comment_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.file_item_url(file_id, "comments", comment_id)?),
            )
            .await?;
        Ok(())
    }

    pub async fn list_revisions(&self, file_id: &str, max_results: usize) -> Result<Vec<Revision>> {
        let max = if max_results == 0 {
            None
        } else {
            Some(max_results)
        };
        let batch_size = page_size(max_results, MAX_SUBRESOURCE_PAGE_SIZE);

        paginate(max, move |page_token| async move {
            let params = QueryParams::new()
                .add("pageSize", batch_size.to_string())
                .add("fields", REVISION_LIST_FIELDS)
                .add_page_token(page_token.as_deref());
            let page: RevisionList = self
                .core
                .execute_json(
                    params.apply(
                        self.core
                            .get(self.file_collection_url(file_id, "revisions")?),
                    ),
                )
                .await?;
            Ok(Page::new(page.revisions, page.next_page_token))
        })
        .await
    }

    pub async fn get_revision(&self, file_id: &str, revision_id: &str) -> Result<Revision> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.file_item_url(file_id, "revisions", revision_id)?)
                    .query(&[("fields", REVISION_FIELDS)]),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn about(&self) -> Result<About> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url("about")?)
                    .query(&[("fields", "user,storageQuota")]),
            )
            .await?;
        Ok(response.json().await?)
    }
}

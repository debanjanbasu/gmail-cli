//! Google Drive API client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in grr-core; this crate
//! adds Drive's URL space, models, files.list pagination, streaming media
//! download, and hand-rolled multipart/related upload (the Drive API has
//! no cheap resumable session for small files — multipart is one round
//! trip).

use std::io::Write;
use std::path::Path;

use futures::StreamExt;
use rand::random;
use reqwest::header::CONTENT_TYPE;
use tokio::io::AsyncWriteExt;
use tracing::info;
use url::Url;

use grr_core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use grr_core::error::{GrrError, Result};
use grr_core::http::{HttpCore, TransportInfo};
use grr_core::runtime::detect_runtime_features;

use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/drive/v3/";
const DEFAULT_UPLOAD_BASE_URL: &str = "https://www.googleapis.com/upload/drive/v3/";
/// GET files is valid bare (files.list); any HTTP response proves transport.
const PROBE_PATH: &str = "files";
/// files.list caps a single page at 1,000 entries.
const MAX_PAGE_SIZE: usize = 1000;

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
            HttpCore::unprobed(auth, grr_core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await
        };
        DriveClient::new(core, base_url, upload_base_url)
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

/// Random hex multipart boundary (128 bits of entropy: the uploaded media
/// can never contain it by accident).
fn generate_boundary() -> String {
    format!("grr-drive-{:032x}", random::<u128>())
}

impl DriveClient {
    /// Create new client from a connected core (test injection point).
    pub fn new(core: HttpCore, base_url: Url, upload_base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "DriveClient initialized: http3={}, io_uring={}",
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

    /// Get access token
    pub async fn access_token(&self) -> Result<String> {
        self.core.auth().get_access_token().await
    }

    /// Build an API URL with proper error handling
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {}", e)))
    }

    /// Build a media-upload URL from the upload base.
    fn upload_url(&self, path: &str) -> Result<Url> {
        self.upload_base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid upload URL: {}", e)))
    }

    /// URL of a single file resource (file IDs are path segments and may
    /// contain reserved characters — always percent-encode).
    fn file_url(&self, file_id: &str) -> Result<Url> {
        self.api_url(&format!("files/{}", urlencoding::encode(file_id)))
    }

    // ══════════════════════════════════════════════════════════════════
    // Files Operations
    // ══════════════════════════════════════════════════════════════════

    /// List files with automatic pagination up to `opts.max_results`
    /// entries (all pages when 0).
    pub async fn list_files(&self, opts: FileListOptions) -> Result<Vec<File>> {
        let mut all_files = Vec::new();
        let mut page_token: Option<String> = None;
        let batch_size = if opts.max_results == 0 {
            MAX_PAGE_SIZE
        } else {
            opts.max_results.min(MAX_PAGE_SIZE)
        };

        loop {
            let mut request = self
                .core
                .get(self.api_url("files")?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(q) = &opts.q {
                request = request.query(&[("q", q)]);
            }
            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let page: FileList = response.json().await?;

            all_files.extend(page.files);
            if opts.max_results > 0 && all_files.len() >= opts.max_results {
                all_files.truncate(opts.max_results);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_files)
    }

    /// Get a single file's metadata by ID
    pub async fn get_file(&self, file_id: &str) -> Result<File> {
        let response = self
            .core
            .execute(self.core.get(self.file_url(file_id)?))
            .await?;
        Ok(response.json().await?)
    }

    /// Stream a file's content (`alt=media`) chunk-by-chunk to `dest`,
    /// so peak memory stays proportional to one chunk, not the file.
    /// Returns the number of bytes written.
    pub async fn download_file(&self, file_id: &str, dest: &Path) -> Result<u64> {
        let mut url = self.file_url(file_id)?;
        url.query_pairs_mut().append_pair("alt", "media");

        let response = self.core.execute(self.core.get(url)).await?;

        let mut file = tokio::fs::File::create(dest).await?;
        let mut written: u64 = 0;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            written += chunk.len() as u64;
        }
        file.flush().await?;
        Ok(written)
    }

    /// Download a file's content (`alt=media`) into memory.
    pub async fn download_file_bytes(&self, file_id: &str) -> Result<Vec<u8>> {
        let mut url = self.file_url(file_id)?;
        url.query_pairs_mut().append_pair("alt", "media");

        let response = self.core.execute(self.core.get(url)).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Upload a local file via multipart/related (one request: JSON
    /// metadata part + raw media part). `name` overrides the uploaded
    /// name (defaults to the local file name); `parent_id` targets a
    /// folder (defaults to the My Drive root).
    pub async fn upload_file(
        &self,
        path: &Path,
        name: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<File> {
        let data = tokio::fs::read(path).await?;
        let filename = match name {
            Some(n) => n.to_string(),
            None => path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ),
        };
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let mut metadata = serde_json::Map::new();
        metadata.insert("name".into(), serde_json::Value::String(filename));
        if let Some(parent) = parent_id {
            metadata.insert(
                "parents".into(),
                serde_json::Value::Array(vec![serde_json::Value::String(parent.to_string())]),
            );
        }
        let metadata_json = serde_json::Value::Object(metadata).to_string();

        // Hand-rolled multipart/related: part 1 JSON metadata, part 2 raw
        // media bytes, matching how grr-gmail assembles MIME bodies.
        let boundary = generate_boundary();
        let mut body =
            Vec::with_capacity(data.len() + metadata_json.len() + boundary.len() * 3 + 192);
        write!(
            body,
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n"
        )?;
        body.extend_from_slice(metadata_json.as_bytes());
        write!(body, "\r\n--{boundary}\r\nContent-Type: {mime}\r\n\r\n")?;
        body.extend_from_slice(&data);
        write!(body, "\r\n--{boundary}--\r\n")?;

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
                    .body(body),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Rename a file (PATCH a partial metadata body; only `name` changes)
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

    /// Delete a file (204 No Content on success)
    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.file_url(file_id)?))
            .await?;
        Ok(())
    }

    // ══════════════════════════════════════════════════════════════════
    // About / Quota
    // ══════════════════════════════════════════════════════════════════

    /// The user's profile and Drive storage quota (about.get)
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

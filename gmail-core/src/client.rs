//! High-performance Gmail API client with HTTP/3, connection pooling, and retry logic

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, ACCEPT_ENCODING};
use reqwest::Client as ReqwestClient;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

use crate::auth::GmailAuth;
use crate::config::{GmailConfig, PerformanceConfig};
use crate::error::{GmailError, Result};
use crate::models::*;
use crate::runtime::{detect_runtime_features, RuntimeFeatures};

/// Gmail API client builder
pub struct GmailClientBuilder {
    config: GmailConfig,
    auth: Option<GmailAuth>,
    http_client: Option<ReqwestClient>,
}

impl GmailClientBuilder {
    pub fn new(config: GmailConfig) -> Self {
        Self {
            config,
            auth: None,
            http_client: None,
        }
    }

    pub fn auth(mut self, auth: GmailAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn http_client(mut self, client: ReqwestClient) -> Self {
        self.http_client = Some(client);
        self
    }

    pub async fn build(self) -> Result<GmailClient> {
        let auth = self.auth.expect("Auth is required");
        let http_client = self.http_client.unwrap_or_else(|| build_http_client(&self.config.performance));
        
        GmailClient::new(auth, http_client, self.config).await
    }
}

/// High-performance Gmail client
#[derive(Clone)]
pub struct GmailClient {
    auth: Arc<GmailAuth>,
    http_client: ReqwestClient,
    config: GmailConfig,
    semaphore: Arc<Semaphore>,
    base_url: Url,
    #[allow(dead_code)]
    runtime_features: RuntimeFeatures,
}

impl GmailClient {
    /// Create new client
    pub async fn new(
        auth: GmailAuth,
        http_client: ReqwestClient,
        config: GmailConfig,
    ) -> Result<Self> {
        let runtime_features = detect_runtime_features();
        
        let max_concurrent = config.performance.max_concurrent;
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        let base_url = Url::parse("https://gmail.googleapis.com/gmail/v1/")
            .map_err(|e| GmailError::Config(format!("Invalid base URL: {}", e)))?;

        info!(
            "GmailClient initialized: http3={}, io_uring={}, simd={}, concurrency={}",
            runtime_features.http3,
            runtime_features.io_uring,
            runtime_features.simd,
            max_concurrent
        );

        Ok(Self {
            auth: Arc::new(auth),
            http_client,
            config,
            semaphore,
            base_url,
            runtime_features,
        })
    }

    /// Get access token
    pub async fn access_token(&self) -> Result<String> {
        self.auth.get_access_token().await
    }

    // ═══════════════════════════════════════════════════════════════════
    // Search Operations
    // ══════════════════════════════════════════════════════════════════

    /// Search messages with streaming support
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<MessageRef>> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;
        let batch_size = max_results.min(500);

        loop {
            let mut request = self
                .http_client
                .get(self.base_url.join("users/me/messages").unwrap())
                .query(&[("q", query), ("maxResults", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.execute_with_retry(request).await?;
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

    /// Stream search results
    pub async fn search_stream<'a>(
        &'a self,
        query: &'a str,
        batch_size: usize,
    ) -> Result<impl futures::Stream<Item = Result<Vec<MessageRef>>> + 'a> {
        use async_stream::stream;
        
        let mut page_token: Option<String> = None;
        let client = self.clone();

        Ok(stream! {
            loop {
                let mut request = client
                    .http_client
                    .get(client.base_url.join("users/me/messages").unwrap())
                    .query(&[("q", query), ("maxResults", &batch_size.to_string())]);

                if let Some(token) = &page_token {
                    request = request.query(&[("pageToken", token)]);
                }

                match client.execute_with_retry(request).await {
                    Ok(response) => {
                        match response.json::<SearchResponse>().await {
                            Ok(search_response) => {
                                if search_response.messages.is_empty() {
                                    break;
                                }
                                yield Ok(search_response.messages);
                                page_token = search_response.next_page_token;
                                if page_token.is_none() {
                                    break;
                                }
                            }
                            Err(e) => {
                                yield Err(GmailError::Http(e));
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        })
    }

    // ═══════════════════════════════════════════════════════════════════
    // Message Operations
    // ══════════════════════════════════════════════════════════════════

    /// Get message by ID
    pub async fn get_message(&self, message_id: &str, format: Option<&str>) -> Result<Message> {
        let mut request = self
            .http_client
            .get(self.base_url.join(&format!("users/me/messages/{}", message_id)).unwrap());

        if let Some(fmt) = format {
            request = request.query(&[("format", fmt)]);
        }

        let response = self.execute_with_retry(request).await?;
        Ok(response.json().await?)
    }

    /// Get message metadata only (lightweight)
    pub async fn get_message_metadata(&self, message_id: &str) -> Result<Message> {
        self.get_message(message_id, Some("metadata")).await
    }

    /// Get message raw (RFC 822)
    pub async fn get_message_raw(&self, message_id: &str) -> Result<String> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join(&format!("users/me/messages/{}", message_id)).unwrap())
                .query(&[("format", "raw")])
        ).await?;
        let message: Message = response.json().await?;
        message.raw.ok_or_else(|| GmailError::NotFound("Raw message not available".into()))
    }

    /// Get thread by ID
    pub async fn get_thread(&self, thread_id: &str) -> Result<Thread> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join(&format!("users/me/threads/{}", thread_id)).unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    /// Get attachment
    pub async fn get_attachment(&self, message_id: &str, attachment_id: &str) -> Result<Attachment> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join(&format!("users/me/messages/{}/attachments/{}", message_id, attachment_id)).unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Send Operations
    // ══════════════════════════════════════════════════════════════════

    /// Send email
    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<Message> {
        let email = build_email(to, subject, body, None, None)?;
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
        
        let request = SendMessageRequest { raw, thread_id: None };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/messages/send").unwrap())
                .json(&request)
        ).await?;
        
        Ok(response.json().await?)
    }

    /// Send email with CC/BCC
    pub async fn send_with_options(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        cc: Option<&str>,
        bcc: Option<&str>,
    ) -> Result<Message> {
        let email = build_email(to, subject, body, cc, bcc)?;
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
        
        let request = SendMessageRequest { raw, thread_id: None };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/messages/send").unwrap())
                .json(&request)
        ).await?;
        
        Ok(response.json().await?)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Draft Operations
    // ══════════════════════════════════════════════════════════════════

    /// Create draft
    pub async fn create_draft(&self, to: &str, subject: &str, body: &str) -> Result<Draft> {
        let email = build_email(to, subject, body, None, None)?;
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
        
        let request = CreateDraftRequest {
            message: DraftMessage { raw, thread_id: None },
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/drafts").unwrap())
                .json(&request)
        ).await?;
        
        Ok(response.json().await?)
    }

    /// List drafts
    pub async fn list_drafts(&self, max_results: usize) -> Result<Vec<Draft>> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join("users/me/drafts").unwrap())
                .query(&[("maxResults", &max_results.to_string())])
        ).await?;
        
        let list_response: ListResponse<Draft> = response.json().await?;
        Ok(list_response.items)
    }

    /// Get draft
    pub async fn get_draft(&self, draft_id: &str) -> Result<Draft> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join(&format!("users/me/drafts/{}", draft_id)).unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    /// Send draft
    pub async fn send_draft(&self, draft_id: &str) -> Result<Message> {
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join(&format!("users/me/drafts/{}/send", draft_id)).unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    /// Delete draft
    pub async fn delete_draft(&self, draft_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.base_url.join(&format!("users/me/drafts/{}", draft_id)).unwrap())
        ).await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════
    // Label Operations
    // ══════════════════════════════════════════════════════════════════

    /// List labels
    pub async fn list_labels(&self) -> Result<Vec<Label>> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join("users/me/labels").unwrap())
        ).await?;
        let list_response: ListResponse<Label> = response.json().await?;
        Ok(list_response.items)
    }

    /// Create label
    pub async fn create_label(&self, name: &str) -> Result<Label> {
        let request = CreateLabelRequest {
            name: name.to_string(),
            label_list_visibility: Some("labelShow".to_string()),
            message_list_visibility: Some("show".to_string()),
            color: None,
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/labels").unwrap())
                .json(&request)
        ).await?;
        Ok(response.json().await?)
    }

    /// Modify labels on message
    pub async fn modify_labels(
        &self,
        message_id: &str,
        add_labels: &[String],
        remove_labels: &[String],
    ) -> Result<Message> {
        let request = ModifyLabelsRequest {
            add_label_ids: add_labels.to_vec(),
            remove_label_ids: remove_labels.to_vec(),
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join(&format!("users/me/messages/{}/modify", message_id)).unwrap())
                .json(&request)
        ).await?;
        Ok(response.json().await?)
    }

    /// Batch modify labels
    pub async fn batch_modify_labels(
        &self,
        message_ids: &[String],
        add_labels: &[String],
        remove_labels: &[String],
    ) -> Result<()> {
        let request = BatchModifyLabelsRequest {
            ids: message_ids.to_vec(),
            add_label_ids: add_labels.to_vec(),
            remove_label_ids: remove_labels.to_vec(),
        };
        
        self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/messages/batchModify").unwrap())
                .json(&request)
        ).await?;
        Ok(())
    }

    /// Delete label
    pub async fn delete_label(&self, label_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.base_url.join(&format!("users/me/labels/{}", label_id)).unwrap())
        ).await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════
    // Message Operations (Trash, Delete, etc.)
    // ══════════════════════════════════════════════════════════════════

    /// Trash message
    pub async fn trash_message(&self, message_id: &str) -> Result<Message> {
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join(&format!("users/me/messages/{}/trash", message_id)).unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    /// Untrash message
    pub async fn untrash_message(&self, message_id: &str) -> Result<Message> {
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join(&format!("users/me/messages/{}/untrash", message_id)).unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    /// Delete message permanently
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.base_url.join(&format!("users/me/messages/{}", message_id)).unwrap())
        ).await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════
    // History & Profile
    // ═════════════════════════════════════════════════════════════════

    /// Get history
    pub async fn get_history(&self, start_history_id: &str) -> Result<Vec<History>> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join("users/me/history").unwrap())
                .query(&[("startHistoryId", start_history_id)])
        ).await?;
        let list_response: ListResponse<History> = response.json().await?;
        Ok(list_response.items)
    }

    /// Get profile
    pub async fn get_profile(&self) -> Result<Profile> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join("users/me/profile").unwrap())
        ).await?;
        Ok(response.json().await?)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Send-as Operations
    // ═════════════════════════════════════════════════════════════════

    /// List send-as aliases
    pub async fn list_send_as(&self) -> Result<Vec<SendAs>> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.base_url.join("users/me/settings/sendAs").unwrap())
        ).await?;
        let list_response: ListResponse<SendAs> = response.json().await?;
        Ok(list_response.items)
    }

    // ══════════════════════════════════════════════════════════════════
    // Watch (Push Notifications)
    // ════════════════════════════════════════════════════════════════

    /// Setup watch
    pub async fn watch(&self, topic_name: &str, label_ids: Option<Vec<String>>) -> Result<WatchResponse> {
        let request = WatchRequest {
            topic_name: topic_name.to_string(),
            label_ids,
            label_filter_action: None,
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/watch").unwrap())
                .json(&request)
        ).await?;
        Ok(response.json().await?)
    }

    /// Stop watch
    pub async fn stop_watch(&self) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/stop").unwrap())
        ).await?;
        Ok(())
    }

    // ═════════════════════════════════════════════════════════════════
    // Import
    // ═════════════════════════════════════════════════════════════════

    /// Import message from RFC 822
    pub async fn import_message(&self, raw_rfc822: &str) -> Result<Message> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_rfc822.as_bytes());
        
        let request = ImportMessageRequest {
            raw,
            internal_date_source: Some("dateHeader".to_string()),
            deleted: Some(false),
            never_spam: Some(false),
            process_for_calendar: Some(false),
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.base_url.join("users/me/messages/import").unwrap())
                .json(&request)
        ).await?;
        Ok(response.json().await?)
    }

    // ══════════════════════════════════════════════════════════════════
    // Internal: Execute with retry, rate limiting, and concurrency control
    // ═════════════════════════════════════════════════════════════════

    async fn execute_with_retry(&self, mut request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        // Acquire semaphore for concurrency control
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| GmailError::Internal("Semaphore closed".into()))?;

        // Add auth header
        let token = self.auth.get_access_token().await?;
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));

        let mut last_error = None;
        
        for attempt in 0..=self.config.performance.retry_attempts {
            let response = request
                .try_clone()
                .ok_or_else(|| GmailError::Internal("Failed to clone request".into()))?
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    
                    if status.is_success() {
                        return Ok(resp);
                    }
                    
                    // Handle specific error codes
                    match status.as_u16() {
                        401 => {
                            // Token might be expired, force refresh
                            warn!("401 Unauthorized, forcing token refresh");
                            self.auth.revoke().await.ok();
                            continue; // Retry with new token
                        }
                        429 => {
                            // Rate limited - extract retry-after header
                            let retry_after = resp
                                .headers()
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(60);
                            return Err(GmailError::RateLimited { retry_after_secs: retry_after });
                        }
                        403 => {
                            return Err(GmailError::PermissionDenied(
                                "Insufficient permissions".into()
                            ));
                        }
                        404 => {
                            return Err(GmailError::NotFound("Resource not found".into()));
                        }
                        500..=599 => {
                            // Server error, retry
                            last_error = Some(GmailError::Api {
                                status: status.as_u16(),
                                message: "Server error".into(),
                            });
                        }
                        _ => {
                            let error_text = resp.text().await.unwrap_or_default();
                            return Err(GmailError::Api {
                                status: status.as_u16(),
                                message: error_text,
                            });
                        }
                    }
                }
                Err(e) => {
                    if e.is_timeout() || e.is_connect() || e.is_request() {
                        last_error = Some(GmailError::Http(e));
                    } else {
                        return Err(GmailError::Http(e));
                    }
                }
            }

            // Exponential backoff
            if attempt < self.config.performance.retry_attempts {
                let backoff = u64::from(self.config.performance.retry_backoff_ms) * (2_u64.pow(attempt));
                debug!("Request failed, retrying in {:?} (attempt {}/{})", backoff, attempt + 1, self.config.performance.retry_attempts);
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| GmailError::Internal("Max retries exceeded".into())))
    }
}

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

/// Build HTTP client with all performance features
fn build_http_client(perf: &PerformanceConfig) -> ReqwestClient {
    let mut builder = ReqwestClient::builder()
        .timeout(perf.request_timeout())
        .connect_timeout(perf.connect_timeout())
        .pool_max_idle_per_host(perf.connection_pool_size)
        .pool_idle_timeout(Duration::from_secs(120))
        .tcp_keepalive(perf.tcp_keepalive())
        .http2_keep_alive_interval(perf.http2_keepalive_interval())
        .http2_adaptive_window(perf.http2_adaptive_window)
        .brotli(perf.enable_brotli)
        .zstd(perf.enable_zstd);

    // HTTP/3
    #[cfg(feature = "http3")]
    if perf.enable_http3 {
        builder = builder.http3_prior_knowledge();
    }

    // HTTP/2
    if perf.enable_http2 {
        builder = builder.http2_prior_knowledge();
    }

    // Default headers
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("zstd, br, gzip, deflate"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    builder = builder.default_headers(headers);

    builder.build().expect("Failed to build HTTP client")
}
//! High-performance Gmail API client with HTTP/3, connection pooling, and retry logic

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

use crate::auth::GmailAuth;
use crate::config::{GmailConfig, PerformanceConfig};
use crate::error::{GmailError, Result};
use crate::models::*;
use crate::runtime::{detect_runtime_features, RuntimeFeatures};

mod drafts;
mod history;
mod labels;
mod messages;
mod settings;
mod threads;
mod watch;

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
        let auth = self.auth.ok_or_else(|| GmailError::Config("Auth is required".into()))?;
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

    /// Force re-authentication by invalidating token
    pub async fn force_refresh(&self) -> Result<()> {
        self.auth.force_refresh().await
    }

    /// Build API URL with proper error handling
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url.join(path).map_err(|e| GmailError::Config(format!("Invalid API URL: {}", e)))
    }

    // ���������������������������������������������������������������������������������������������������������������������������������������
    // Search Operations
    // �������������������������������������������������������������������������������������������������������������������������������������

    /// Search messages with streaming support
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<MessageRef>> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;
        let batch_size = max_results.min(500);

        loop {
            let mut request = self
                .http_client
                .get(self.api_url("users/me/messages")?)
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
    pub fn search_stream(
        &self,
        query: String,
        batch_size: usize,
    ) -> impl futures::Stream<Item = Result<Vec<MessageRef>>> {
        use futures::stream::unfold;

        let client = self.clone();

        unfold(
            Some((query, batch_size, None::<String>)),
            move |state| {
                let client = client.clone();
                async move {
                    let (query, batch_size, page_token) = state?;
                    let mut request = client
                        .http_client
                        .get(client.api_url("users/me/messages").ok()?)
                        .query(&[("q", query.as_str()), ("maxResults", &batch_size.to_string())]);

                    if let Some(token) = page_token {
                        request = request.query(&[("pageToken", token)]);
                    }

                    let response = client.execute_with_retry(request).await.ok()?;
                    let search_response = response.json::<SearchResponse>().await.ok()?;

                    if search_response.messages.is_empty() {
                        return None;
                    }

                    let next_state = Some((
                        query.clone(),
                        batch_size,
                        search_response.next_page_token,
                    ));

                    Some((Ok(search_response.messages), next_state))
                }
            },
        )
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
                let backoff = self.config.performance.retry_backoff_ms * (2_u64.pow(attempt));
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

    #[allow(clippy::expect_used)]
    builder.build().expect("Failed to build HTTP client")
}
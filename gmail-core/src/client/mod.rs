//! High-performance Gmail API client with HTTP/3, connection pooling, and retry logic

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client as ReqwestClient;
use reqwest::header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

use crate::auth::GmailAuth;
use crate::config::{GmailConfig, PerformanceConfig};
use crate::error::{GmailError, Result};
use crate::models::*;
use crate::runtime::{RuntimeFeatures, detect_runtime_features};

mod drafts;
mod history;
mod labels;
mod messages;
mod settings;
mod threads;
mod watch;

mod streaming;

pub use streaming::*;

const DEFAULT_BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1/";
const DEFAULT_UPLOAD_BASE_URL: &str = "https://gmail.googleapis.com/upload/gmail/v1/";

/// Transport protocol selection resolved from config and compile-time features.
///
/// Prior-knowledge modes are mutually exclusive: reqwest rejects a client
/// configured with both HTTP/3 and HTTP/2 prior knowledge.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransportMode {
    Http3PriorKnowledge,
    Http2PriorKnowledge,
    AlpnDefault,
}

/// Resolve which transport mode the HTTP client should use.
///
/// HTTP/3 wins when requested and the `http3` feature is compiled in;
/// otherwise HTTP/2 prior knowledge applies when enabled; otherwise ALPN
/// negotiation defaults apply.
pub fn resolve_transport_mode(
    enable_http3: bool,
    http3_feature: bool,
    enable_http2: bool,
) -> TransportMode {
    if enable_http3 && http3_feature {
        TransportMode::Http3PriorKnowledge
    } else if enable_http2 {
        TransportMode::Http2PriorKnowledge
    } else {
        TransportMode::AlpnDefault
    }
}

/// Observed transport negotiation details captured during client construction.
#[derive(Debug, Clone, Default)]
pub struct TransportInfo {
    pub negotiated_version: String,
    pub http3_requested: bool,
    pub http3_effective: bool,
    pub fell_back: bool,
}

/// Gmail API client builder
pub struct GmailClientBuilder {
    config: GmailConfig,
    auth: Option<GmailAuth>,
    http_client: Option<ReqwestClient>,
    base_url: Option<Url>,
    upload_base_url: Option<Url>,
}

impl GmailClientBuilder {
    pub fn new(config: GmailConfig) -> Self {
        Self {
            config,
            auth: None,
            http_client: None,
            base_url: None,
            upload_base_url: None,
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

    /// Override the Gmail API base URL (primarily for test injection).
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Override the Gmail media-upload base URL (primarily for test injection).
    pub fn upload_base_url(mut self, url: Url) -> Self {
        self.upload_base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<GmailClient> {
        let auth = self
            .auth
            .ok_or_else(|| GmailError::Config("Auth is required".into()))?;
        let base_url = match self.base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GmailError::Config(format!("Invalid base URL: {}", e)))?,
        };
        let upload_base_url = match self.upload_base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_UPLOAD_BASE_URL)
                .map_err(|e| GmailError::Config(format!("Invalid upload base URL: {}", e)))?,
        };

        let requested = self.config.performance.enable_http3 && cfg!(feature = "http3");
        let mut info = TransportInfo {
            http3_requested: requested,
            http3_effective: requested,
            ..Default::default()
        };
        let mut http_client = self
            .http_client
            .unwrap_or_else(|| build_http_client(&self.config.performance));

        if requested {
            match probe(&http_client, &base_url, &auth, reqwest::Version::HTTP_3).await {
                Ok(v) => info.negotiated_version = v,
                Err(e) => {
                    warn!("HTTP/3 probe failed ({e}); rebuilding without h3");
                    let mut perf_no_h3 = self.config.performance.clone();
                    perf_no_h3.enable_http3 = false;
                    http_client = build_http_client(&perf_no_h3);
                    info.negotiated_version =
                        probe(&http_client, &base_url, &auth, reqwest::Version::HTTP_2).await?;
                    info.http3_effective = false;
                    info.fell_back = true;
                }
            }
        } else {
            info.negotiated_version = "not-probed".into();
        }

        GmailClient::new(
            auth,
            http_client,
            self.config,
            base_url,
            upload_base_url,
            info,
        )
        .await
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
    upload_base_url: Url,
    transport_info: TransportInfo,
    #[allow(dead_code)]
    runtime_features: RuntimeFeatures,
}

impl GmailClient {
    /// Create new client
    pub async fn new(
        auth: GmailAuth,
        http_client: ReqwestClient,
        config: GmailConfig,
        base_url: Url,
        upload_base_url: Url,
        transport_info: TransportInfo,
    ) -> Result<Self> {
        let runtime_features = detect_runtime_features();

        let max_concurrent = config.performance.max_concurrent;
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        info!(
            "GmailClient initialized: http3={}, io_uring={}, concurrency={}",
            runtime_features.http3, runtime_features.io_uring, max_concurrent
        );

        Ok(Self {
            auth: Arc::new(auth),
            http_client,
            config,
            semaphore,
            base_url,
            upload_base_url,
            transport_info,
            runtime_features,
        })
    }

    /// Get access token
    pub async fn access_token(&self) -> Result<String> {
        self.auth.get_access_token().await
    }

    /// Transport negotiation details observed during client construction.
    pub fn transport_info(&self) -> &TransportInfo {
        &self.transport_info
    }

    /// Performance configuration this client was built with.
    pub fn performance_config(&self) -> &PerformanceConfig {
        &self.config.performance
    }

    /// Force re-authentication by invalidating token
    pub async fn force_refresh(&self) -> Result<()> {
        self.auth.force_refresh().await
    }

    /// Build API URL with proper error handling
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GmailError::Config(format!("Invalid API URL: {}", e)))
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

        unfold(Some((query, batch_size, None::<String>)), move |state| {
            let client = client.clone();
            async move {
                let (query, batch_size, page_token) = state?;
                let mut request = client
                    .http_client
                    .get(client.api_url("users/me/messages").ok()?)
                    .query(&[
                        ("q", query.as_str()),
                        ("maxResults", &batch_size.to_string()),
                    ]);

                if let Some(token) = page_token {
                    request = request.query(&[("pageToken", token)]);
                }

                let response = client.execute_with_retry(request).await.ok()?;
                let search_response = response.json::<SearchResponse>().await.ok()?;

                if search_response.messages.is_empty() {
                    return None;
                }

                let next_state = Some((query.clone(), batch_size, search_response.next_page_token));

                Some((Ok(search_response.messages), next_state))
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    // Internal: Execute with retry, rate limiting, and concurrency control
    // ═════════════════════════════════════════════════════════════════

    async fn execute_with_retry(
        &self,
        mut request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        // Acquire semaphore for concurrency control
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| GmailError::Internal("Semaphore closed".into()))?;

        // Add auth header
        let token = self.auth.get_access_token().await?;
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));
        request = apply_transport_version(request, self.transport_info.http3_effective);

        let mut pending = Some(request);
        let mut last_error = None;

        for attempt in 0..=self.config.performance.retry_attempts {
            // Streaming bodies are non-replayable: try_clone() yields None and
            // only one send attempt is possible, so the builder is consumed.
            let to_send = match pending.take() {
                None => break,
                Some(builder) => match builder.try_clone() {
                    Some(replayable) => {
                        pending = Some(builder);
                        replayable
                    }
                    None => builder,
                },
            };

            let response = to_send.send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        return Ok(resp);
                    }

                    // Handle specific error codes
                    match status.as_u16() {
                        401 => {
                            // The bearer token above was fetched once before
                            // this loop, so retrying re-sends the same stale
                            // credential and can never succeed — and clearing
                            // storage would drop callers into the implicit
                            // OAuth flow mid-request. Fail fast instead.
                            return Err(GmailError::Auth(
                                format!(
                                    "request rejected as unauthorized (401); \
                                     your access token is expired or invalid — \
                                     rerun `gmail auth login`"
                                )
                                .into(),
                            ));
                        }
                        429 => {
                            // Rate limited - extract retry-after header
                            let retry_after = resp
                                .headers()
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(60);
                            return Err(GmailError::RateLimited {
                                retry_after_secs: retry_after,
                            });
                        }
                        403 => {
                            return Err(GmailError::PermissionDenied(
                                "Insufficient permissions".into(),
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
                debug!(
                    "Request failed, retrying in {:?} (attempt {}/{})",
                    backoff,
                    attempt + 1,
                    self.config.performance.retry_attempts
                );
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

/// Apply the transport-level HTTP version override for data-plane requests.
///
/// reqwest only routes an individual request over HTTP/3 when the request
/// itself carries `version(HTTP_3)`; client-level `http3_prior_knowledge()`
/// merely builds the QUIC connector. When h3 is effective, every API request
/// must be explicitly versioned or it silently rides TCP.
pub fn apply_transport_version(
    builder: reqwest::RequestBuilder,
    http3_effective: bool,
) -> reqwest::RequestBuilder {
    if http3_effective {
        builder.version(reqwest::Version::HTTP_3)
    } else {
        builder
    }
}

/// Probe the negotiated HTTP version with a real authenticated request.
async fn probe(
    client: &ReqwestClient,
    base: &Url,
    auth: &GmailAuth,
    version: reqwest::Version,
) -> Result<String> {
    let token = auth.get_access_token().await?;
    let url = base.join("users/me/profile")?;
    let resp = client
        .get(url)
        .bearer_auth(&token)
        .version(version)
        .send()
        .await?
        .error_for_status()?;
    Ok(format!("{:?}", resp.version()))
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

    // Transport protocol: prior-knowledge modes are mutually exclusive.
    match resolve_transport_mode(
        perf.enable_http3,
        cfg!(feature = "http3"),
        perf.enable_http2,
    ) {
        TransportMode::Http3PriorKnowledge => {
            #[cfg(feature = "http3")]
            {
                builder = builder.http3_prior_knowledge();
            }
        }
        TransportMode::Http2PriorKnowledge => {
            builder = builder.http2_prior_knowledge();
        }
        TransportMode::AlpnDefault => {}
    }

    // Default headers
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static("zstd, br, gzip, deflate"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    builder = builder.default_headers(headers);

    #[allow(clippy::expect_used)]
    builder.build().expect("Failed to build HTTP client")
}

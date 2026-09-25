//! Service-agnostic HTTP core: one authenticated transport for every
//! Google API client (Gmail, Calendar, Drive, Contacts, Chat, Forms).
//!
//! Zero-config by construction: all tuning is compile-time constants.
//! tokio sizes OS threads to cores; the semaphore bounds only in-flight
//! HTTP requests so bursts stay under Google's per-user rate limits.

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client as ReqwestClient;
use reqwest::header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

use crate::core::auth::GoogleAuth;
use crate::core::error::{GrrError, Result};

// ── Auto-tuned transport constants ────────────────────────────────────────

/// Hard ceiling on in-flight HTTP requests. Google's per-user rate limits
/// (not CPU) are the scarce resource; exceeding this burns quota into 429s.
pub(crate) const MAX_INFLIGHT_REQUESTS: usize = 64;
/// Whole-request timeout (includes server processing time).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// TCP+TLS establishment only.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Idle keepalive connections per host for instant reuse.
const POOL_IDLE_PER_HOST: usize = 32;
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const TCP_KEEPALIVE: Duration = Duration::from_secs(60);
const H2_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
/// Transient-failure retries (connect/timeout/5xx/429).
const RETRY_ATTEMPTS: u32 = 3;
/// Exponential backoff base; grows 100ms -> 200ms -> 400ms.
const RETRY_BACKOFF_MS: u64 = 100;
/// Never sleep longer than this honoring a 429 Retry-After mid-command.
const MAX_RATE_LIMIT_WAIT: Duration = Duration::from_secs(30);

/// Whether this build of grr-core has HTTP/3 compiled in. The crate's
/// own `cfg` is invisible to dependents; this const makes the state
/// observable for `grr transport` and tests.
pub const HTTP3_COMPILED: bool = cfg!(feature = "http3");

/// Transport protocol selection resolved from compile-time features.
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
/// HTTP/3 wins when the `http3` feature is compiled in; otherwise HTTP/2
/// prior knowledge applies; otherwise ALPN negotiation defaults apply.
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

/// Observed transport negotiation details captured during client
/// construction.
#[derive(Debug, Clone, Default)]
pub struct TransportInfo {
    pub negotiated_version: String,
    pub http3_requested: bool,
    pub http3_effective: bool,
    pub fell_back: bool,
}

/// The shared authenticated HTTP engine behind every service client.
///
/// Owns the reqwest client, the in-flight request valve, and the retry
/// executor (429 Retry-After, transient 5xx, fast-fail 401/403/404).
/// Service clients build URLs from their own base and hand requests to
/// [`HttpCore::execute`].
#[derive(Clone)]
pub struct HttpCore {
    auth: Arc<GoogleAuth>,
    http_client: ReqwestClient,
    semaphore: Arc<Semaphore>,
    transport_info: TransportInfo,
}

impl HttpCore {
    /// Build the core, probing the negotiated HTTP version against the
    /// service base URL (any authenticated probe path works; a dead
    /// stored token must not brick construction — `grr auth login`
    /// recovers without one).
    pub async fn connect(auth: GoogleAuth, base_url: &Url, probe_path: &str) -> Self {
        let requested = cfg!(feature = "http3");
        let mut info = TransportInfo {
            http3_requested: requested,
            http3_effective: requested,
            ..Default::default()
        };
        let mut http_client = build_http_client();

        if requested {
            match probe(
                &http_client,
                base_url,
                &auth,
                reqwest::Version::HTTP_3,
                probe_path,
            )
            .await
            {
                Ok(v) => info.negotiated_version = v,
                Err(e) => {
                    warn!("HTTP/3 probe failed ({e}); rebuilding without h3");
                    http_client = build_http_client();
                    match probe(
                        &http_client,
                        base_url,
                        &auth,
                        reqwest::Version::HTTP_2,
                        probe_path,
                    )
                    .await
                    {
                        Ok(v) => {
                            info.negotiated_version = v;
                            info.http3_effective = false;
                            info.fell_back = true;
                        }
                        Err(e) => {
                            // Offline machine or dead token: continue so
                            // `grr auth login` can still run.
                            warn!("HTTP/2 probe failed ({e}); continuing unprobed");
                            info.negotiated_version = "not-probed".into();
                            info.http3_effective = false;
                        }
                    }
                }
            }
        } else {
            info.negotiated_version = "not-probed".into();
        }

        info!(
            "HttpCore connected (probe {}: {}): http3_effective={}, max_inflight_requests={}",
            probe_path, info.negotiated_version, info.http3_effective, MAX_INFLIGHT_REQUESTS
        );

        Self {
            auth: Arc::new(auth),
            http_client,
            semaphore: Arc::new(Semaphore::new(MAX_INFLIGHT_REQUESTS)),
            transport_info: info,
        }
    }

    /// Core assembled without probing (test injection: custom client, mock
    /// base URLs).
    pub fn unprobed(auth: GoogleAuth, http_client: ReqwestClient) -> Self {
        Self {
            auth: Arc::new(auth),
            http_client,
            semaphore: Arc::new(Semaphore::new(MAX_INFLIGHT_REQUESTS)),
            transport_info: TransportInfo {
                negotiated_version: "not-probed".into(),
                http3_requested: HTTP3_COMPILED,
                http3_effective: false,
                fell_back: false,
            },
        }
    }

    /// Start a GET request against an absolute URL.
    pub fn get(&self, url: Url) -> reqwest::RequestBuilder {
        self.http_client.get(url)
    }

    /// Start a POST request against an absolute URL.
    pub fn post(&self, url: Url) -> reqwest::RequestBuilder {
        self.http_client.post(url)
    }

    /// Start a PUT request against an absolute URL.
    pub fn put(&self, url: Url) -> reqwest::RequestBuilder {
        self.http_client.put(url)
    }

    /// Start a PATCH request against an absolute URL.
    pub fn patch(&self, url: Url) -> reqwest::RequestBuilder {
        self.http_client.patch(url)
    }

    /// Start a DELETE request against an absolute URL.
    pub fn delete(&self, url: Url) -> reqwest::RequestBuilder {
        self.http_client.delete(url)
    }

    /// The underlying reqwest client (multipart uploads, raw control).
    pub fn http_client(&self) -> &ReqwestClient {
        &self.http_client
    }

    /// Shared Google OAuth handle (login, device flow, token backend).
    pub fn auth(&self) -> &GoogleAuth {
        &self.auth
    }

    /// Transport negotiation details observed during connection.
    pub fn transport_info(&self) -> &TransportInfo {
        &self.transport_info
    }

    /// Execute a request: bearer auth, HTTP/3 version pinning, in-flight
    /// valve, and the retry policy (429 Retry-After honored up to
    /// [`MAX_RATE_LIMIT_WAIT`]; 401 fails fast with a `grr auth login`
    /// hint; transient errors back off exponentially).
    pub async fn execute(&self, mut request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| GrrError::Internal("Semaphore closed".into()))?;

        let token = self.auth.get_access_token().await?;
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));
        request = apply_transport_version(request, self.transport_info.http3_effective);

        let mut pending = Some(request);
        let mut last_error = None;

        for attempt in 0..=RETRY_ATTEMPTS {
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

                    match status.as_u16() {
                        401 => {
                            // The bearer token above was fetched once before
                            // this loop, so retrying re-sends the same stale
                            // credential and can never succeed — and clearing
                            // storage would drop callers into the implicit
                            // OAuth flow mid-request. Fail fast instead.
                            return Err(GrrError::Auth(
                                "request rejected as unauthorized (401); \
                                 your access token is expired or invalid — \
                                 rerun `grr auth login`"
                                    .into(),
                            ));
                        }
                        429 => {
                            // Rate limited. Honor the server's Retry-After and
                            // keep going when the wait is short; surface the
                            // wait immediately when it is not.
                            let retry_after = resp
                                .headers()
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(60);
                            let wait = Duration::from_secs(retry_after);
                            if attempt < RETRY_ATTEMPTS && wait <= MAX_RATE_LIMIT_WAIT {
                                warn!(
                                    "rate limited; honoring Retry-After of {retry_after}s \
                                     (attempt {}/{})",
                                    attempt + 1,
                                    RETRY_ATTEMPTS
                                );
                                tokio::time::sleep(wait).await;
                                continue;
                            }
                            return Err(GrrError::RateLimited {
                                retry_after_secs: retry_after,
                            });
                        }
                        403 => {
                            return Err(GrrError::PermissionDenied(
                                "Insufficient permissions for this API or scope \
                                 (was the API enabled in Google Cloud, and does \
                                 your login include its scope? rerun `grr auth login` \
                                 to grant new scopes)"
                                    .into(),
                            ));
                        }
                        404 => {
                            return Err(GrrError::NotFound("Resource not found".into()));
                        }
                        500..=599 => {
                            last_error = Some(GrrError::Api {
                                status: status.as_u16(),
                                message: "Server error".into(),
                            });
                        }
                        _ => {
                            let error_text = resp.text().await.unwrap_or_default();
                            return Err(GrrError::Api {
                                status: status.as_u16(),
                                message: error_text,
                            });
                        }
                    }
                }
                Err(e) => {
                    if e.is_timeout() || e.is_connect() || e.is_request() {
                        last_error = Some(GrrError::Http(e));
                    } else {
                        return Err(GrrError::Http(e));
                    }
                }
            }

            if attempt < RETRY_ATTEMPTS {
                let backoff = RETRY_BACKOFF_MS * (2_u64.pow(attempt));
                debug!(
                    "Request failed, retrying in {:?} (attempt {}/{})",
                    backoff,
                    attempt + 1,
                    RETRY_ATTEMPTS
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| GrrError::Internal("Max retries exceeded".into())))
    }
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
///
/// Any HTTP response — even a 404/401 — proves the transport negotiated
/// (some services, e.g. Forms, have no cheap "list" endpoint to hit).
/// Only transport-level failures (connect/timeout) fail the probe.
async fn probe(
    client: &ReqwestClient,
    base: &Url,
    auth: &GoogleAuth,
    version: reqwest::Version,
    probe_path: &str,
) -> std::result::Result<String, GrrError> {
    let token = auth.get_access_token().await?;
    let url = base.join(probe_path)?;
    let resp = client
        .get(url)
        .bearer_auth(&token)
        .version(version)
        .send()
        .await?;
    Ok(format!("{:?}", resp.version()))
}

/// Build HTTP client with auto-tuned transport settings.
///
/// Zero-config: every value is a constant tuned for Google frontends and
/// bounded async I/O; the tokio runtime already sizes OS threads to cores.
pub fn build_http_client() -> ReqwestClient {
    let mut builder = ReqwestClient::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_max_idle_per_host(POOL_IDLE_PER_HOST)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .tcp_keepalive(TCP_KEEPALIVE)
        .http2_keep_alive_interval(H2_KEEPALIVE_INTERVAL)
        .http2_adaptive_window(true)
        // Always on: Google frontends send gzip regardless of toggles, and
        // advertising an encoding without decoding it hands raw compressed
        // bytes to serde ("expected value at line 1 column 1").
        .brotli(true)
        .zstd(true)
        .gzip(true);

    match resolve_transport_mode(cfg!(feature = "http3"), cfg!(feature = "http3"), true) {
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

//! Configuration types for gmail-core

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration for Gmail client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GmailConfig {
    #[serde(default)]
    pub oauth: OAuthConfig,
    
    #[serde(default)]
    pub performance: PerformanceConfig,
    
    #[serde(default)]
    pub output: OutputConfig,
    
    #[serde(default)]
    pub runtime: RuntimeConfig,
    
    #[serde(default)]
    pub cache: CacheConfig,
}

/// OAuth2 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    
    #[serde(default = "default_redirect_uri")]
    pub redirect_uri: String,
    
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    
    #[serde(default)]
    pub use_pkce: bool,
}

fn default_redirect_uri() -> String {
    "http://localhost:3434/oauth/callback".to_string()
}

fn default_scopes() -> Vec<String> {
    vec![
        "https://www.googleapis.com/auth/gmail.readonly".to_string(),
        "https://www.googleapis.com/auth/gmail.compose".to_string(),
        #[cfg(feature = "http3")]
        "https://www.googleapis.com/auth/gmail.modify".to_string(),
        "https://www.googleapis.com/auth/gmail.labels".to_string(),
        "https://mail.google.com/".to_string(),
    ]
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: default_redirect_uri(),
            scopes: default_scopes(),
            use_pkce: true,
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PerformanceConfig {
    /// Maximum concurrent requests
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    
    /// Connection pool size per host
    #[serde(default = "default_pool_size")]
    pub connection_pool_size: usize,
    
    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,
    
    /// Connect timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,
    
    /// Retry attempts
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    
    /// Retry backoff base milliseconds
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff_ms: u64,
    
    /// Enable HTTP/3 (requires unstable feature)
    #[serde(default)]
    pub enable_http3: bool,
    
    /// Enable HTTP/2
    #[serde(default = "default_true")]
    pub enable_http2: bool,
    
    /// Enable zstd compression
    #[serde(default = "default_true")]
    pub enable_zstd: bool,
    
    /// Enable brotli compression
    #[serde(default = "default_true")]
    pub enable_brotli: bool,
    
    /// Enable io_uring on Linux (auto-detected if not set)
    #[serde(default)]
    pub enable_io_uring: Option<bool>,
    
    /// TCP keepalive interval seconds
    #[serde(default = "default_keepalive")]
    pub tcp_keepalive_secs: u64,
    
    /// HTTP/2 adaptive window
    #[serde(default = "default_true")]
    pub http2_adaptive_window: bool,
    
    /// HTTP/2 keepalive interval seconds
    #[serde(default = "default_http2_keepalive")]
    pub http2_keepalive_interval_secs: u64,
}

fn default_max_concurrent() -> usize { 20 }
fn default_pool_size() -> usize { 50 }
fn default_request_timeout() -> u64 { 30 }
fn default_connect_timeout() -> u64 { 5 }
fn default_retry_attempts() -> u32 { 3 }
fn default_retry_backoff() -> u64 { 100 }
fn default_true() -> bool { true }
fn default_keepalive() -> u64 { 30 }
fn default_http2_keepalive() -> u64 { 30 }

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent: default_max_concurrent(),
            connection_pool_size: default_pool_size(),
            request_timeout_secs: default_request_timeout(),
            connect_timeout_secs: default_connect_timeout(),
            retry_attempts: default_retry_attempts(),
            retry_backoff_ms: default_retry_backoff(),
            enable_http3: cfg!(feature = "http3"),
            enable_http2: default_true(),
            enable_zstd: default_true(),
            enable_brotli: default_true(),
            enable_io_uring: None, // auto-detect
            tcp_keepalive_secs: default_keepalive(),
            http2_adaptive_window: default_true(),
            http2_keepalive_interval_secs: default_http2_keepalive(),
        }
    }
}

/// Output formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OutputConfig {
    /// Default output format: json, jsonl, table, pretty
    #[serde(default = "default_format")]
    pub default_format: String,
    
    /// Colorize output
    #[serde(default = "default_true")]
    pub color: bool,
    
    /// Pretty print JSON
    #[serde(default)]
    pub pretty: bool,
    
    /// Include null fields in output
    #[serde(default)]
    pub include_nulls: bool,
}

fn default_format() -> String { "json".to_string() }

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            default_format: default_format(),
            color: default_true(),
            pretty: false,
            include_nulls: false,
        }
    }
}

/// Runtime feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuntimeConfig {
    /// Enable metrics collection
    #[serde(default)]
    pub enable_metrics: bool,
    
    /// Metrics port (if enabled)
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    
    /// Enable tracing
    #[serde(default = "default_true")]
    pub enable_tracing: bool,
    
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_metrics_port() -> u16 { 9090 }
fn default_log_level() -> String { "info".to_string() }

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            enable_metrics: false,
            metrics_port: default_metrics_port(),
            enable_tracing: default_true(),
            log_level: default_log_level(),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CacheConfig {
    /// Cache directory
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    
    /// Enable response caching (ETag-based)
    #[serde(default = "default_true")]
    pub enable_etag_cache: bool,
    
    /// Max cache size in MB
    #[serde(default = "default_cache_size")]
    pub max_cache_size_mb: u64,
    
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gmail-opencode")
}

fn default_cache_size() -> u64 { 100 }
fn default_cache_ttl() -> u64 { 3600 }

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_dir(),
            enable_etag_cache: default_true(),
            max_cache_size_mb: default_cache_size(),
            cache_ttl_secs: default_cache_ttl(),
        }
    }
}

impl Default for GmailConfig {
    fn default() -> Self {
        Self {
            oauth: OAuthConfig::default(),
            performance: PerformanceConfig::default(),
            output: OutputConfig::default(),
            runtime: RuntimeConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

impl PerformanceConfig {
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }
    
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs)
    }
    
    pub fn retry_backoff(&self) -> Duration {
        Duration::from_millis(self.retry_backoff_ms)
    }
    
    pub fn tcp_keepalive(&self) -> Duration {
        Duration::from_secs(self.tcp_keepalive_secs)
    }
    
    pub fn http2_keepalive_interval(&self) -> Duration {
        Duration::from_secs(self.http2_keepalive_interval_secs)
    }
}
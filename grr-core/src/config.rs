//! Configuration types for grr-core.
//!
//! Zero-config by design: the only thing a user must supply is OAuth
//! credentials. Every performance knob (concurrency, pooling, timeouts,
//! compression, transport) is auto-tuned at runtime from machine
//! capabilities — see [`crate::client`].

use serde::{Deserialize, Serialize};

/// Main configuration for the client
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct GmailConfig {
    #[serde(default)]
    pub oauth: OAuthConfig,
}

/// OAuth2 configuration.
///
/// Accepts both kebab-case (Rust-native) and snake_case (TypeScript-era
/// `config.toml` migration) key spellings. Redirect URI and scopes are
/// compile-time constants — there is nothing to tune here on purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub struct OAuthConfig {
    #[serde(alias = "client_id")]
    pub client_id: String,

    /// Optional client secret. Sent at the token endpoint only when
    /// present and non-empty. PKCE-only providers (no secret issued)
    /// work with this absent; providers that mandate a secret
    /// (currently including Google, even for Desktop clients) need it.
    #[serde(default, alias = "client_secret")]
    pub client_secret: Option<String>,
}

#[cfg(test)]
mod config_compat_tests {
    use super::*;
    use figment::{
        Figment,
        providers::{Format, Toml},
    };

    #[test]
    fn oauth_accepts_snake_case_keys_from_ts_era_configs() {
        let toml = r#"
[oauth]
client_id = "id-123"
client_secret = "secret-456"
"#;
        let config = Figment::new()
            .merge(Toml::string(toml))
            .extract::<GmailConfig>()
            .unwrap_or_else(|_| GmailConfig::default());
        assert_eq!(config.oauth.client_id, "id-123");
        assert_eq!(config.oauth.client_secret.as_deref(), Some("secret-456"));
    }

    #[test]
    fn oauth_accepts_kebab_case_keys() {
        let toml = r#"
[oauth]
client-id = "id-123"
client-secret = "secret-456"
"#;
        let config = Figment::new()
            .merge(Toml::string(toml))
            .extract::<GmailConfig>()
            .unwrap_or_else(|_| GmailConfig::default());
        assert_eq!(config.oauth.client_id, "id-123");
    }

    #[test]
    fn legacy_config_sections_are_ignored() {
        // Pre-zero-config configs carry [performance]/[output]/[runtime]
        // sections; they must still parse, just ignored.
        let toml = r#"
[oauth]
client-id = "id-123"
[performance]
max-concurrent = 99
[cache]
cache-dir = "/tmp/x"
"#;
        let config = Figment::new()
            .merge(Toml::string(toml))
            .extract::<GmailConfig>()
            .unwrap_or_else(|_| GmailConfig::default());
        assert_eq!(config.oauth.client_id, "id-123");
    }
}

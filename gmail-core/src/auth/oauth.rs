//! OAuth authorization flow with PKCE

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use urlencoding;

use crate::error::{GmailError, Result};

use super::TokenStorage;

impl super::GmailAuth {
    /// Run full OAuth2 flow with PKCE
    pub(crate) async fn run_oauth_flow(&self) -> Result<TokenStorage> {
        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = if self.config.use_pkce {
            let mut verifier_bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut verifier_bytes);
            let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);
            let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(Sha256::digest(verifier.as_bytes()).as_slice());
            (Some(challenge), Some(verifier))
        } else {
            (None, None)
        };

        // Generate authorization URL
        let mut auth_url = "https://accounts.google.com/o/oauth2/v2/auth".to_string();
        auth_url.push_str("?response_type=code");
        auth_url.push_str(&format!(
            "&client_id={}",
            urlencoding::encode(&self.config.client_id)
        ));
        auth_url.push_str(&format!(
            "&redirect_uri={}",
            urlencoding::encode(&self.config.redirect_uri)
        ));
        auth_url.push_str(&format!(
            "&scope={}",
            urlencoding::encode(&self.config.scopes.join(" "))
        ));
        auth_url.push_str("&access_type=offline");
        auth_url.push_str("&prompt=consent");

        if let Some(challenge) = &pkce_challenge {
            auth_url.push_str(&format!("&code_challenge={}", challenge));
            auth_url.push_str("&code_challenge_method=S256");
        }

        info!("Opening browser for authentication: {}", auth_url);

        // Open browser
        if let Err(e) = open::that(auth_url.as_str()) {
            warn!(
                "Failed to open browser: {}. Please manually open: {}",
                e, auth_url
            );
        }

        // Start local server to receive callback
        let (code, _state) = self.start_callback_server().await?;

        // Exchange code for tokens
        let mut form = vec![
            ("code", code),
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("redirect_uri", self.config.redirect_uri.clone()),
            ("grant_type", "authorization_code".to_string()),
        ];

        if let Some(verifier) = pkce_verifier {
            form.push(("code_verifier", verifier));
        }

        let response = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&form)
            .send()
            .await
            .map_err(GmailError::Http)?;

        let token_data: serde_json::Value = response.json().await.map_err(GmailError::Http)?;

        let access_token = token_data["access_token"]
            .as_str()
            .ok_or_else(|| GmailError::Auth(anyhow!("No access token in response").into()))?
            .to_string();

        let expires_in = token_data["expires_in"].as_u64().unwrap_or(3600);

        let refresh_token = token_data["refresh_token"].as_str().map(|s| s.to_string());

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            + expires_in;

        Ok(TokenStorage {
            access_token,
            refresh_token,
            expires_at,
            token_type: "Bearer".to_string(),
            scope: self.config.scopes.join(" "),
        })
    }
}

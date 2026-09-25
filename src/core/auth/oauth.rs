//! OAuth authorization flow with PKCE (RFC 7636), always on.

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use url::Url;

use crate::core::error::{GrrError, Result};

use super::{REDIRECT_URI, SCOPES, TokenStorage};

impl super::GoogleAuth {
    /// Run full OAuth2 flow with PKCE
    pub(crate) async fn run_oauth_flow(&self) -> Result<TokenStorage> {
        let mut verifier_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);
        let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(verifier.as_bytes()).as_slice());

        let mut state_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut state_bytes);
        let state = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(state_bytes);

        let mut auth_url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;
        auth_url
            .query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("scope", &super::scopes_joined())
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .finish();

        info!("Opening browser for authentication");

        let browser_url = auth_url.to_string();
        let launch_url = browser_url.clone();
        match tokio::task::spawn_blocking(move || open::that(launch_url)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!("Failed to open browser: {e}. Please manually open: {browser_url}"),
            Err(e) => warn!("Browser launcher failed: {e}. Please manually open: {browser_url}"),
        }

        let (code, returned_state) = self.start_callback_server().await?;
        if returned_state != state {
            return Err(GrrError::Auth(
                anyhow::anyhow!("OAuth callback state did not match request").into(),
            ));
        }

        let mut form = vec![
            ("code", code),
            ("client_id", self.config.client_id.clone()),
            ("redirect_uri", REDIRECT_URI.to_string()),
            ("grant_type", "authorization_code".to_string()),
        ];
        if let Some(secret) = self.config.client_secret.clone().filter(|s| !s.is_empty()) {
            form.push(("client_secret", secret));
        }
        form.push(("code_verifier", verifier));

        let response = self
            .http_client
            .post(self.token_endpoint.as_str())
            .form(&form)
            .send()
            .await
            .map_err(GrrError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(GrrError::Http)?;
        let token_data: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            GrrError::Auth(
                anyhow::anyhow!("token endpoint returned {status}: unparseable body ({e})").into(),
            )
        })?;

        if !status.is_success() {
            return Err(GrrError::Auth(
                anyhow::anyhow!(
                    "token endpoint returned {status}: {}",
                    crate::core::error::json_error_detail(&token_data)
                )
                .into(),
            ));
        }

        super::device::token_storage_from_response(&token_data, SCOPES)
    }
}

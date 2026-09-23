//! OAuth authorization flow with PKCE (RFC 7636), always on.

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tracing::info;
use tracing::warn;
use urlencoding;

use crate::error::{GrrError, Result};

use super::{REDIRECT_URI, SCOPES, TokenStorage};

impl super::GoogleAuth {
    /// Run full OAuth2 flow with PKCE
    pub(crate) async fn run_oauth_flow(&self) -> Result<TokenStorage> {
        // PKCE is unconditional: the verifier is the client identity proof.
        let mut verifier_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);
        let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(verifier.as_bytes()).as_slice());

        // Generate authorization URL
        let mut auth_url = "https://accounts.google.com/o/oauth2/v2/auth".to_string();
        auth_url.push_str("?response_type=code");
        auth_url.push_str(&format!(
            "&client_id={}",
            urlencoding::encode(&self.config.client_id)
        ));
        auth_url.push_str("&redirect_uri=");
        auth_url.push_str(&urlencoding::encode(REDIRECT_URI));
        auth_url.push_str("&scope=");
        auth_url.push_str(&urlencoding::encode(&super::scopes_joined()));
        auth_url.push_str("&access_type=offline");
        auth_url.push_str("&prompt=consent");
        auth_url.push_str(&format!("&code_challenge={challenge}"));
        auth_url.push_str("&code_challenge_method=S256");

        info!("Opening browser for authentication");

        // Open browser
        if let Err(e) = open::that(auth_url.as_str()) {
            warn!(
                "Failed to open browser: {}. Please manually open: {}",
                e, auth_url
            );
        }

        // Start local server to receive callback
        let (code, _state) = self.start_callback_server().await?;

        // The secret goes only to providers that mandate it (Google does,
        // even for Desktop clients); PKCE-only providers get none.
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
            .post("https://oauth2.googleapis.com/token")
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
                    super::device::error_detail(&token_data)
                )
                .into(),
            ));
        }

        super::device::token_storage_from_response(&token_data, SCOPES)
    }
}

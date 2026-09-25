//! OAuth 2.0 Device Authorization Grant (RFC 8628).
//!
//! Headless fallback for the default PKCE loopback flow: the CLI shows a
//! URL plus a user code, the user approves in any browser (on any device),
//! and the CLI polls the token endpoint until approval lands.
//!
//! A configured client secret is sent when present; secretless PKCE
//! providers work with it absent.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::anyhow;

use crate::core::error::{GrrError, Result, json_error_detail};

use super::TokenStorage;

const DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Pending device authorization: show `verification_url` + `user_code` to
/// the user, then poll with [`super::GoogleAuth::poll_device_code`] until done.
pub struct DeviceAuthChallenge {
    /// URL the user must open (e.g. <https://www.google.com/device>).
    pub verification_url: String,
    /// Short code the user types at `verification_url`.
    pub user_code: String,
    pub(crate) device_code: String,
    pub(crate) interval: Duration,
    pub(crate) deadline: Instant,
}

impl DeviceAuthChallenge {
    /// How long to wait before the next poll (grows on `slow_down`).
    pub fn retry_after(&self) -> Duration {
        self.interval
    }

    /// Whether the user code has expired and the flow must restart.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

impl super::GoogleAuth {
    /// Start a device flow: returns the challenge to display to the user.
    pub async fn request_device_code(&self) -> Result<DeviceAuthChallenge> {
        let mut form = vec![
            ("client_id", self.config.client_id.clone()),
            ("scope", super::scopes_joined()),
        ];
        if let Some(secret) = self.config.client_secret.clone().filter(|s| !s.is_empty()) {
            form.push(("client_secret", secret));
        }

        let response = self
            .http_client
            .post(DEVICE_CODE_URL)
            .form(&form)
            .send()
            .await
            .map_err(GrrError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(GrrError::Http)?;
        let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            GrrError::Auth(
                anyhow!("device endpoint returned {status}: unparseable body ({e})").into(),
            )
        })?;

        if !status.is_success() {
            return Err(GrrError::Auth(
                anyhow!(
                    "device endpoint returned {status}: {}",
                    json_error_detail(&body)
                )
                .into(),
            ));
        }

        let device_code = body["device_code"]
            .as_str()
            .ok_or_else(|| GrrError::Auth(anyhow!("device endpoint omitted device_code").into()))?;
        let user_code = body["user_code"]
            .as_str()
            .ok_or_else(|| GrrError::Auth(anyhow!("device endpoint omitted user_code").into()))?;
        let verification_url = body
            .get("verification_url")
            .or_else(|| body.get("verification_uri"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                GrrError::Auth(anyhow!("device endpoint omitted verification_url").into())
            })?;

        let expires_in = body["expires_in"].as_u64().unwrap_or(1800);
        let interval = body["interval"]
            .as_u64()
            .map(Duration::from_secs)
            .filter(|d| !d.is_zero())
            .unwrap_or_else(|| Duration::from_secs(5));

        Ok(DeviceAuthChallenge {
            verification_url: verification_url.to_string(),
            user_code: user_code.to_string(),
            device_code: device_code.to_string(),
            interval,
            deadline: Instant::now() + Duration::from_secs(expires_in),
        })
    }

    /// Poll once. Returns `Ok(None)` while the user has not approved yet
    /// (caller sleeps `retry_after()` and retries), `Ok(Some)` with the
    /// stored token on approval, or `Err` on denial/expiry/failure.
    pub async fn poll_device_code(
        &self,
        challenge: &mut DeviceAuthChallenge,
    ) -> Result<Option<TokenStorage>> {
        let mut form = vec![
            ("client_id", self.config.client_id.clone()),
            ("device_code", challenge.device_code.clone()),
            ("grant_type", DEVICE_GRANT.to_string()),
        ];
        if let Some(secret) = self.config.client_secret.clone().filter(|s| !s.is_empty()) {
            form.push(("client_secret", secret));
        }

        let response = self
            .http_client
            .post(self.token_endpoint.as_str())
            .form(&form)
            .send()
            .await
            .map_err(GrrError::Http)?;

        // Device polling reports status via 400 + JSON error codes, so the
        // body is parsed before looking at the HTTP status.
        let body_text = response.text().await.map_err(GrrError::Http)?;
        let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            GrrError::Auth(anyhow!("token endpoint returned unparseable body ({e})").into())
        })?;

        if let Some(code) = body["error"].as_str() {
            match code {
                "authorization_pending" => return Ok(None),
                "slow_down" => {
                    challenge.interval += Duration::from_secs(5);
                    return Ok(None);
                }
                _ => {
                    return Err(GrrError::Auth(
                        anyhow!("device poll failed: {}", json_error_detail(&body)).into(),
                    ));
                }
            }
        }

        let storage = token_storage_from_response(&body, super::SCOPES)?;
        *self.token_storage.write().await = Some(storage.clone());
        self.save_token(&storage).await?;
        Ok(Some(storage))
    }

    /// Fresh interactive login: drops any stored (possibly dead) token,
    /// runs the PKCE browser flow, saves and returns the new token.
    pub async fn login(&self) -> Result<TokenStorage> {
        // Best effort: a dead on-disk token must never block fresh consent.
        let _ = self.revoke().await;
        let storage = self.run_oauth_flow().await?;
        *self.token_storage.write().await = Some(storage.clone());
        self.save_token(&storage).await?;
        Ok(storage)
    }
}

/// Build [`TokenStorage`] from a successful token-endpoint JSON body.
/// Shared by the PKCE exchange and the device poll.
pub(crate) fn token_storage_from_response(
    token_data: &serde_json::Value,
    scopes: &[&str],
) -> Result<TokenStorage> {
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| GrrError::Auth(anyhow!("No access token in response").into()))?
        .to_string();

    let expires_in = token_data["expires_in"].as_u64().unwrap_or(3600);
    let refresh_token = token_data["refresh_token"].as_str().map(str::to_string);

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
        scope: scopes.join(" "),
    })
}

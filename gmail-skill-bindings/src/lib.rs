//! gmail-skill-bindings - NAPI bindings for opencode skill integration

#[cfg(feature = "napi-bindings")]
use napi_derive::napi;
#[cfg(feature = "napi-bindings")]
use gmail_core::{GmailClient, GmailConfig, AuthConfigBuilder};

#[cfg(feature = "napi-bindings")]
#[napi]
pub struct GmailSkillClient {
    inner: GmailClient,
}

#[cfg(feature = "napi-bindings")]
#[napi]
impl GmailSkillClient {
    #[napi(constructor)]
    pub async fn new() -> napi::Result<Self> {
        let config = GmailConfig::default();
        let auth = AuthConfigBuilder::new()
            .client_id(std::env::var("GMAIL_CLIENT_ID").unwrap_or_default())
            .client_secret(std::env::var("GMAIL_CLIENT_SECRET").unwrap_or_default())
            .build()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;

        let client = gmail_core::GmailClientBuilder::new(config)
            .auth(auth)
            .build()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;

        Ok(Self { inner: client })
    }

    #[napi]
    pub async fn search(&self, query: String, max_results: u32) -> napi::Result<Vec<gmail_core::MessageRef>> {
        self.inner.search(&query, max_results as usize)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn send(&self, to: String, subject: String, body: String) -> napi::Result<gmail_core::Message> {
        self.inner.send(&to, &subject, &body)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn list_labels(&self) -> napi::Result<Vec<gmail_core::Label>> {
        self.inner.list_labels()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn create_draft(&self, to: String, subject: String, body: String) -> napi::Result<gmail_core::Draft> {
        self.inner.create_draft(&to, &subject, &body)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}
use tracing::info;
use url::Url;

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, TransportInfo};
use crate::core::runtime::detect_runtime_features;

use crate::forms::models::*;

const DEFAULT_BASE_URL: &str = "https://forms.googleapis.com/v1/";
const PROBE_PATH: &str = "forms";

macro_rules! execute_json {
    ($client:ident, $($request:tt)*) => {{
        async {
            let response = $client.core.execute($($request)*).await?;
            let value = response.json().await?;
            Ok::<_, GrrError>(value)
        }
    }};
}

pub struct FormsClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
}

impl FormsClientBuilder {
    pub fn new() -> Self {
        Self {
            auth: None,
            base_url: None,
        }
    }

    pub fn auth(mut self, auth: GoogleAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<FormsClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid base URL: {e}")))?,
        };

        let core = if has_base_override {
            HttpCore::unprobed(auth, crate::core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await
        };
        FormsClient::new(core, base_url)
    }
}

impl Default for FormsClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct FormsClient {
    core: HttpCore,
    base_url: Url,
}

impl FormsClient {
    pub fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "FormsClient initialized: http3={}, io_uring={}",
            features.http3, features.io_uring
        );
        Ok(Self { core, base_url })
    }

    pub fn core(&self) -> &HttpCore {
        &self.core
    }

    pub fn transport_info(&self) -> &TransportInfo {
        self.core.transport_info()
    }

    pub fn token_backend(&self) -> &'static str {
        self.core.auth().token_backend()
    }

    pub async fn login(&self) -> Result<TokenStorage> {
        self.core.auth().login().await
    }

    pub async fn request_device_code(&self) -> Result<DeviceAuthChallenge> {
        self.core.auth().request_device_code().await
    }

    pub async fn poll_device_code(
        &self,
        challenge: &mut DeviceAuthChallenge,
    ) -> Result<Option<TokenStorage>> {
        self.core.auth().poll_device_code(challenge).await
    }

    pub async fn access_token(&self) -> Result<String> {
        self.core.auth().get_access_token().await
    }

    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {e}")))
    }

    fn form_url(&self, form_id: &str) -> Result<Url> {
        self.api_url(&normalize_form_id(form_id))
    }

    fn responses_url(&self, form_id: &str) -> Result<Url> {
        self.api_url(&format!("{}/responses", normalize_form_id(form_id)))
    }

    fn batch_update_url(&self, form_id: &str) -> Result<Url> {
        self.api_url(&format!("{}:batchUpdate", normalize_form_id(form_id)))
    }

    fn watches_url(&self, form_id: &str) -> Result<Url> {
        self.api_url(&format!("{}/watches", normalize_form_id(form_id)))
    }

    fn watch_url(&self, form_id: &str, watch_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "{}/watches/{}",
            normalize_form_id(form_id),
            urlencoding::encode(watch_id)
        ))
    }

    fn renew_watch_url(&self, form_id: &str, watch_id: &str) -> Result<Url> {
        self.api_url(&format!("{}:renew", self.watch_url(form_id, watch_id)?))
    }

    pub async fn get_form(&self, form_id: &str) -> Result<Form> {
        execute_json!(self, self.core.get(self.form_url(form_id)?)).await
    }

    pub async fn list_responses(
        &self,
        form_id: &str,
        max: Option<usize>,
    ) -> Result<Vec<FormResponse>> {
        let mut all_responses = Vec::new();
        let mut page_token: Option<String> = None;
        let batch_size = max.map_or(5000, |m| m.min(5000));

        loop {
            let mut request = self
                .core
                .get(self.responses_url(form_id)?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListFormResponsesResponse = execute_json!(self, request).await?;
            if let Some(page_responses) = page.responses {
                all_responses.extend(page_responses);
            }
            if let Some(maximum) = max
                && all_responses.len() >= maximum
            {
                all_responses.truncate(maximum);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_responses)
    }

    pub async fn create_form(
        &self,
        title: &str,
        description: Option<&str>,
        publish: bool,
    ) -> Result<Form> {
        if title.trim().is_empty() {
            return Err(GrrError::InvalidArgument(
                "form title must not be empty".into(),
            ));
        }
        let request = CreateFormRequest {
            info: CreateFormInfo {
                title: title.to_string(),
                description: description.map(str::to_string),
            },
        };
        let unpublished = (!publish).to_string();
        execute_json!(
            self,
            self.core
                .post(self.api_url("forms")?)
                .query(&[("unpublished", unpublished.as_str())])
                .json(&request),
        )
        .await
    }

    pub async fn update_form(
        &self,
        form_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> Result<Form> {
        if title.is_none() && description.is_none() {
            return Err(GrrError::InvalidArgument(
                "at least one form field must be supplied".into(),
            ));
        }

        let mut update_mask = Vec::new();
        if title.is_some() {
            update_mask.push("title");
        }
        if description.is_some() {
            update_mask.push("description");
        }
        let request = BatchUpdateFormRequest {
            include_form_in_response: Some(true),
            requests: vec![UpdateFormRequestItem {
                update_form_info: UpdateFormInfoRequest {
                    info: UpdateFormInfo {
                        title: title.map(str::to_string),
                        description: description.map(str::to_string),
                    },
                    update_mask: update_mask.join(","),
                },
            }],
        };

        let response: BatchUpdateFormResponse = execute_json!(
            self,
            self.core
                .post(self.batch_update_url(form_id)?)
                .json(&request),
        )
        .await?;
        Ok(response.form.unwrap_or_default())
    }

    pub async fn watch_form(&self, form_id: &str, topic_name: &str) -> Result<Watch> {
        if topic_name.trim().is_empty() {
            return Err(GrrError::InvalidArgument(
                "topic name must not be empty".into(),
            ));
        }
        let request = CreateWatchRequest {
            watch: Watch {
                target: Some(WatchTarget {
                    topic: Some(CloudPubsubTopic {
                        topic_name: Some(topic_name.to_string()),
                    }),
                }),
                event_type: Some("RESPONSES".into()),
                ..Watch::default()
            },
            watch_id: None,
        };
        execute_json!(
            self,
            self.core.post(self.watches_url(form_id)?).json(&request),
        )
        .await
    }

    pub async fn list_watches(&self, form_id: &str) -> Result<Vec<Watch>> {
        let response: ListWatchesResponse =
            execute_json!(self, self.core.get(self.watches_url(form_id)?)).await?;
        Ok(response.watches.unwrap_or_default())
    }

    pub async fn delete_watch(&self, form_id: &str, watch_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.watch_url(form_id, watch_id)?))
            .await?;
        Ok(())
    }

    pub async fn renew_watch(&self, form_id: &str, watch_id: &str) -> Result<Watch> {
        execute_json!(
            self,
            self.core
                .post(self.renew_watch_url(form_id, watch_id)?)
                .json(&RenewWatchRequest {}),
        )
        .await
    }
}

fn normalize_form_id(form_id: &str) -> String {
    let bare = form_id.strip_prefix("forms/").unwrap_or(form_id);
    format!("forms/{}", urlencoding::encode(bare))
}

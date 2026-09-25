use tracing::info;
use url::Url;

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, QueryParams, TransportInfo, join_url, parse_url};
use crate::core::runtime::detect_runtime_features;
use crate::core::{Page, paginate};

use crate::forms::models::*;

const DEFAULT_BASE_URL: &str = "https://forms.googleapis.com/v1/";
const PROBE_PATH: &str = "forms";

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
            None => parse_url(DEFAULT_BASE_URL, "base")?,
        };

        let core = if has_base_override {
            HttpCore::unprobed(auth, crate::core::http::build_http_client()?)
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await?
        };
        FormsClient::new(core, base_url).await
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
    pub async fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features().await;
        info!(
            "FormsClient initialized: http3=always, io_uring={}",
            features.io_uring
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
        join_url(&self.base_url, path, "API")
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
        self.core
            .execute_json(self.core.get(self.form_url(form_id)?))
            .await
    }

    pub async fn list_responses(
        &self,
        form_id: &str,
        max: Option<usize>,
    ) -> Result<Vec<FormResponse>> {
        let batch_size = max.map_or(5000, |value| value.min(5000));

        paginate(max, move |page_token| async move {
            let params = QueryParams::new()
                .add("pageSize", batch_size.to_string())
                .add_page_token(page_token.as_deref());
            let page: ListFormResponsesResponse = self
                .core
                .execute_json(params.apply(self.core.get(self.responses_url(form_id)?)))
                .await?;
            Ok(Page::new(
                page.responses.unwrap_or_default(),
                page.next_page_token,
            ))
        })
        .await
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
        self.core
            .execute_json(
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

        let response: BatchUpdateFormResponse = self
            .core
            .execute_json(
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
        self.core
            .execute_json(self.core.post(self.watches_url(form_id)?).json(&request))
            .await
    }

    pub async fn list_watches(&self, form_id: &str) -> Result<Vec<Watch>> {
        let response: ListWatchesResponse = self
            .core
            .execute_json(self.core.get(self.watches_url(form_id)?))
            .await?;
        Ok(response.watches.unwrap_or_default())
    }

    pub async fn delete_watch(&self, form_id: &str, watch_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.watch_url(form_id, watch_id)?))
            .await?;
        Ok(())
    }

    pub async fn renew_watch(&self, form_id: &str, watch_id: &str) -> Result<Watch> {
        self.core
            .execute_json(
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
